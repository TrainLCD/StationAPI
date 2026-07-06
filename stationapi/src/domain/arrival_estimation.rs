//! 駅リスト始点からの予定到着時間(始点からの累積分数)を推定する純粋ロジック。
//!
//! 公式時刻表・商用 API・実距離データが無い前提で、駅座標(直線距離)と
//! メタデータ(列車種別の停車/通過パターン、路線種別、駅間平均距離)だけから
//! 物理的に妥当な所要時間を推定する。
//!
//! モデル概要:
//! 1. 連続駅間の直線距離(haversine)を求める。
//! 2. 迂回係数 `α` で「みなし走行距離(軌道距離)」へ補正する。`α` は
//!    `Line.average_distance`(メートル単位・実距離±10%精度)が得られる路線では
//!    `average_distance / 直線平均駅間距離` で較正し、得られない路線では
//!    路線種別ベースの固定値にフォールバックする。較正の母数(直線平均)は
//!    クエリで切り出した部分区間ではなく **経路全体の駅列** から取る。
//!    `average_distance` は路線全体の平均駅間距離なので、駅間隔が路線平均と
//!    異なる部分区間(長距離路線の都心側など)の直線平均と比較すると `α` が
//!    実態(1.0〜1.1 程度)から大きく外れてしまうため。経路自体が路線の一部に
//!    偏り較正が破綻する路線は、実測値の較正テーブル
//!    (`LINE_DETOUR_OVERRIDES`)で `α` を上書きする。
//! 3. 停車駅間ごとに「加速→巡航→減速」の運動学モデルで走行時間を算出する。
//!    停車が多いほど巡航しきれず平均速度が落ちる(各停が速達より遅い)現象が
//!    加減速ペナルティとして自然に表現される。運動学モデルは理想走行
//!    (最大加速→最高速度で巡航→最大減速)を仮定するため、実ダイヤに含まれる
//!    途中の速度制限・惰行・回復余裕のぶん系統的に速すぎる。これを走行時間への
//!    運転余裕率 `run_margin` として補正する(実路線の時刻表との較正で約 1.15)。
//!    通過駅には分岐器・曲線の速度制限ぶんの小ペナルティ `pass_penalty` を加える。
//! 4. 中間停車駅に停車時間 `dwell` を加算して累積する。
//!
//! 速度は路線種別の基本速度に列車種別(`TrainTypeKind`)の倍率を掛けて決める。
//! 快速系(Branch/Rapid/CommuterRapid)は各停と同じ車両・線路を走り、速達性は
//! 通過(停車回数減)そのもので表現されるため倍率 1.0。急行・特急はより高速な
//! 走行(待避線での追い抜き前提のダイヤ)を、新快速級(HighSpeedRapid)は
//! 130km/h 運転をそれぞれ倍率で表す。実路線の時刻表(中央快速・井の頭急行・
//! 東横特急急行・京急快特・小田急快急・新快速など)との較正に基づく。
//!
//! 入力経路は運用上 `line_group_cd` を跨がない(=単一の列車・直通サービス)ため
//! 乗換時間は加算しない。直通で `line_cd` が変わる区間は `α`・最高速度の
//! 切り替えにのみ用いる。
//!
//! IO を持たない純粋関数群なので、すべて単体テスト可能。

use std::collections::HashMap;

use crate::domain::entity::gtfs::TransportType;
use crate::domain::entity::station::Station;
use crate::domain::segment_speed_table::{
    segment_override_applies_to_kind, segment_speed_override_kmh,
};
use crate::domain::speed_table::line_speed_override_kmh;
use crate::proto::{StopCondition, TrainTypeKind};

/// 1 駅分の推定結果。
#[derive(Clone, Debug, PartialEq)]
pub struct EstimatedStop {
    pub station_cd: i32,
    pub station_g_cd: i32,
    /// この駅が属する経路(line_group_cd)。複数候補経路をフラットな Vec で返す際に、
    /// 呼び出し側が経路境界を復元できるようにする。line_group_cd が無い駅は line_cd。
    pub line_group_cd: Option<i32>,
    /// 始点からの累積到着時間(分)。通過駅は停車駅間を速度プロファイル別に分割した
    /// 走行時間の積み上げで求めた通過時刻。
    pub cumulative_minutes: f64,
    /// 始点からの累積出発時刻(分)。中間停車駅では「到着 + 停車時間(dwell)」、
    /// 通過駅では通過時刻(= `cumulative_minutes`)、終点では到着時刻と同じ
    /// (以降の出発が無いため)。クライアントは基準駅 k0 と任意の駅 k について
    /// 「到着(k) − 出発(k0)」で基準駅発の正確な残り時間(発車基準 ETA)を計算できる。
    pub departure_cumulative_minutes: f64,
    /// その駅に停車するか(false = 通過)。
    pub stops_here: bool,
}

/// 推定で使う調整可能なパラメータ。すべて「実距離・実速度・ダイヤが無い」前提の
/// ヒューリスティックであり、後から較正・上書きできるよう一箇所に集約する。
#[derive(Clone, Copy, Debug)]
pub struct EstimationParams {
    /// 加速度 (m/s^2)。
    pub accel: f64,
    /// 減速度 (m/s^2)。
    pub decel: f64,
    /// 中間停車駅 1 駅あたりの停車時間(分)。ドア開閉・乗降に加え、
    /// 出発までの余裕時分も含む実効値。
    pub dwell_minutes: f64,
    /// 走行時間に掛ける運転余裕率。運動学モデルの理想走行と実ダイヤの差
    /// (途中の速度制限・惰行・回復余裕)を包括する補正係数。
    pub run_margin: f64,
    /// 通過駅 1 駅あたりの通過ペナルティ(秒)。分岐器・ホーム進入部の曲線など、
    /// 駅部を最高速度で通過しきれないぶんの実効値。
    pub pass_penalty_seconds: f64,
    /// 迂回係数 `α` のクランプ下限。
    pub detour_min: f64,
    /// 迂回係数 `α` のクランプ上限。
    pub detour_max: f64,
}

impl Default for EstimationParams {
    fn default() -> Self {
        Self {
            accel: 0.7,
            decel: 0.9,
            dwell_minutes: 0.6,
            run_margin: 1.15,
            pass_penalty_seconds: 3.0,
            detour_min: 1.0,
            detour_max: 1.6,
        }
    }
}

/// 新幹線を表す `line_type`。
const LINE_TYPE_SHINKANSEN: i32 = 1;
/// 地下鉄を表す `line_type`。
const LINE_TYPE_SUBWAY: i32 = 3;
/// 路面電車を表す `line_type`。
const LINE_TYPE_TRAM: i32 = 4;
/// AGT/モノレールを表す `line_type`。
const LINE_TYPE_AGT: i32 = 5;
/// ケーブルカーを表す `line_type`。
const LINE_TYPE_CABLE: i32 = 0;

/// バスの最高速度(km/h)。市街地の法定速度・信号停止を踏まえた実効上限。
/// GTFS 由来のバス路線は `line_type` に GTFS の `route_type`(バス=3)がそのまま
/// 入っており鉄道の路線種別(3=地下鉄)と衝突するため、速度・迂回係数の判定には
/// `line_type` ではなく `transport_type` を使う。
const BUS_MAX_SPEED_KMH: f64 = 50.0;
/// バスのフォールバック迂回係数。道路網の直線距離に対する迂回率。
const BUS_FALLBACK_DETOUR: f64 = 1.30;

/// 地球半径(メートル)。
const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

/// 2 点間の距離を haversine 公式で求める。返り値はメートル。
pub fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_METERS * c
}

/// 環状判定の最小駅数。実在の環状路線は最少 13 駅(富山都心線)。
const CIRCULAR_MIN_STATIONS: usize = 6;
/// シーム(末尾駅→先頭駅)距離の、平均駅間距離に対する許容倍率。
const CIRCULAR_SEAM_MEAN_RATIO: f64 = 2.0;
/// シーム距離の絶対上限(メートル)。実在環状路線の最大シームは山手線の約 1.4km。
const CIRCULAR_SEAM_MAX_METERS: f64 = 3_000.0;
/// 連続駅間距離の最大値の、中央値に対する許容倍率。格納順に不連続な飛び
/// (支線を跨ぐ成田線や、別線の駅が末尾に連なる近鉄奈良線など)を持つ路線を
/// 環状と誤検出しないためのガード。
const CIRCULAR_MAX_GAP_MEDIAN_RATIO: f64 = 3.0;

/// 順序付き駅リストが環状(ループ)経路かどうかを座標だけから推定する。
///
/// スキーマに環状フラグは無いため、「末尾駅から先頭駅へ戻るシーム距離が
/// 通常の駅間距離と同程度に短い」ことをヒューリスティックに判定する。
/// シードデータ全路線・全 line_group に対して山手線・大阪環状線(環状運転系統)・
/// 名城線・札幌市電・富山都心線・伊予鉄環状線のみが検出され、誤検出が
/// 無いことを確認済み。
pub fn is_circular_route(stops: &[&Station]) -> bool {
    let n = stops.len();
    if n < CIRCULAR_MIN_STATIONS {
        return false;
    }
    // 先頭駅が末尾にも重複して格納された「閉じた」ループはラップ不要
    // (ラップすると閉じ駅が二重に現れる)。
    if stops[0].station_cd == stops[n - 1].station_cd {
        return false;
    }
    let mut gaps: Vec<f64> = (1..n)
        .map(|i| {
            haversine_distance(
                stops[i - 1].lat,
                stops[i - 1].lon,
                stops[i].lat,
                stops[i].lon,
            )
        })
        .collect();
    let mean_gap = gaps.iter().sum::<f64>() / gaps.len() as f64;
    if mean_gap <= 0.0 {
        return false;
    }
    let seam = haversine_distance(
        stops[n - 1].lat,
        stops[n - 1].lon,
        stops[0].lat,
        stops[0].lon,
    );
    if seam > CIRCULAR_SEAM_MAX_METERS || seam > CIRCULAR_SEAM_MEAN_RATIO * mean_gap {
        return false;
    }
    gaps.sort_by(|a, b| a.total_cmp(b));
    let median_gap = gaps[gaps.len() / 2];
    let max_gap = gaps[gaps.len() - 1];
    median_gap > 0.0 && max_gap <= CIRCULAR_MAX_GAP_MEDIAN_RATIO * median_gap
}

/// 弧の直線距離合計(メートル)。無向の弧候補の比較に使う。
fn arc_length_meters(arc: &[&Station]) -> f64 {
    (1..arc.len())
        .map(|i| haversine_distance(arc[i - 1].lat, arc[i - 1].lon, arc[i].lat, arc[i].lon))
        .sum()
}

/// 環状経路上で `from_idx` → `to_idx` へ進む弧(arc)の駅列を返す。
///
/// `directed == true`(direction_id 指定あり)なら格納された向きに沿って進み、
/// 必要ならシーム(末尾→先頭)を跨いでラップする(決して反転しない)。
/// `directed == false` なら順方向弧と逆方向弧のうち直線距離合計が短い方を返す
/// (同値なら順方向)。駅数比較だと対蹠ペアでタイになるため距離で比べる。
pub fn select_circular_arc<'a>(
    stops: &[&'a Station],
    from_idx: usize,
    to_idx: usize,
    directed: bool,
) -> Vec<&'a Station> {
    // 順方向弧: 格納順に進み、末尾を越えたら先頭へラップする。
    let forward: Vec<&Station> = if from_idx <= to_idx {
        stops[from_idx..=to_idx].to_vec()
    } else {
        stops[from_idx..]
            .iter()
            .chain(stops[..=to_idx].iter())
            .copied()
            .collect()
    };
    if directed {
        return forward;
    }
    // 逆方向弧: 格納順を遡り、先頭を越えたら末尾へラップする。
    let backward: Vec<&Station> = if to_idx <= from_idx {
        stops[to_idx..=from_idx].iter().rev().copied().collect()
    } else {
        stops[..=from_idx]
            .iter()
            .rev()
            .chain(stops[to_idx..].iter().rev())
            .copied()
            .collect()
    };
    if arc_length_meters(&backward) < arc_length_meters(&forward) {
        backward
    } else {
        forward
    }
}

/// `average_distance` が得られない路線で使う、路線種別ベースの固定迂回係数。
fn fallback_detour_factor(line_type: Option<i32>, transport_type: TransportType) -> f64 {
    if transport_type == TransportType::Bus {
        return BUS_FALLBACK_DETOUR;
    }
    match line_type {
        Some(LINE_TYPE_SHINKANSEN) => 1.15,
        Some(LINE_TYPE_SUBWAY) => 1.20,
        Some(LINE_TYPE_TRAM) => 1.40,
        Some(LINE_TYPE_AGT) => 1.20,
        Some(LINE_TYPE_CABLE) => 1.10,
        _ => 1.30,
    }
}

/// 鉄道の迂回係数 `α` のクランプ上限。隣接駅間の軌道は直線距離の 1.1〜1.2 倍を
/// 超えることがほぼ無いため、`average_distance` の母数が汚れている路線
/// (別線区の駅列が同一 line_cd の末尾に連なる成田スカイアクセス線など)で
/// `α` が張り付いて走行距離を大幅に過大評価しないためのガード。
/// 道路網の迂回率が高いバスには適用しない(`params.detour_max` のまま)。
const RAIL_DETOUR_MAX: f64 = 1.35;

/// 路線ごとの実測迂回係数 `α` の較正テーブル。`(line_cd, α)`。
///
/// `α = average_distance / 直線平均駅間距離` の較正は「分子(路線全体の平均実
/// 駅間距離)と分母(経路の平均直線駅間距離)が同じ駅間隔分布を持つ」ことを
/// 仮定している。駅間隔が区間によって大きく異なる路線(都心側は 1〜2km 間隔・
/// 山間側は 3〜5km 間隔の西武池袋線など)では、経路(列車の運転区間)が路線の
/// 一部に限られると分母だけ狭い駅間隔で平均され、`α` が実態から大きく外れて
/// 走行距離・所要時間を系統的に過大評価する(例: 西武池袋線 池袋→飯能経路の
/// 較正値は 1.24 だが、営業キロ 43.7km / 直線 41.1km = 実測 1.06)。
///
/// ここには営業キロと駅座標の直線距離から実測した迂回係数を載せ、
/// `average_distance` ベースの較正より優先する。
/// 検証していない路線を推測で追加しないこと(較正フォールバックに任せる)。
const LINE_DETOUR_OVERRIDES: &[(i32, f64)] = &[
    // 西武池袋線: 池袋→飯能 営業キロ 43.7km / 直線距離合計 41.1km。
    // 池袋→所沢 準急 実28〜31分・急行 実24分への較正で検証。
    (22001, 1.06),
];

/// `line_cd` に対応する実測迂回係数を返す。エントリが無ければ `None`。
pub fn line_detour_override(line_cd: i32) -> Option<f64> {
    LINE_DETOUR_OVERRIDES
        .iter()
        .find(|(lc, _)| *lc == line_cd)
        .map(|(_, v)| *v)
}

/// 迂回係数 `α` を決める。
///
/// `avg_distance_km`(= `Line.average_distance` を km 換算した値、実距離±10%精度)が得られる場合は
/// `avg_distance_km / mean_straight_km` で較正し、`detour_min..=detour_max` にクランプする
/// (鉄道は上限をさらに `RAIL_DETOUR_MAX` で抑える)。
/// 得られない(`<= 0`)場合や直線平均が 0 の場合は路線種別ベースの固定値へフォールバックする。
pub fn detour_factor_for(
    avg_distance_km: f64,
    mean_straight_km: f64,
    line_type: Option<i32>,
    transport_type: TransportType,
    params: &EstimationParams,
) -> f64 {
    if avg_distance_km > 0.0 && mean_straight_km > 0.0 {
        let detour_max = if transport_type == TransportType::Bus {
            params.detour_max
        } else {
            params.detour_max.min(RAIL_DETOUR_MAX)
        }
        // detour_min > 上限 の設定でも clamp(min > max) でパニックしないよう正規化。
        .max(params.detour_min);
        (avg_distance_km / mean_straight_km).clamp(params.detour_min, detour_max)
    } else {
        fallback_detour_factor(line_type, transport_type)
    }
}

/// 路線種別ごとの基本最高速度(km/h)。
fn base_speed_kmh(line_type: Option<i32>) -> f64 {
    match line_type {
        Some(LINE_TYPE_SHINKANSEN) => 250.0,
        Some(LINE_TYPE_SUBWAY) => 75.0,
        Some(LINE_TYPE_TRAM) => 40.0,
        Some(LINE_TYPE_AGT) => 60.0,
        Some(LINE_TYPE_CABLE) => 12.0,
        _ => 80.0,
    }
}

/// 列車種別(`TrainTypeKind`)ごとの速度倍率。
///
/// 快速系(Rapid/CommuterRapid)や支線直通(Branch)は各停と同じ車両・線路を
/// 走るため各停と同速とし、速達性は通過(加減速・停車の削減)だけで表現する。
/// 急行・特急は速達ダイヤの実勢巡航速度、新快速級(HighSpeedRapid)は
/// 130km/h 運転を倍率で表す(在来線基本 80km/h × 1.5 = 120km/h)。
fn kind_speed_multiplier(kind: Option<i32>) -> f64 {
    match kind.and_then(|v| TrainTypeKind::try_from(v).ok()) {
        Some(TrainTypeKind::Express) => 1.15,
        Some(TrainTypeKind::LimitedExpress) => 1.2,
        Some(TrainTypeKind::HighSpeedRapid) => 1.5,
        _ => 1.0,
    }
}

/// 路線種別・列車種別から最高速度(km/h)を決める。
///
/// バスは `line_type`(GTFS の route_type が混入)や `kind`(BusRoute=7 は経路
/// マーカーであり優等種別ではない)で判定できないため、`transport_type` で
/// 先に分岐して固定の実効上限を返す。
/// 鉄道は路線 × 種別の較正テーブル(`speed_table`)を最優先し、エントリが
/// 無ければ「路線種別の基本速度 × 種別倍率」の一般則にフォールバックする。
fn max_speed_kmh(
    line_cd: i32,
    line_type: Option<i32>,
    kind: Option<i32>,
    transport_type: TransportType,
) -> f64 {
    if transport_type == TransportType::Bus {
        return BUS_MAX_SPEED_KMH;
    }
    if let Some(v) = line_speed_override_kmh(line_cd, kind) {
        return v;
    }
    let base = base_speed_kmh(line_type);
    if line_type == Some(LINE_TYPE_SHINKANSEN) {
        return base;
    }
    base * kind_speed_multiplier(kind)
}

/// 停車駅間の走行時間(分)を運動学モデルで求める。
///
/// 列車は 0→v_max 加速 → 巡航 → v_max→0 減速すると仮定する。
/// 区間距離が短く v_max に到達できない場合は三角形プロファイルで頂点速度を解く。
/// 理想走行と実ダイヤの差を埋めるため、結果に運転余裕率 `run_margin` を掛ける。
pub fn segment_run_minutes(distance_m: f64, v_max_kmh: f64, params: &EstimationParams) -> f64 {
    if distance_m <= 0.0 || v_max_kmh <= 0.0 {
        return 0.0;
    }
    let v = v_max_kmh / 3.6; // m/s
    let a = params.accel;
    let b = params.decel;
    let d_acc = v * v / (2.0 * a);
    let d_dec = v * v / (2.0 * b);

    let seconds = if distance_m >= d_acc + d_dec {
        // v_max に到達:加速 + 巡航 + 減速。
        v / a + v / b + (distance_m - d_acc - d_dec) / v
    } else {
        // 三角形プロファイル:頂点速度 v_peak で加速→減速。
        let v_peak = (2.0 * distance_m * a * b / (a + b)).sqrt();
        v_peak / a + v_peak / b
    };
    seconds * params.run_margin / 60.0
}

/// その駅に列車が停車するか判定する。端点(始点・終点)は常に停車扱い。
fn is_stop(station: &Station, is_endpoint: bool) -> bool {
    if is_endpoint {
        return true;
    }
    // 通過: pass == Some(1) もしくは stop_condition == Not。
    // Partial/Weekday/Holiday は MVP では停車扱い(将来は曜日入力で精緻化)。
    station.pass != Some(1) && station.stop_condition != StopCondition::Not
}

/// `line_cd` ごとに較正した迂回係数 `α` を返すマップを作る。
///
/// 同一 `line_cd` が連続する駅ペアの直線距離だけを平均(直通の切れ目で生じる
/// 路線跨ぎペアは除外)して `average_distance` と比較する。
fn detour_factors_by_line(
    stops: &[&Station],
    straight_km: &[f64],
    params: &EstimationParams,
) -> HashMap<i32, f64> {
    // line_cd -> (直線距離の合計, ペア数, average_distance, line_type, transport_type)
    let mut acc: HashMap<i32, (f64, u32, f64, Option<i32>, TransportType)> = HashMap::new();

    for i in 1..stops.len() {
        let cur = stops[i];
        let prev = stops[i - 1];
        // average_distance / line_type / transport_type は line_cd 単位で同じなので最初に見たものを採用。
        // average_distance はメートル単位で格納されているため km へ変換して直線平均と比較する。
        let entry = acc.entry(cur.line_cd).or_insert((
            0.0,
            0,
            cur.average_distance.unwrap_or(0.0) / 1000.0,
            cur.line_type,
            cur.transport_type,
        ));
        // 同一路線が連続するペアだけを直線平均の母数にする。
        if prev.line_cd == cur.line_cd {
            entry.0 += straight_km[i];
            entry.1 += 1;
        }
    }

    acc.into_iter()
        .map(
            |(line_cd, (sum, count, avg_distance, line_type, transport_type))| {
                // 実測の較正テーブルにある路線は average_distance ベースの
                // 較正より優先する(経路が路線の一部だと較正が破綻するため)。
                let detour = line_detour_override(line_cd).unwrap_or_else(|| {
                    let mean_straight = if count > 0 { sum / count as f64 } else { 0.0 };
                    detour_factor_for(
                        avg_distance,
                        mean_straight,
                        line_type,
                        transport_type,
                        params,
                    )
                });
                (line_cd, detour)
            },
        )
        .collect()
}

/// 停車駅間(`seg`)の各駅(通過駅・終点停車駅)へ到着時刻(分)を割り当てる。
///
/// `seg` は `(みなし走行距離 m, 最高速度 km/h, result index, 停車駅か)` のサブ区間列で、
/// 末尾要素が次の停車駅に対応する。列車は始点停車駅で 0 から加速し終点停車駅で 0 まで減速する。
/// 途中の通過駅では停車・加減速をしないが、直通で `line_cd` / 速度が変わるためサブ区間ごとに
/// 巡航時間を速度別に積む。単一サブ区間のときは加減速まで含む運動学モデル(短区間は三角形)で
/// 厳密に計算する。複数サブ区間の境界での速度遷移は簡易的に瞬時とみなす。
fn assign_segment_times(
    result: &mut [EstimatedStop],
    seg: &[(f64, f64, usize, bool)],
    departure_minutes: f64,
    params: &EstimationParams,
) {
    if seg.is_empty() {
        return;
    }
    if seg.len() == 1 {
        let (track_m, v_kmh, idx, _) = seg[0];
        let arrival = departure_minutes + segment_run_minutes(track_m, v_kmh, params);
        result[idx].cumulative_minutes = arrival;
        // 停車駅の出発時刻(dwell 加算)は呼び出し側が上書きする。
        result[idx].departure_cumulative_minutes = arrival;
        return;
    }

    // 複数サブ区間: 始点加速 + 各サブ区間の巡航 + 終点減速。
    let v_first = seg[0].1 / 3.6; // m/s
    let v_last = seg[seg.len() - 1].1 / 3.6;
    let accel_penalty_sec = v_first / (2.0 * params.accel);
    let decel_penalty_sec = v_last / (2.0 * params.decel);

    let mut cruise_sec = 0.0;
    for &(track_m, v_kmh, idx, is_stop) in seg.iter() {
        let v = v_kmh / 3.6;
        if v > 0.0 {
            cruise_sec += track_m / v;
        }
        if !is_stop {
            // 通過駅は分岐器・ホーム進入部の曲線で最高速度を維持できないぶんの
            // 小ペナルティを加える。
            cruise_sec += params.pass_penalty_seconds;
        }
        let seconds = if is_stop {
            // 終点停車駅: 加速 + 全巡航 + 減速。
            accel_penalty_sec + cruise_sec + decel_penalty_sec
        } else {
            // 通過駅: 加速 + ここまでの巡航(減速はまだ)。
            accel_penalty_sec + cruise_sec
        };
        let arrival = departure_minutes + seconds * params.run_margin / 60.0;
        result[idx].cumulative_minutes = arrival;
        // 通過駅は停車しないので出発 = 通過時刻。末尾の停車駅は呼び出し側が
        // dwell を加味した出発時刻で上書きする。
        result[idx].departure_cumulative_minutes = arrival;
    }
}

/// 順序付き駅リスト(単一 `line_group_cd` の経路)に対し、始点からの累積到着時間(分)を推定する。
///
/// `stops` は始点→終点の順に並んでいること。返り値は入力と同じ順・同じ要素数。
/// 迂回係数 `α` は `stops` 自身から較正する。`stops` が経路全体の一部を切り出した
/// 区間の場合は [`estimate_arrival_minutes_calibrated`] で経路全体を較正母数に渡すこと。
pub fn estimate_arrival_minutes(
    stops: &[&Station],
    params: &EstimationParams,
) -> Vec<EstimatedStop> {
    estimate_arrival_minutes_calibrated(stops, stops, params)
}

/// [`estimate_arrival_minutes`] の較正母数指定版。
///
/// `calibration_stops` には経路全体(部分区間へ切り出す前)の駅列を渡す。
/// `Line.average_distance` は路線全体の平均駅間距離なので、迂回係数
/// `α = average_distance / 直線平均駅間距離` の分母も路線全体相当の駅列から
/// 取らないと較正が破綻する(例: 長距離路線の都心側だけを切り出すと駅間隔が
/// 路線平均より狭く、`α` が上限に張り付いて走行距離を大幅に過大評価する)。
pub fn estimate_arrival_minutes_calibrated(
    stops: &[&Station],
    calibration_stops: &[&Station],
    params: &EstimationParams,
) -> Vec<EstimatedStop> {
    let n = stops.len();
    if n == 0 {
        return Vec::new();
    }

    // 各駅 i について「前駅との直線距離(km)」。straight_km[0] は未使用(0)。
    let mut straight_km = vec![0.0_f64; n];
    for i in 1..n {
        straight_km[i] = haversine_distance(
            stops[i - 1].lat,
            stops[i - 1].lon,
            stops[i].lat,
            stops[i].lon,
        ) / 1000.0;
    }

    // line_cd ごとの迂回係数。較正には経路全体の駅列を用いる。
    // 環状路線のシーム辺(末尾駅→先頭駅)は意図的に母数へ含めない。分子側の
    // `average_distance` 自体が格納順の隣接ペアのみ(シーム辺なし)から算出されて
    // おり(scripts/compute_average_distance.py)、分母だけシーム辺を足すと母数が
    // ズレるため。
    let calibration = if calibration_stops.is_empty() {
        stops
    } else {
        calibration_stops
    };
    let mut calib_straight_km = vec![0.0_f64; calibration.len()];
    for i in 1..calibration.len() {
        calib_straight_km[i] = haversine_distance(
            calibration[i - 1].lat,
            calibration[i - 1].lon,
            calibration[i].lat,
            calibration[i].lon,
        ) / 1000.0;
    }
    let detour_by_line = detour_factors_by_line(calibration, &calib_straight_km, params);
    let detour_of = |station: &Station| -> f64 {
        detour_by_line
            .get(&station.line_cd)
            .copied()
            .unwrap_or_else(|| {
                line_detour_override(station.line_cd).unwrap_or_else(|| {
                    fallback_detour_factor(station.line_type, station.transport_type)
                })
            })
    };

    // 各駅が停車するか。
    let stops_here: Vec<bool> = stops
        .iter()
        .enumerate()
        .map(|(i, s)| is_stop(s, i == 0 || i == n - 1))
        .collect();

    // 経路スライス内で路線ごとに通過駅があるか。通過駅が一つも無い路線では、
    // 優等種別(急行・特急など)でも実質各駅停車として走る(東急田園都市線の
    // 急行が半蔵門線内で各駅停車になる直通など)ため、その路線の区間では
    // 種別の速度倍率・種別別の速度較正を適用せず各停(Default)として扱う。
    // 判定は `is_stop` と同じ端点補正込みの `stops_here` を使う(端点は常に
    // 停車扱いのため、端点の pass フラグだけでは優等扱いにしない)。
    let mut line_has_pass: HashMap<i32, bool> = HashMap::new();
    for (i, s) in stops.iter().enumerate() {
        let entry = line_has_pass.entry(s.line_cd).or_insert(false);
        *entry = *entry || !stops_here[i];
    }

    let line_group_of =
        |station: &Station| -> Option<i32> { station.line_group_cd.or(Some(station.line_cd)) };

    let mut result: Vec<EstimatedStop> = Vec::with_capacity(n);

    // 始点。即時出発とみなすので到着・出発とも 0 分。
    result.push(EstimatedStop {
        station_cd: stops[0].station_cd,
        station_g_cd: stops[0].station_g_cd,
        line_group_cd: line_group_of(stops[0]),
        cumulative_minutes: 0.0,
        departure_cumulative_minutes: 0.0,
        stops_here: stops_here[0],
    });

    // 直前の停車駅を出発した時刻(分)。始点は即時出発なので 0。
    let mut last_departure = 0.0_f64;
    // 現在の停車間セグメントに溜めるサブ区間。
    // (みなし走行距離 m, 最高速度 km/h, result index, 停車駅か)
    let mut seg: Vec<(f64, f64, usize, bool)> = Vec::new();

    for i in 1..n {
        let track_m = straight_km[i] * detour_of(stops[i]) * 1000.0;
        // この路線に通過駅が無ければ各駅停車として振る舞う(種別倍率なし・
        // Default の速度較正を使用)。
        let effective_kind = if line_has_pass
            .get(&stops[i].line_cd)
            .copied()
            .unwrap_or(false)
        {
            stops[i].kind
        } else {
            None
        };
        let mut v_kmh = max_speed_kmh(
            stops[i].line_cd,
            stops[i].line_type,
            effective_kind,
            stops[i].transport_type,
        );
        // 隣接駅ペア単位の較正(GTFS 実ダイヤ由来)があれば路線単位の速度より
        // 優先する。急曲線・急勾配で路線平均より遅い区間(大江戸線 月島〜赤羽橋
        // など)の区間差を反映する。各停系種別の鉄道のみ。
        if stops[i].transport_type != TransportType::Bus
            && segment_override_applies_to_kind(effective_kind)
        {
            if let Some(v) = segment_speed_override_kmh(
                stops[i].line_cd,
                stops[i - 1].station_cd,
                stops[i].station_cd,
            ) {
                v_kmh = v;
            }
        }

        let idx = result.len();
        result.push(EstimatedStop {
            station_cd: stops[i].station_cd,
            station_g_cd: stops[i].station_g_cd,
            line_group_cd: line_group_of(stops[i]),
            cumulative_minutes: 0.0,
            departure_cumulative_minutes: 0.0,
            stops_here: stops_here[i],
        });
        seg.push((track_m, v_kmh, idx, stops_here[i]));

        if stops_here[i] {
            // 停車駅に到達 → 速度プロファイル別サブ区間で走行時間を積み上げ、
            // 区間内の各駅(通過駅・終点停車駅)へ到着時刻を割り当てる。
            assign_segment_times(&mut result, &seg, last_departure, params);

            let arrival = result[idx].cumulative_minutes;
            // 中間停車駅(終点以外)では停車時間を加えて次区間の出発時刻にする。
            last_departure = if i == n - 1 {
                arrival
            } else {
                arrival + params.dwell_minutes
            };
            result[idx].departure_cumulative_minutes = last_departure;

            seg.clear();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用の最小 Station を作る(座標・路線情報・停車情報のみ意味を持つ)。
    fn station(
        station_cd: i32,
        line_cd: i32,
        lat: f64,
        lon: f64,
        average_distance: Option<f64>,
    ) -> Station {
        Station {
            station_cd,
            station_g_cd: station_cd,
            station_name: String::new(),
            station_name_k: String::new(),
            station_name_r: None,
            station_name_zh: None,
            station_name_ko: None,
            station_numbers: vec![],
            station_number1: None,
            station_number2: None,
            station_number3: None,
            station_number4: None,
            three_letter_code: None,
            line_cd,
            line: None,
            lines: vec![],
            pref_cd: 13,
            post: String::new(),
            address: String::new(),
            lon,
            lat,
            open_ymd: String::new(),
            close_ymd: String::new(),
            e_status: 0,
            e_sort: station_cd,
            stop_condition: StopCondition::All,
            distance: None,
            has_train_types: false,
            train_type: None,
            company_cd: Some(1),
            line_name: None,
            line_name_k: None,
            line_name_h: None,
            line_name_r: None,
            line_name_zh: None,
            line_name_ko: None,
            line_color_c: None,
            line_type: Some(2),
            line_symbol1: None,
            line_symbol2: None,
            line_symbol3: None,
            line_symbol4: None,
            line_symbol1_color: None,
            line_symbol2_color: None,
            line_symbol3_color: None,
            line_symbol4_color: None,
            line_symbol1_shape: None,
            line_symbol2_shape: None,
            line_symbol3_shape: None,
            line_symbol4_shape: None,
            average_distance,
            type_id: None,
            sst_id: None,
            type_cd: None,
            line_group_cd: Some(1000),
            pass: None,
            type_name: None,
            type_name_k: None,
            type_name_r: None,
            type_name_zh: None,
            type_name_ko: None,
            color: None,
            direction: None,
            kind: None,
            transport_type: TransportType::Rail,
        }
    }

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-3, "expected {b}, got {a}");
    }

    #[test]
    fn haversine_same_point_is_zero() {
        approx(haversine_distance(35.0, 139.0, 35.0, 139.0), 0.0);
    }

    #[test]
    fn segment_run_reaches_vmax_for_long_distance() {
        let p = EstimationParams::default();
        // 10km を 80km/h で。巡航支配。
        let t = segment_run_minutes(10_000.0, 80.0, &p);
        // 巡航のみなら 10km / 80km/h × 余裕率 1.15 = 8.625 分。加減速分だけ少し増える。
        assert!(t > 8.6 && t < 10.0, "got {t}");
    }

    #[test]
    fn run_margin_scales_run_time_but_not_dwell() {
        let mut p = EstimationParams {
            run_margin: 1.0,
            ..Default::default()
        };
        let base = segment_run_minutes(5_000.0, 80.0, &p);
        p.run_margin = 1.15;
        let with_margin = segment_run_minutes(5_000.0, 80.0, &p);
        approx(with_margin, base * 1.15);
    }

    #[test]
    fn segment_run_uses_triangular_profile_for_short_distance() {
        let p = EstimationParams::default();
        // ごく短い区間では v_max に到達しない(三角形プロファイル)。
        let short = segment_run_minutes(100.0, 250.0, &p);
        // v_max=250 で台形を仮定した場合よりは時間がかかる(頂点速度が低い)。
        assert!(short > 0.0);
        // 同じ距離なら v_max を上げても結果は変わらない(到達しないため)。
        let short_slow = segment_run_minutes(100.0, 80.0, &p);
        approx(short, short_slow);
    }

    #[test]
    fn detour_factor_calibrates_then_clamps() {
        let p = EstimationParams::default();
        // average 1.3km / 直線 1.0km = 1.3。
        approx(
            detour_factor_for(1.3, 1.0, Some(2), TransportType::Rail, &p),
            1.3,
        );
        // 鉄道は RAIL_DETOUR_MAX = 1.35 でクランプ。
        approx(
            detour_factor_for(5.0, 1.0, Some(2), TransportType::Rail, &p),
            1.35,
        );
        // detour_min が鉄道上限 1.35 を超える設定でもパニックせず detour_min を採用。
        let wide_min = EstimationParams {
            detour_min: 1.4,
            ..Default::default()
        };
        approx(
            detour_factor_for(5.0, 1.0, Some(2), TransportType::Rail, &wide_min),
            1.4,
        );
        // バスは params.detour_max = 1.6 のままクランプ。
        approx(
            detour_factor_for(5.0, 1.0, Some(3), TransportType::Bus, &p),
            1.6,
        );
        // average_distance 無し → 在来線フォールバック 1.30。
        approx(
            detour_factor_for(0.0, 1.0, Some(2), TransportType::Rail, &p),
            1.30,
        );
        // 新幹線フォールバック 1.15。
        approx(
            detour_factor_for(0.0, 1.0, Some(1), TransportType::Rail, &p),
            1.15,
        );
        // バスは line_type=3(GTFS の route_type)でも地下鉄 1.20 ではなく道路 1.30。
        approx(
            detour_factor_for(0.0, 1.0, Some(3), TransportType::Bus, &p),
            1.30,
        );
    }

    /// 緯度方向に約 1.8km 間隔で並ぶ直線 3 駅。全駅停車。
    fn three_collinear_stations() -> Vec<Station> {
        // 0.016 度 ≈ 1.78km。
        vec![
            station(1, 100, 35.000, 139.0, None),
            station(2, 100, 35.016, 139.0, None),
            station(3, 100, 35.032, 139.0, None),
        ]
    }

    #[test]
    fn cumulative_times_increase_with_dwell() {
        let p = EstimationParams::default();
        let stations = three_collinear_stations();
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);

        assert_eq!(est.len(), 3);
        // 始点は 0 分。
        approx(est[0].cumulative_minutes, 0.0);
        assert!(est.iter().all(|e| e.stops_here));
        // 単調増加。
        assert!(est[1].cumulative_minutes > est[0].cumulative_minutes);
        assert!(est[2].cumulative_minutes > est[1].cumulative_minutes);
        // 中間駅で dwell が入るので、2区間目の到着は
        // 「1区間の所要 × 2 + dwell」付近になる。
        let leg = est[1].cumulative_minutes;
        approx(est[2].cumulative_minutes, leg * 2.0 + p.dwell_minutes);
    }

    #[test]
    fn departure_minutes_add_dwell_at_intermediate_stops_only() {
        let p = EstimationParams::default();
        let stations = three_collinear_stations();
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);

        // 始点: 即時出発なので到着・出発とも 0 分。
        approx(est[0].cumulative_minutes, 0.0);
        approx(est[0].departure_cumulative_minutes, 0.0);
        // 中間停車駅: 出発 = 到着 + dwell。
        approx(
            est[1].departure_cumulative_minutes,
            est[1].cumulative_minutes + p.dwell_minutes,
        );
        // 終点: 以降の出発が無いので出発 = 到着。
        approx(
            est[2].departure_cumulative_minutes,
            est[2].cumulative_minutes,
        );
        // クライアント側の発車基準 ETA: 到着(終点) − 出発(中間駅) = 1 区間の走行時間。
        let leg = est[1].cumulative_minutes;
        approx(
            est[2].cumulative_minutes - est[1].departure_cumulative_minutes,
            leg,
        );
    }

    #[test]
    fn departure_minutes_equal_pass_time_at_passed_stations() {
        let p = EstimationParams::default();
        let mut stations = three_collinear_stations();
        stations[1].pass = Some(1);
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);

        // 通過駅は停車しないので出発 = 通過時刻(dwell 加算なし)。
        assert!(!est[1].stops_here);
        approx(
            est[1].departure_cumulative_minutes,
            est[1].cumulative_minutes,
        );
    }

    #[test]
    fn passed_station_skips_dwell_and_is_interpolated() {
        let p = EstimationParams::default();
        let mut stations = three_collinear_stations();
        // 中間駅を通過にする。
        stations[1].pass = Some(1);
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);

        // 通過駅は stops_here=false。
        assert!(!est[1].stops_here);
        assert!(est[0].stops_here && est[2].stops_here);

        // 通過駅の通過時刻は区間内に収まり、終点より手前。
        assert!(est[1].cumulative_minutes > 0.0);
        assert!(est[1].cumulative_minutes < est[2].cumulative_minutes);

        // 全区間が 1 つの停車間セグメントになり、dwell が入らないぶん、
        // 全駅停車のときより終点到着が早い。
        let all_stop_stations = three_collinear_stations();
        let all_stop_refs: Vec<&Station> = all_stop_stations.iter().collect();
        let all_stop = estimate_arrival_minutes(&all_stop_refs, &p);
        assert!(est[2].cumulative_minutes < all_stop[2].cumulative_minutes);
    }

    #[test]
    fn line_group_cd_is_propagated() {
        let p = EstimationParams::default();
        let mut stations = three_collinear_stations();
        for s in stations.iter_mut() {
            s.line_group_cd = Some(777);
        }
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);
        assert!(est.iter().all(|e| e.line_group_cd == Some(777)));

        // line_group_cd が無い場合は line_cd にフォールバック。
        let mut no_group = three_collinear_stations();
        for s in no_group.iter_mut() {
            s.line_group_cd = None;
        }
        let refs2: Vec<&Station> = no_group.iter().collect();
        let est2 = estimate_arrival_minutes(&refs2, &p);
        assert!(est2.iter().all(|e| e.line_group_cd == Some(100)));
    }

    #[test]
    fn speed_profile_splits_within_pass_through_segment() {
        let p = EstimationParams::default();

        // 始点→通過→終点。通過駅から line_cd が変わる直通区間。
        // どちらも在来線(普通)。通過駅を line 200 側に置き、優等種別の速度が
        // 「通過駅のある路線」で有効になる条件を満たす。
        let mut slow = three_collinear_stations();
        slow[1].pass = Some(1);
        slow[1].line_cd = 200; // 直通で line_cd が変わる
        slow[2].line_cd = 200;
        let slow_refs: Vec<&Station> = slow.iter().collect();
        let slow_est = estimate_arrival_minutes(&slow_refs, &p);

        // 後半サブ区間(通過→終点)だけ高速種別(特急 → 80×1.2=96km/h)にする。
        let mut fast = three_collinear_stations();
        fast[1].pass = Some(1);
        fast[1].line_cd = 200;
        fast[2].line_cd = 200;
        fast[2].kind = Some(TrainTypeKind::LimitedExpress as i32); // 後半サブ区間の v_max を上げる
        let fast_refs: Vec<&Station> = fast.iter().collect();
        let fast_est = estimate_arrival_minutes(&fast_refs, &p);

        // 後半サブ区間が速くなったぶん、終点到着が早くなる(=区間ごとに速度が効いている)。
        assert!(
            fast_est[2].cumulative_minutes < slow_est[2].cumulative_minutes,
            "fast {} should be < slow {}",
            fast_est[2].cumulative_minutes,
            slow_est[2].cumulative_minutes
        );
        // 通過駅(前半サブ区間のみ)の通過時刻は速度を変えていないので不変。
        approx(
            fast_est[1].cumulative_minutes,
            slow_est[1].cumulative_minutes,
        );
    }

    #[test]
    fn express_kind_without_passes_behaves_like_local() {
        // 経路スライス内に通過駅が無い路線では、優等種別でも各駅停車として走る
        // (東急田園都市線の急行が半蔵門線内で各駅停車になる直通など)。
        // 種別倍率(特急 ×1.2)を適用してはいけない。
        let p = EstimationParams::default();
        let local = three_collinear_stations();
        let local_refs: Vec<&Station> = local.iter().collect();
        let local_est = estimate_arrival_minutes(&local_refs, &p);

        let mut express = three_collinear_stations();
        for s in express.iter_mut() {
            s.kind = Some(TrainTypeKind::LimitedExpress as i32);
        }
        let express_refs: Vec<&Station> = express.iter().collect();
        let express_est = estimate_arrival_minutes(&express_refs, &p);

        for (e, l) in express_est.iter().zip(local_est.iter()) {
            approx(e.cumulative_minutes, l.cumulative_minutes);
        }

        // 同じ種別でも通過駅があれば優等として扱われ、種別倍率のぶん速くなる。
        let mut passing = express.clone();
        passing[1].pass = Some(1);
        let passing_refs: Vec<&Station> = passing.iter().collect();
        let passing_est = estimate_arrival_minutes(&passing_refs, &p);
        let mut local_passing = three_collinear_stations();
        local_passing[1].pass = Some(1);
        let lp_refs: Vec<&Station> = local_passing.iter().collect();
        let lp_est = estimate_arrival_minutes(&lp_refs, &p);
        assert!(
            passing_est[2].cumulative_minutes < lp_est[2].cumulative_minutes,
            "passing express {} should be < passing local {}",
            passing_est[2].cumulative_minutes,
            lp_est[2].cumulative_minutes
        );
    }

    #[test]
    fn average_distance_meters_calibrates_detour_instead_of_clamping() {
        let p = EstimationParams::default();
        // 両毛線 伊勢崎→国定 相当: 直線約 5.65km、average_distance = 4866.8(メートル)。
        // メートルを km と誤解釈すると α が 4866.8/5.65 → 上限 1.6 に張り付き、
        // 実乗車時間(5〜6分)より大幅に長い約 6.9 分と推定されてしまう。
        let a = station(1, 11341, 36.326849, 139.193704, Some(4866.8));
        let b = station(2, 11341, 36.359018, 139.242463, Some(4866.8));
        let stations = [a, b];
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);

        // km 換算後は α = 4.8668 / 5.65 < 1 → detour_min の 1.0 でクランプされ、
        // みなし走行距離は直線距離そのものになる。
        let straight_m = haversine_distance(36.326849, 139.193704, 36.359018, 139.242463);
        let v_kmh = 80.0; // 在来線・普通(kind=None)
        let expected = segment_run_minutes(straight_m, v_kmh, &p);
        approx(est[1].cumulative_minutes, expected);
        // 実乗車時間は 5〜6 分。
        assert!(
            est[1].cumulative_minutes > 4.5 && est[1].cumulative_minutes < 6.5,
            "got {}",
            est[1].cumulative_minutes
        );
    }

    #[test]
    fn oedo_line_segment_matches_real_travel_time() {
        let p = EstimationParams::default();
        // 都営大江戸線 落合南長崎→光が丘(各駅停車のみの区間)。
        // 実座標・実データ値(average_distance = 1066.84m、地下鉄 line_type=3)。
        // 実乗車時間: 新江古田 2分 / 練馬 5分 / 豊島園 7分 / 練馬春日町 9分 / 光が丘 11分。
        let coords = [
            (35.723608, 139.683303),            // 落合南長崎
            (35.732538, 139.670653),            // 新江古田
            (35.737404, 139.65477),             // 練馬
            (35.742567043044, 139.64894845621), // 豊島園
            (35.751452, 139.640236),            // 練馬春日町
            (35.758526, 139.628603),            // 光が丘
        ];
        let stations: Vec<Station> = coords
            .iter()
            .enumerate()
            .map(|(i, &(lat, lon))| {
                let mut s = station(i as i32 + 1, 99301, lat, lon, Some(1066.84));
                s.line_type = Some(LINE_TYPE_SUBWAY);
                s
            })
            .collect();
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);

        let real = [2.0, 5.0, 7.0, 9.0, 11.0];
        for (e, r) in est[1..].iter().zip(real.iter()) {
            assert!(
                (e.cumulative_minutes - r).abs() < 1.0,
                "station_cd {}: est {} vs real {}",
                e.station_cd,
                e.cumulative_minutes,
                r
            );
        }
    }

    #[test]
    fn ginza_line_shibuya_to_shimbashi_matches_real_travel_time() {
        let p = EstimationParams::default();
        // 東京メトロ銀座線 渋谷→新橋。実座標・実データ値
        // (average_distance = 780.46154m、地下鉄 line_type=3)。
        // 実乗車時間は東京メトロ/乗換案内系の標準所要時間で約13分。
        let data = [
            (2800119, 35.659066, 139.701000), // 渋谷
            (2800118, 35.665247, 139.712314), // 表参道
            (2800117, 35.670527, 139.717857), // 外苑前
            (2800116, 35.672765, 139.724159), // 青山一丁目
            (2800115, 35.677021, 139.737047), // 赤坂見附
            (2800114, 35.673621, 139.741419), // 溜池山王
            (2800113, 35.670236, 139.749832), // 虎ノ門
            (2800112, 35.667434, 139.758432), // 新橋
        ];
        let stations: Vec<Station> = data
            .iter()
            .map(|&(cd, lat, lon)| {
                let mut s = station(cd, 28001, lat, lon, Some(780.46154));
                s.line_type = Some(LINE_TYPE_SUBWAY);
                s.line_group_cd = Some(28001);
                s
            })
            .collect();
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);

        let total = est.last().unwrap().cumulative_minutes;
        assert!((12.0..14.0).contains(&total), "got {total}");
    }

    /// GTFS インポート後のバス停を再現する(line_type=3 は GTFS route_type のバス、
    /// kind=7 は TrainTypeKind::BusRoute、average_distance は無し)。
    fn bus_station(station_cd: i32, line_cd: i32, lat: f64, lon: f64) -> Station {
        let mut s = station(station_cd, line_cd, lat, lon, None);
        s.transport_type = TransportType::Bus;
        s.line_type = Some(3);
        s.kind = Some(7);
        s
    }

    #[test]
    fn bus_ike65_dense_stops_match_real_travel_time() {
        let p = EstimationParams::default();
        // 都営バス池65 落合南長崎駅前→目白駅前相当: 停留所間隔約 300m × 7 区間。
        // 実乗車時間は約 10 分(表定速度 約13km/h)。0.0027 度 ≈ 300m。
        let stations: Vec<Station> = (0..8)
            .map(|i| bus_station(i + 1, 100_000_001, 35.72 + 0.0027 * i as f64, 139.69))
            .collect();
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);

        let total = est[7].cumulative_minutes;
        assert!(total > 9.0 && total < 11.0, "got {total}");
    }

    #[test]
    fn bus_long_segment_does_not_cruise_at_rail_speed() {
        let p = EstimationParams::default();
        // 停留所間隔 3km の直行区間。地下鉄扱い(75km/h × 1.2 = 90km/h)のままだと
        // 約 3.4 分と過小評価される。バス上限 50km/h では約 5.7 分。
        let stations = [
            bus_station(1, 100_000_001, 35.72, 139.69),
            bus_station(2, 100_000_001, 35.747, 139.69), // 0.027 度 ≈ 3km
        ];
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);

        let t = est[1].cumulative_minutes;
        assert!((4.5..7.5).contains(&t), "got {t}");
    }

    #[test]
    fn bus_kind_does_not_get_express_multiplier() {
        let p = EstimationParams::default();
        // kind=7(BusRoute) は優等種別ではないので ×1.2 が掛かってはいけない。
        let make = |kind: Option<i32>| -> f64 {
            let mut a = bus_station(1, 100_000_001, 35.72, 139.69);
            let mut b = bus_station(2, 100_000_001, 35.747, 139.69);
            a.kind = kind;
            b.kind = kind;
            let stations = [a, b];
            let refs: Vec<&Station> = stations.iter().collect();
            estimate_arrival_minutes(&refs, &p)[1].cumulative_minutes
        };
        approx(make(Some(7)), make(None));
    }

    /// 半径約 1.1km の円周上に等間隔で並ぶ n 駅の合成環状路線を作る(station_cd は 1..=n)。
    fn ring_stations(n: usize) -> Vec<Station> {
        (0..n)
            .map(|i| {
                let theta = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
                let lat = 35.0 + 0.01 * theta.cos();
                let lon = 139.0 + 0.01 * theta.sin() / 35.0_f64.to_radians().cos();
                station(i as i32 + 1, 500, lat, lon, None)
            })
            .collect()
    }

    /// 山手線 line_group_cd 363 の格納順(大崎→…→高輪ゲートウェイ→品川)と実座標。
    /// average_distance は実データ値(1093.74m)。
    fn yamanote_stations() -> Vec<Station> {
        let coords = [
            (1130201, 35.619772, 139.728439), // 大崎
            (1130202, 35.625974, 139.723822), // 五反田
            (1130203, 35.633923, 139.715775), // 目黒
            (1130204, 35.646685, 139.71007),  // 恵比寿
            (1130205, 35.658871, 139.701238), // 渋谷
            (1130206, 35.670646, 139.702592), // 原宿
            (1130207, 35.683061, 139.702042), // 代々木
            (1130208, 35.689729, 139.700464), // 新宿
            (1130209, 35.700875, 139.700261), // 新大久保
            (1130210, 35.712677, 139.703715), // 高田馬場
            (1130211, 35.720476, 139.706228), // 目白
            (1130212, 35.730256, 139.711086), // 池袋
            (1130213, 35.731412, 139.728584), // 大塚
            (1130214, 35.733445, 139.739303), // 巣鴨
            (1130215, 35.736825, 139.748053), // 駒込
            (1130216, 35.737781, 139.761229), // 田端
            (1130217, 35.731954, 139.766857), // 西日暮里
            (1130218, 35.727908, 139.771287), // 日暮里
            (1130219, 35.721484, 139.778015), // 鶯谷
            (1130220, 35.71379, 139.777043),  // 上野
            (1130221, 35.707282, 139.774727), // 御徒町
            (1130222, 35.698619, 139.773288), // 秋葉原
            (1130223, 35.691173, 139.770641), // 神田
            (1130224, 35.681391, 139.766103), // 東京
            (1130225, 35.675441, 139.763806), // 有楽町
            (1130226, 35.666195, 139.758587), // 新橋
            (1130227, 35.655391, 139.757135), // 浜松町
            (1130228, 35.645736, 139.747575), // 田町
            (1130230, 35.6355, 139.7407),     // 高輪ゲートウェイ
            (1130229, 35.62876, 139.738999),  // 品川
        ];
        coords
            .iter()
            .map(|&(cd, lat, lon)| {
                let mut s = station(cd, 11302, lat, lon, Some(1093.73663));
                s.line_group_cd = Some(363);
                s
            })
            .collect()
    }

    #[test]
    fn is_circular_route_detects_yamanote_ring() {
        let stations = yamanote_stations();
        let refs: Vec<&Station> = stations.iter().collect();
        assert!(is_circular_route(&refs));
    }

    #[test]
    fn is_circular_route_rejects_linear_line() {
        // 直線 8 駅(約 1.8km 間隔)。端点間 12km 超なので環状ではない。
        let stations: Vec<Station> = (0..8)
            .map(|i| station(i + 1, 100, 35.0 + 0.016 * i as f64, 139.0, None))
            .collect();
        let refs: Vec<&Station> = stations.iter().collect();
        assert!(!is_circular_route(&refs));
    }

    #[test]
    fn is_circular_route_rejects_small_ring() {
        // 幾何的には環状でも最小駅数未満なら対象外。
        let stations = ring_stations(5);
        let refs: Vec<&Station> = stations.iter().collect();
        assert!(!is_circular_route(&refs));
    }

    #[test]
    fn is_circular_route_rejects_closed_loop_with_duplicate_endpoint() {
        // 先頭駅が末尾にも重複格納された「閉じた」ループ(Port Liner 型)は
        // ラップすると閉じ駅が二重になるため環状扱いしない。
        let mut stations = ring_stations(10);
        let mut closing = stations[0].clone();
        closing.e_sort = 11;
        stations.push(closing);
        let refs: Vec<&Station> = stations.iter().collect();
        assert!(!is_circular_route(&refs));
    }

    #[test]
    fn is_circular_route_rejects_discontinuous_order() {
        // 近鉄奈良線型: 直線に進んだ後、格納順の末尾に始点近くの別線駅が連なる。
        // シームは短いが内部に巨大な飛びがあるため環状ではない。
        let mut stations: Vec<Station> = (0..8)
            .map(|i| station(i + 1, 100, 35.0, 139.0 + 0.011 * i as f64, None))
            .collect();
        // 始点から約 200m の位置に「飛び」で戻ってくる駅を追加。
        stations.push(station(9, 100, 35.0018, 139.0, None));
        let refs: Vec<&Station> = stations.iter().collect();
        assert!(!is_circular_route(&refs));
    }

    #[test]
    fn circular_arc_directed_wraps_seam() {
        let stations = ring_stations(10);
        let refs: Vec<&Station> = stations.iter().collect();

        // 格納順の向きのままシームを跨いでラップする(反転しない)。
        let arc = select_circular_arc(&refs, 8, 2, true);
        let cds: Vec<i32> = arc.iter().map(|s| s.station_cd).collect();
        assert_eq!(cds, vec![9, 10, 1, 2, 3]);

        // シームを跨がない場合は通常のスライス。
        let arc = select_circular_arc(&refs, 2, 8, true);
        let cds: Vec<i32> = arc.iter().map(|s| s.station_cd).collect();
        assert_eq!(cds, vec![3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn circular_arc_undirected_picks_shorter_arc() {
        let stations = ring_stations(10);
        let refs: Vec<&Station> = stations.iter().collect();

        // 順方向にラップする 3 駅の弧が最短(逆方向は 9 駅)。
        let arc = select_circular_arc(&refs, 9, 1, false);
        let cds: Vec<i32> = arc.iter().map(|s| s.station_cd).collect();
        assert_eq!(cds, vec![10, 1, 2]);

        // 対称ケース: 逆方向にラップする 3 駅の弧が最短。
        let arc = select_circular_arc(&refs, 1, 9, false);
        let cds: Vec<i32> = arc.iter().map(|s| s.station_cd).collect();
        assert_eq!(cds, vec![2, 1, 10]);
    }

    #[test]
    fn yamanote_tamachi_to_shibuya_crosses_seam() {
        // バグ再現回帰: 外回り 田町→渋谷 は品川・大崎経由の 8 駅(実乗車時間 約15分)。
        // 修正前は格納順の継ぎ目(品川⇔大崎)を跨げず、東京・池袋経由の
        // 24 駅の逆側の弧が返っていた。
        let p = EstimationParams::default();
        let stations = yamanote_stations();
        let refs: Vec<&Station> = stations.iter().collect();
        assert!(is_circular_route(&refs));

        let tamachi = 27; // 田町
        let shibuya = 4; // 渋谷
        let arc = select_circular_arc(&refs, tamachi, shibuya, true);
        let cds: Vec<i32> = arc.iter().map(|s| s.station_cd).collect();
        assert_eq!(
            cds,
            vec![1130228, 1130230, 1130229, 1130201, 1130202, 1130203, 1130204, 1130205],
        );

        let est = estimate_arrival_minutes(&arc, &p);
        assert!(est
            .windows(2)
            .all(|w| w[1].cumulative_minutes > w[0].cumulative_minutes));
        let total = est.last().unwrap().cumulative_minutes;
        assert!((12.0..18.0).contains(&total), "got {total}");
    }

    #[test]
    fn empty_input_returns_empty() {
        let p = EstimationParams::default();
        assert!(estimate_arrival_minutes(&[], &p).is_empty());
        assert!(estimate_arrival_minutes_calibrated(&[], &[], &p).is_empty());
    }

    #[test]
    fn rapid_and_branch_kinds_run_at_local_speed() {
        // 快速(Rapid)・支線(Branch)・通勤快速(CommuterRapid)は各停と同速。
        // 速達性は通過そのもので表現され、速度倍率では表現しない。
        let p = EstimationParams::default();
        let time_with_kind = |kind: Option<i32>| -> f64 {
            let mut stations = three_collinear_stations();
            for s in stations.iter_mut() {
                s.kind = kind;
            }
            let refs: Vec<&Station> = stations.iter().collect();
            estimate_arrival_minutes(&refs, &p)[2].cumulative_minutes
        };
        let local = time_with_kind(None);
        approx(time_with_kind(Some(TrainTypeKind::Rapid as i32)), local);
        approx(time_with_kind(Some(TrainTypeKind::Branch as i32)), local);
        approx(
            time_with_kind(Some(TrainTypeKind::CommuterRapid as i32)),
            local,
        );
        // 急行 < 各停、特急 < 急行、新快速級 < 特急 の順に速い。
        // 優等種別の倍率は「経路内に中間通過駅がある路線」でのみ有効なため、
        // 末尾側に中間通過駅(4駅目)+終点(5駅目)を足した経路の 3 駅目到着で比較する。
        let time_with_kind_passing = |kind: Option<i32>| -> f64 {
            let mut stations = three_collinear_stations();
            let mut passed = station(4, 100, 35.048, 139.0, None);
            passed.pass = Some(1);
            stations.push(passed);
            stations.push(station(5, 100, 35.064, 139.0, None));
            for s in stations.iter_mut() {
                s.kind = kind;
            }
            let refs: Vec<&Station> = stations.iter().collect();
            estimate_arrival_minutes(&refs, &p)[2].cumulative_minutes
        };
        let local = time_with_kind_passing(None);
        let express = time_with_kind_passing(Some(TrainTypeKind::Express as i32));
        let limited = time_with_kind_passing(Some(TrainTypeKind::LimitedExpress as i32));
        let high_speed = time_with_kind_passing(Some(TrainTypeKind::HighSpeedRapid as i32));
        assert!(
            express < local,
            "express {express} should be < local {local}"
        );
        assert!(
            limited < express,
            "limited {limited} should be < express {express}"
        );
        assert!(
            high_speed < limited,
            "high_speed {high_speed} should be < limited {limited}"
        );
    }

    #[test]
    fn speed_table_override_beats_general_rule() {
        // 京急本線(27001)の快特(Express)は較正テーブルの 120km/h が適用され、
        // 一般則(80×1.15=92km/h)の路線より同一区間を速く走る。
        let p = EstimationParams::default();
        let time_on_line = |line_cd: i32| -> f64 {
            // 8km 区間(巡航支配)で比較する。0.072 度 ≈ 8km。
            // 優等種別の速度は中間通過駅のある路線でのみ有効なため、後方に
            // 中間通過駅(3駅目)+終点(4駅目)を足し、2 駅目(単一サブ区間の
            // 停車駅)への到着時間を見る。
            let mut stations = vec![
                station(1, line_cd, 35.000, 139.0, None),
                station(2, line_cd, 35.072, 139.0, None),
                station(3, line_cd, 35.080, 139.0, None),
                station(4, line_cd, 35.088, 139.0, None),
            ];
            stations[2].pass = Some(1);
            for s in stations.iter_mut() {
                s.kind = Some(TrainTypeKind::Express as i32);
            }
            let refs: Vec<&Station> = stations.iter().collect();
            estimate_arrival_minutes(&refs, &p)[1].cumulative_minutes
        };
        let keikyu = time_on_line(27001);
        let generic = time_on_line(100);
        assert!(
            keikyu < generic,
            "keikyu {keikyu} should be < generic {generic}"
        );
        // テーブル値 120km/h での運動学モデルと厳密に一致する。
        let straight_m = haversine_distance(35.000, 139.0, 35.072, 139.0);
        // average_distance 無し → 在来線フォールバック α=1.30。
        approx(keikyu, segment_run_minutes(straight_m * 1.30, 120.0, &p));
    }

    #[test]
    fn pass_penalty_adds_time_per_passed_station() {
        let mut p = EstimationParams {
            pass_penalty_seconds: 0.0,
            ..Default::default()
        };
        let mut stations = three_collinear_stations();
        stations[1].pass = Some(1);
        let refs: Vec<&Station> = stations.iter().collect();
        let base = estimate_arrival_minutes(&refs, &p)[2].cumulative_minutes;

        p.pass_penalty_seconds = 3.0;
        let with_penalty = estimate_arrival_minutes(&refs, &p)[2].cumulative_minutes;
        // 通過駅 1 駅ぶんのペナルティ(運転余裕率込み)だけ遅くなる。
        approx(with_penalty, base + 3.0 * p.run_margin / 60.0);
    }

    #[test]
    fn slice_calibration_uses_full_route_regression() {
        // スライス較正バグの回帰: 「都心側は駅間 1km・郊外側は駅間 4km」の路線で
        // average_distance(路線全体の平均実駅間距離 ≈ 2km)を持つとき、都心側だけを
        // 切り出して較正すると α = 2.0/1.0 → 鉄道上限 1.35 に張り付き、走行距離を
        // 35% 過大評価してしまう。経路全体を較正母数に渡せば α ≈ 1.0 になる。
        let p = EstimationParams::default();
        let avg_dist_m = Some(2000.0);
        let mut stations: Vec<Station> = Vec::new();
        // 都心側: 1km(緯度 0.009 度)間隔 × 6 駅。
        for i in 0..6 {
            stations.push(station(
                i + 1,
                100,
                35.0 + 0.009 * i as f64,
                139.0,
                avg_dist_m,
            ));
        }
        // 郊外側: 4km(緯度 0.036 度)間隔 × 3 駅。
        let base_lat = 35.0 + 0.009 * 5.0;
        for i in 0..3 {
            stations.push(station(
                7 + i,
                100,
                base_lat + 0.036 * (i + 1) as f64,
                139.0,
                avg_dist_m,
            ));
        }
        let full: Vec<&Station> = stations.iter().collect();
        let slice: Vec<&Station> = full[..6].to_vec();

        let self_calibrated = estimate_arrival_minutes(&slice, &p);
        let full_calibrated = estimate_arrival_minutes_calibrated(&slice, &full, &p);

        // スライス単体較正(旧挙動)は α が上限に張り付き大幅に遅い推定になる。
        assert!(
            full_calibrated[5].cumulative_minutes < self_calibrated[5].cumulative_minutes * 0.9,
            "full {} should be much faster than slice-calibrated {}",
            full_calibrated[5].cumulative_minutes,
            self_calibrated[5].cumulative_minutes
        );
        // 経路全体較正のスライス推定は、経路全体推定の該当区間と一致する。
        let full_est = estimate_arrival_minutes(&full, &p);
        approx(
            full_calibrated[5].cumulative_minutes,
            full_est[5].cumulative_minutes,
        );
    }

    /// 西武池袋線 準急(kind=Express)の実ダイヤ較正回帰テスト。
    ///
    /// 迂回係数の実測較正(`LINE_DETOUR_OVERRIDES`)の回帰: 西武池袋線は都心側
    /// 1〜2km・山間側 3〜5km と駅間隔の区間差が大きく、準急の経路(池袋→飯能)を
    /// 較正母数にすると `α = 2041m / 1645m = 1.24` と実測(営業キロ比 1.06)を
    /// 大きく超え、全区間の所要時間を 10〜17% 過大評価していた
    /// (実車では表示 ETA より約 1 分早く到着する)。
    ///
    /// 期待値は平日日中の準急 池袋→所沢の実時刻表から、石神井公園での
    /// 待避停車ぶんを除いた純走行ベースで取っている。
    #[test]
    fn seibu_ikebukuro_semi_express_matches_real_travel_time() {
        let p = EstimationParams::default();
        // 池袋→飯能(準急の経路全体)。pass=1 は通過駅。実座標・実データ値
        // (average_distance = 2041.41825m)。
        // (station_cd, lat, lon, pass, 実所要分(池袋発・停車駅のみ))
        let data: &[(i32, f64, f64, i32, Option<f64>)] = &[
            (2200101, 35.72913, 139.711461, 0, Some(0.0)),   // 池袋
            (2200102, 35.726572, 139.694363, 1, None),       // 椎名町
            (2200103, 35.73003, 139.683294, 1, None),        // 東長崎
            (2200104, 35.737557, 139.672814, 1, None),       // 江古田
            (2200105, 35.738797, 139.662602, 1, None),       // 桜台
            (2200106, 35.737893, 139.654368, 0, Some(6.5)),  // 練馬
            (2200107, 35.736767, 139.637456, 1, None),       // 中村橋
            (2200108, 35.735867, 139.62969, 1, None),        // 富士見台
            (2200109, 35.740622, 139.616749, 1, None),       // 練馬高野台
            (2200110, 35.743563, 139.606981, 0, Some(10.5)), // 石神井公園
            (2200111, 35.749406, 139.586732, 0, Some(13.0)), // 大泉学園
            (2200112, 35.748222, 139.567753, 0, Some(15.5)), // 保谷
            (2200113, 35.751485, 139.545852, 0, Some(18.5)), // ひばりヶ丘
            (2200114, 35.760445, 139.533739, 0, Some(20.5)), // 東久留米
            (2200115, 35.772221, 139.519917, 0, Some(23.0)), // 清瀬
            (2200116, 35.778614, 139.496539, 0, Some(25.5)), // 秋津
            (2200131, 35.786627, 139.473324, 0, Some(28.5)), // 所沢
            (2200117, 35.789303, 139.455959, 0, None),       // 西所沢
            (2200118, 35.800535, 139.438016, 0, None),       // 小手指
            (2200119, 35.810445, 139.416975, 0, None),       // 狭山ヶ丘
            (2200120, 35.820963, 139.412736, 0, None),       // 武蔵藤沢
            (2200121, 35.845112, 139.39842, 0, None),        // 稲荷山公園
            (2200122, 35.842904, 139.390294, 0, None),       // 入間市
            (2200123, 35.83769, 139.360115, 0, None),        // 仏子
            (2200124, 35.84058, 139.345316, 0, None),        // 元加治
            (2200125, 35.851189, 139.318824, 0, None),       // 飯能
        ];
        let stations: Vec<Station> = data
            .iter()
            .map(|&(cd, lat, lon, pass, _)| {
                let mut s = station(cd, 22001, lat, lon, Some(2041.41825));
                s.kind = Some(TrainTypeKind::Express as i32);
                if pass == 1 {
                    s.pass = Some(1);
                }
                s
            })
            .collect();
        // 本番同様、所沢までの乗車区間を切り出し、較正母数には経路全体を渡す。
        let full: Vec<&Station> = stations.iter().collect();
        let slice: Vec<&Station> = full[..17].to_vec();
        let est = estimate_arrival_minutes_calibrated(&slice, &full, &p);

        for (e, d) in est.iter().zip(data.iter()) {
            let Some(real) = d.4 else { continue };
            assert!(
                (e.cumulative_minutes - real).abs() < 1.5,
                "station_cd {}: est {} vs real {}",
                d.0,
                e.cumulative_minutes,
                real
            );
        }
    }

    #[test]
    fn line_detour_override_lookup() {
        // 西武池袋線は実測値 1.06。
        assert_eq!(line_detour_override(22001), Some(1.06));
        // 未較正の路線はエントリ無し。
        assert_eq!(line_detour_override(11302), None);
    }

    #[test]
    fn line_detour_override_beats_average_distance_calibration() {
        let p = EstimationParams::default();
        // 西武池袋線 石神井公園→大泉学園 相当の 2 駅。average_distance(2041m)を
        // 直線平均(約 2.0km)で割る較正では α ≈ 1.0 だが、実測テーブルの 1.06 が
        // 優先される。
        let a = station(1, 22001, 35.743563, 139.606981, Some(2041.41825));
        let b = station(2, 22001, 35.749406, 139.586732, Some(2041.41825));
        let stations = [a, b];
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);

        let straight_m = haversine_distance(35.743563, 139.606981, 35.749406, 139.586732);
        // 在来線・kind=None → 80km/h。みなし走行距離は直線 × 1.06。
        let expected = segment_run_minutes(straight_m * 1.06, 80.0, &p);
        approx(est[1].cumulative_minutes, expected);
    }
}
