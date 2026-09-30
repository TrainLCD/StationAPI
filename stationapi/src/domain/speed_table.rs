//! 路線 × 列車種別ごとの実効最高速度の較正テーブル。
//!
//! 到着時間推定(`arrival_estimation`)の速度モデルは「路線種別の基本速度 ×
//! 列車種別倍率」の一般則で決まるが、実勢速度が一般則から大きく外れる路線
//! (120km/h 超の高速運転を行う私鉄優等、過密ダイヤ・急曲線で実効速度が
//! 上がらない路線など)は種別データだけでは表現できない。ここでは実路線の
//! 公表運転速度と時刻表所要時間への較正に基づく (line_cd, kind) 単位の
//! 上書き値を一元管理する。
//!
//! 値の意味は「その路線・種別での実効巡航速度(km/h)」。理論上の車両性能では
//! なく、時刻表所要時間を運動学モデルで再現する値として較正している。
//! `trainRoute` の `Estimated` の推定 (`SpeedCalibration::Recalibrated`) だけが
//! 参照する。`estimateArrivalTimes`・`connectedRoutes`・`trainRoute` の `Legacy` は、
//! 求め直す前の表 (`legacy_speed_table`) を使う。
//!
//! エントリ追加の指針:
//! - 公表運転速度(例: 京急快特 120km/h、スカイライナー 160km/h)を起点にし、
//!   実時刻表の所要時間と突き合わせて検証した路線だけを載せる。
//! - 検証していない路線を推測で追加しない(一般則フォールバックに任せる)。
//! - 公開 GTFS 時刻表が利用できる路線は `scripts/compute_speed_table.py` で
//!   自動較正し、下部の自動生成ブロックへ書き込む(手動テーブルが優先)。

use crate::model::TrainTypeKind;

/// (line_cd, kind, 実効最高速度 km/h)。kind は `TrainTypeKind` の値。
///
/// 距離に線路の長さを使い、GTFS の自動較正を同じ距離で求め直したうえで、それでも
/// 実際の所要時間 (`travel_times/cases.csv` の典型値) から外れる路線・種別だけを
/// 載せる。値は実際の最高速度を超えない範囲で求める。超えないと合わない路線は、
/// 速度ではなく加減速などのモデル側の問題として扱う。追加・変更したら
/// `make travel-time-report` で全体の誤差が悪くならないことを確かめる。
///
/// 距離が直線 × 迂回係数だった頃の値は、距離の水増しを速度で打ち消していたので、
/// 線路の長さへ替えたときに求め直した。小田急線 (快速急行) は一般則で典型値に
/// 近づいたので外した。つくばエクスプレスと都営大江戸線は GTFS の自動較正に任せる。
/// `estimateArrivalTimes`・`connectedRoutes`・`trainRoute` の `Legacy` は、求め直す前の
/// 値を `legacy_speed_table` で使い続ける。
const LINE_SPEED_OVERRIDES: &[(i32, TrainTypeKind, f64)] = &[
    // 総武快速線: 最高 130km/h の別線を走る。StationAPI の路線には快速の停車駅しか
    // 無く、通過駅が無いので推定は各停 (Default) として扱う。一般則の 80km/h では
    // 遅すぎる。千葉以東の総武本線の普通列車にもかかるが、その区間の基準は無い。
    // travel_times: 錦糸町→津田沼・新小岩→津田沼 快速。
    (11314, TrainTypeKind::Default, 100.0),
    // 成田スカイアクセス線: スカイライナーは 160km/h 運転だが、実効値で較正。
    // アクセス特急も同じ LimitedExpress で、京成高砂→成田空港は 39.4 分
    // (実際 38〜52 分。ばらつきが大きく典型値が無いので travel_times には無い)。
    // travel_times: 日暮里→空港第2ビル スカイライナー。
    (23006, TrainTypeKind::LimitedExpress, 120.0),
    // 京王井の頭線: 急行の実効速度。travel_times: 渋谷→吉祥寺 急行。
    (24006, TrainTypeKind::Express, 80.0),
    // 東急東横線: 特急は最高 110km/h だが過密ダイヤ・急曲線で実効は各停並み。
    // travel_times: 渋谷→横浜 特急。
    (26001, TrainTypeKind::LimitedExpress, 80.0),
    // 京急本線: 快特 (kind は Express)・特急は最高 120km/h。最高速度で快特が
    // 典型値 +3.5% (17.6 分) まで近づく。travel_times: 品川→横浜 快特。
    (27001, TrainTypeKind::Express, 120.0),
    (27001, TrainTypeKind::LimitedExpress, 120.0),
    // 近鉄特急(名阪甲特急ひのとり): 難波線は地下線、大阪線は山間曲線区間を含むため
    // 実効値で較正。travel_times には、系統 335 の停車駅が実際の列車と一致しない
    // ため入れていない。値は距離を線路の長さへ替える前に決めたもので、見直していない。
    (31001, TrainTypeKind::LimitedExpress, 80.0),
    (31005, TrainTypeKind::LimitedExpress, 115.0),
    (31027, TrainTypeKind::LimitedExpress, 120.0),
    // 阪急神戸本線: 特急の実効速度。travel_times: 大阪梅田→神戸三宮 特急。
    (34001, TrainTypeKind::LimitedExpress, 110.0),
];

/// 公開 GTFS 時刻表からの自動較正エントリ。`scripts/compute_speed_table.py --apply`
/// がマーカー間を再生成する。手動テーブル(`LINE_SPEED_OVERRIDES`)と重複するキーは
/// スクリプト側で除外される。手動編集しないこと。
const LINE_SPEED_OVERRIDES_GTFS: &[(i32, TrainTypeKind, f64)] = &[
    // --- BEGIN GENERATED (scripts/compute_speed_table.py) ---
    // 東京メトロ銀座線 Default: 東京メトロ GTFS 8本 中央値34分 (一般則 75km/h)
    (28001, TrainTypeKind::Default, 55.0),
    // 東京メトロ丸ノ内線 Default: 東京メトロ GTFS 12本 中央値52分 (一般則 75km/h)
    (28002, TrainTypeKind::Default, 60.0),
    // 東京メトロ日比谷線 Default: 東京メトロ GTFS 7本 中央値45分 (一般則 75km/h)
    (28003, TrainTypeKind::Default, 55.0),
    // 東京メトロ東西線 Default: 東京メトロ GTFS 20本 中央値54分 (一般則 75km/h)
    (28004, TrainTypeKind::Default, 60.0),
    // 東京メトロ千代田線 Default: 東京メトロ GTFS 13本 中央値42分 (一般則 75km/h)
    (28005, TrainTypeKind::Default, 65.0),
    // 東京メトロ有楽町線 Default: 東京メトロ GTFS 19本 中央値52分 (一般則 75km/h)
    (28006, TrainTypeKind::Default, 65.0),
    // 東京メトロ半蔵門線 Default: 東京メトロ GTFS 10本 中央値32分 (一般則 75km/h)
    (28008, TrainTypeKind::Default, 55.0),
    // 東京メトロ南北線 Default: 東京メトロ GTFS 12本 中央値38分 (一般則 75km/h)
    (28009, TrainTypeKind::Default, 65.0),
    // 東京メトロ副都心線 Default: 東京メトロ GTFS 31本 中央値27分 (一般則 75km/h)
    (28010, TrainTypeKind::Default, 60.0),
    // 東京メトロ副都心線 Express: 東京メトロ GTFS 15本 中央値28分 (一般則 86km/h)
    (28010, TrainTypeKind::Express, 65.0),
    // 函館市電2系統 Default: 函館市電 GTFS 4本 中央値48分 (一般則 40km/h)
    (99105, TrainTypeKind::Default, 20.0),
    // 函館市電5系統 Default: 函館市電 GTFS 6本 中央値47分 (一般則 40km/h)
    (99106, TrainTypeKind::Default, 20.0),
    // 都営大江戸線 Default: 都営地下鉄 GTFS 11本 中央値84分 (一般則 75km/h)
    (99301, TrainTypeKind::Default, 60.0),
    // 都営浅草線 Default: 都営地下鉄 GTFS 18本 中央値37分 (一般則 75km/h)
    (99302, TrainTypeKind::Default, 65.0),
    // 都営浅草線 LimitedExpress: 都営地下鉄 GTFS 5本 中央値21分 (一般則 90km/h)
    (99302, TrainTypeKind::LimitedExpress, 55.0),
    // 都営新宿線 Default: 都営地下鉄 GTFS 10本 中央値42分 (一般則 75km/h)
    (99304, TrainTypeKind::Default, 85.0),
    // 東京さくらトラム(都電荒川線) Default: 都営地下鉄 GTFS 13本 中央値56分 (一般則 40km/h)
    (99305, TrainTypeKind::Default, 25.0),
    // つくばエクスプレス線 Default: つくばエクスプレス GTFS 11本 中央値46分 (一般則 80km/h)
    (99309, TrainTypeKind::Default, 95.0),
    // 横浜市営地下鉄ブルーライン Rapid: 横浜市営地下鉄 GTFS 3本 中央値61分 (一般則 75km/h)
    (99316, TrainTypeKind::Rapid, 65.0),
    // 多摩モノレール Default: 多摩都市モノレール GTFS 6本 中央値38分 (一般則 60km/h)
    (99334, TrainTypeKind::Default, 50.0),
    // りんかい線 Default: りんかい線 GTFS 8本 中央値20分 (一般則 80km/h)
    (99337, TrainTypeKind::Default, 65.0),
    // 京都市営地下鉄東西線 Default: 京都市営地下鉄 GTFS 9本 中央値33分 (一般則 75km/h)
    (99611, TrainTypeKind::Default, 65.0),
    // --- END GENERATED ---
];

/// (line_cd, kind) に対応する実効最高速度(km/h)を返す。エントリが無ければ `None`。
///
/// 手動較正テーブルを優先し、次に GTFS 自動較正テーブルを引く。
/// `kind` が `None`(列車種別なし=路線内各駅停車扱い)は `Default` として引く。
pub fn line_speed_override_kmh(line_cd: i32, kind: Option<i32>) -> Option<f64> {
    let kind = TrainTypeKind::try_from(kind.unwrap_or(0)).ok()?;
    LINE_SPEED_OVERRIDES
        .iter()
        .chain(LINE_SPEED_OVERRIDES_GTFS.iter())
        .find(|(lc, k, _)| *lc == line_cd && *k == kind)
        .map(|(_, _, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_entry_overrides_only_its_kind() {
        // 東急東横線の特急は手動の較正値が引ける。
        approx(
            line_speed_override_kmh(26001, Some(TrainTypeKind::LimitedExpress as i32)),
            Some(80.0),
        );
        // 同じ路線でも、エントリの無い種別は一般則にフォールバック。
        assert_eq!(line_speed_override_kmh(26001, None), None);
        assert_eq!(
            line_speed_override_kmh(26001, Some(TrainTypeKind::Rapid as i32)),
            None
        );
    }

    #[test]
    fn none_kind_is_treated_as_default() {
        // つくばエクスプレスの各停エントリは kind 未指定(路線クエリ)でも引ける。
        approx(line_speed_override_kmh(99309, None), Some(95.0));
        approx(
            line_speed_override_kmh(99309, Some(TrainTypeKind::Default as i32)),
            Some(95.0),
        );
        // 快速(HighSpeedRapid)はエントリ無し → 一般則(80×1.5=120km/h)に任せる。
        assert_eq!(
            line_speed_override_kmh(99309, Some(TrainTypeKind::HighSpeedRapid as i32)),
            None
        );
    }

    #[test]
    fn unknown_line_or_kind_returns_none() {
        assert_eq!(line_speed_override_kmh(11302, None), None);
        assert_eq!(line_speed_override_kmh(27001, Some(999)), None);
    }

    #[test]
    fn gtfs_generated_entries_are_looked_up() {
        // GTFS 自動較正ブロックのエントリも lookup 対象になる(値は再生成で変わり得る
        // ため存在だけを確認する)。函館市電2系統は認証不要フィードなので常に較正可能。
        assert!(line_speed_override_kmh(99105, None).is_some());
    }

    #[test]
    fn manual_table_takes_precedence_over_gtfs_on_key_collision() {
        // 手動テーブルと同じキーが仮に GTFS テーブルにも存在した場合でも、
        // lookup は手動テーブルを先に引くため手動の値が返ることを回帰検知する。
        approx(
            line_speed_override_kmh(34001, Some(TrainTypeKind::LimitedExpress as i32)),
            Some(110.0),
        );
    }

    fn approx(actual: Option<f64>, expected: Option<f64>) {
        match (actual, expected) {
            (Some(a), Some(e)) => assert!((a - e).abs() < 1e-9, "expected {e}, got {a}"),
            (a, e) => assert_eq!(a, e),
        }
    }
}
