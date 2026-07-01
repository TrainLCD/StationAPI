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
//!    路線種別ベースの固定値にフォールバックする。
//! 3. 停車駅間ごとに「加速→巡航→減速」の運動学モデルで走行時間を算出する。
//!    停車が多いほど巡航しきれず平均速度が落ちる(各停が速達より遅い)現象が
//!    加減速ペナルティとして自然に表現される。
//! 4. 中間停車駅に停車時間 `dwell` を加算して累積する。
//!
//! 入力経路は運用上 `line_group_cd` を跨がない(=単一の列車・直通サービス)ため
//! 乗換時間は加算しない。直通で `line_cd` が変わる区間は `α`・最高速度の
//! 切り替えにのみ用いる。
//!
//! IO を持たない純粋関数群なので、すべて単体テスト可能。

use std::collections::HashMap;

use crate::domain::entity::station::Station;
use crate::proto::StopCondition;

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
    /// 中間停車駅 1 駅あたりの停車時間(分)。
    pub dwell_minutes: f64,
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
            dwell_minutes: 0.4,
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

/// `average_distance` が得られない路線で使う、路線種別ベースの固定迂回係数。
fn fallback_detour_factor(line_type: Option<i32>) -> f64 {
    match line_type {
        Some(LINE_TYPE_SHINKANSEN) => 1.15,
        Some(LINE_TYPE_SUBWAY) => 1.20,
        Some(LINE_TYPE_TRAM) => 1.40,
        Some(LINE_TYPE_AGT) => 1.20,
        Some(LINE_TYPE_CABLE) => 1.10,
        _ => 1.30,
    }
}

/// 迂回係数 `α` を決める。
///
/// `avg_distance_km`(= `Line.average_distance` を km 換算した値、実距離±10%精度)が得られる場合は
/// `avg_distance_km / mean_straight_km` で較正し、`detour_min..=detour_max` にクランプする。
/// 得られない(`<= 0`)場合や直線平均が 0 の場合は路線種別ベースの固定値へフォールバックする。
pub fn detour_factor_for(
    avg_distance_km: f64,
    mean_straight_km: f64,
    line_type: Option<i32>,
    params: &EstimationParams,
) -> f64 {
    if avg_distance_km > 0.0 && mean_straight_km > 0.0 {
        (avg_distance_km / mean_straight_km).clamp(params.detour_min, params.detour_max)
    } else {
        fallback_detour_factor(line_type)
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
        _ => 85.0,
    }
}

/// 路線種別・列車種別から最高速度(km/h)を決める。
fn max_speed_kmh(line_type: Option<i32>, kind: Option<i32>) -> f64 {
    let base = base_speed_kmh(line_type);
    if line_type == Some(LINE_TYPE_SHINKANSEN) {
        return base;
    }
    match kind {
        Some(k) if k != 0 => base * 1.2,
        _ => base,
    }
}

/// 停車駅間の走行時間(分)を運動学モデルで求める。
///
/// 列車は 0→v_max 加速 → 巡航 → v_max→0 減速すると仮定する。
/// 区間距離が短く v_max に到達できない場合は三角形プロファイルで頂点速度を解く。
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
    seconds / 60.0
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
    // line_cd -> (直線距離の合計, ペア数, average_distance, line_type)
    let mut acc: HashMap<i32, (f64, u32, f64, Option<i32>)> = HashMap::new();

    for i in 1..stops.len() {
        let cur = stops[i];
        let prev = stops[i - 1];
        // average_distance / line_type は line_cd 単位で同じなので最初に見たものを採用。
        // average_distance はメートル単位で格納されているため km へ変換して直線平均と比較する。
        let entry = acc.entry(cur.line_cd).or_insert((
            0.0,
            0,
            cur.average_distance.unwrap_or(0.0) / 1000.0,
            cur.line_type,
        ));
        // 同一路線が連続するペアだけを直線平均の母数にする。
        if prev.line_cd == cur.line_cd {
            entry.0 += straight_km[i];
            entry.1 += 1;
        }
    }

    acc.into_iter()
        .map(|(line_cd, (sum, count, avg_distance, line_type))| {
            let mean_straight = if count > 0 { sum / count as f64 } else { 0.0 };
            (
                line_cd,
                detour_factor_for(avg_distance, mean_straight, line_type, params),
            )
        })
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
        result[idx].cumulative_minutes =
            departure_minutes + segment_run_minutes(track_m, v_kmh, params);
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
        let seconds = if is_stop {
            // 終点停車駅: 加速 + 全巡航 + 減速。
            accel_penalty_sec + cruise_sec + decel_penalty_sec
        } else {
            // 通過駅: 加速 + ここまでの巡航(減速はまだ)。
            accel_penalty_sec + cruise_sec
        };
        result[idx].cumulative_minutes = departure_minutes + seconds / 60.0;
    }
}

/// 順序付き駅リスト(単一 `line_group_cd` の経路)に対し、始点からの累積到着時間(分)を推定する。
///
/// `stops` は始点→終点の順に並んでいること。返り値は入力と同じ順・同じ要素数。
pub fn estimate_arrival_minutes(
    stops: &[&Station],
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

    // line_cd ごとの迂回係数。
    let detour_by_line = detour_factors_by_line(stops, &straight_km, params);
    let detour_of = |station: &Station| -> f64 {
        detour_by_line
            .get(&station.line_cd)
            .copied()
            .unwrap_or_else(|| fallback_detour_factor(station.line_type))
    };

    // 各駅が停車するか。
    let stops_here: Vec<bool> = stops
        .iter()
        .enumerate()
        .map(|(i, s)| is_stop(s, i == 0 || i == n - 1))
        .collect();

    let line_group_of =
        |station: &Station| -> Option<i32> { station.line_group_cd.or(Some(station.line_cd)) };

    let mut result: Vec<EstimatedStop> = Vec::with_capacity(n);

    // 始点。
    result.push(EstimatedStop {
        station_cd: stops[0].station_cd,
        station_g_cd: stops[0].station_g_cd,
        line_group_cd: line_group_of(stops[0]),
        cumulative_minutes: 0.0,
        stops_here: stops_here[0],
    });

    // 直前の停車駅を出発した時刻(分)。始点は即時出発なので 0。
    let mut last_departure = 0.0_f64;
    // 現在の停車間セグメントに溜めるサブ区間。
    // (みなし走行距離 m, 最高速度 km/h, result index, 停車駅か)
    let mut seg: Vec<(f64, f64, usize, bool)> = Vec::new();

    for i in 1..n {
        let track_m = straight_km[i] * detour_of(stops[i]) * 1000.0;
        let v_kmh = max_speed_kmh(stops[i].line_type, stops[i].kind);

        let idx = result.len();
        result.push(EstimatedStop {
            station_cd: stops[i].station_cd,
            station_g_cd: stops[i].station_g_cd,
            line_group_cd: line_group_of(stops[i]),
            cumulative_minutes: 0.0,
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

            seg.clear();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entity::gtfs::TransportType;

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
        // 巡航のみなら 10km / 80km/h = 7.5 分。加減速分だけ少し増える。
        assert!(t > 7.5 && t < 9.0, "got {t}");
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
        approx(detour_factor_for(1.3, 1.0, Some(2), &p), 1.3);
        // 上限 1.6 でクランプ。
        approx(detour_factor_for(5.0, 1.0, Some(2), &p), 1.6);
        // average_distance 無し → 在来線フォールバック 1.30。
        approx(detour_factor_for(0.0, 1.0, Some(2), &p), 1.30);
        // 新幹線フォールバック 1.15。
        approx(detour_factor_for(0.0, 1.0, Some(1), &p), 1.15);
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
        // 中間駅で dwell(0.4分)が入るので、2区間目の到着は
        // 「1区間の所要 × 2 + dwell」付近になる。
        let leg = est[1].cumulative_minutes;
        approx(est[2].cumulative_minutes, leg * 2.0 + p.dwell_minutes);
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

        // 始点→通過→終点。通過駅で line_cd が変わる直通区間。
        // どちらも在来線(普通)。
        let mut slow = three_collinear_stations();
        slow[1].pass = Some(1);
        slow[2].line_cd = 200; // 直通で line_cd が変わる
        let slow_refs: Vec<&Station> = slow.iter().collect();
        let slow_est = estimate_arrival_minutes(&slow_refs, &p);

        // 後半サブ区間(通過→終点)だけ高速種別(kind != 0 → 100km/h)にする。
        let mut fast = three_collinear_stations();
        fast[1].pass = Some(1);
        fast[2].line_cd = 200;
        fast[2].kind = Some(1); // 速達種別 → 後半サブ区間の v_max を上げる
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
    fn average_distance_meters_calibrates_detour_instead_of_clamping() {
        let p = EstimationParams::default();
        // 両毛線 伊勢崎→国定 相当: 直線約 5.65km、average_distance = 4866.8(メートル)。
        // メートルを km と誤解釈すると α が 4866.8/5.65 → 上限 1.6 に張り付き、
        // 実乗車時間(5〜6分)より大幅に長い約 6.9 分と推定されてしまう。
        let a = station(1, 11341, 36.326849, 139.193704, Some(4866.8));
        let b = station(2, 11341, 36.359018, 139.242463, Some(4866.8));
        let stations = vec![a, b];
        let refs: Vec<&Station> = stations.iter().collect();
        let est = estimate_arrival_minutes(&refs, &p);

        // km 換算後は α = 4.8668 / 5.65 < 1 → detour_min の 1.0 でクランプされ、
        // みなし走行距離は直線距離そのものになる。
        let straight_m = haversine_distance(36.326849, 139.193704, 36.359018, 139.242463);
        let v_kmh = 85.0; // 在来線・普通(kind=None)
        let expected = segment_run_minutes(straight_m, v_kmh, &p);
        approx(est[1].cumulative_minutes, expected);
        assert!(
            est[1].cumulative_minutes < 6.0,
            "got {}",
            est[1].cumulative_minutes
        );
    }

    #[test]
    fn empty_input_returns_empty() {
        let p = EstimationParams::default();
        assert!(estimate_arrival_minutes(&[], &p).is_empty());
    }
}
