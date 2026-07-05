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
//! 到着時間推定と GetTrainRoute の速度プロファイル(`resolve_speed_profile`)の
//! 両方から参照される。
//!
//! エントリ追加の指針:
//! - 公表運転速度(例: 京急快特 120km/h、スカイライナー 160km/h)を起点にし、
//!   実時刻表の所要時間と突き合わせて検証した路線だけを載せる。
//! - 検証していない路線を推測で追加しない(一般則フォールバックに任せる)。
//! - 公開 GTFS 時刻表が利用できる路線は `scripts/compute_speed_table.py` で
//!   自動較正し、下部の自動生成ブロックへ書き込む(手動テーブルが優先)。

use crate::proto::TrainTypeKind;

/// (line_cd, kind, 実効最高速度 km/h)。kind は `TrainTypeKind` の値。
/// 較正ベンチマーク: 各行のコメントの実所要時間(日中標準)に対し誤差 ±10% 以内。
const LINE_SPEED_OVERRIDES: &[(i32, TrainTypeKind, f64)] = &[
    // 京急本線: 快特・特急 120km/h 運転。品川→横浜 快特 実17分。
    (27001, TrainTypeKind::Express, 120.0),
    (27001, TrainTypeKind::LimitedExpress, 120.0),
    // 小田急線: 最高 110km/h。新宿→町田 快速急行 実34分。
    (25001, TrainTypeKind::Express, 110.0),
    // 東急東横線: 特急は最高 110km/h だが過密ダイヤ・急曲線で実効は各停並み。
    // 渋谷→横浜 特急 実27分。
    (26001, TrainTypeKind::LimitedExpress, 80.0),
    // 京王井の頭線: 急行の実効速度。渋谷→吉祥寺 急行 実17分。
    (24006, TrainTypeKind::Express, 80.0),
    // 京成本線(都心側): スカイライナーの上野→高砂間の実効速度。
    (23001, TrainTypeKind::LimitedExpress, 100.0),
    // 成田スカイアクセス線: スカイライナー 160km/h 運転、アクセス特急 120km/h 運転。
    // 日暮里→空港第2ビル スカイライナー 実36分。
    (23006, TrainTypeKind::LimitedExpress, 160.0),
    (23006, TrainTypeKind::Express, 120.0),
    // つくばエクスプレス: 各停も 125km/h 運転(駅間が短いため実効値で較正)。
    // 秋葉原→つくば 普通 実66分。快速(kind=HighSpeedRapid)は一般則 120km/h で十分。
    (99309, TrainTypeKind::Default, 95.0),
    // 阪急神戸本線: 特急 115km/h 運転。大阪梅田→神戸三宮 実27分。
    (34001, TrainTypeKind::LimitedExpress, 110.0),
    // 近鉄特急(名阪甲特急ひのとり): 大阪難波→近鉄名古屋 実125分。
    // 難波線は地下線、大阪線は山間曲線区間を含むため実効値で較正。
    (31001, TrainTypeKind::LimitedExpress, 80.0),
    (31005, TrainTypeKind::LimitedExpress, 115.0),
    (31027, TrainTypeKind::LimitedExpress, 120.0),
];

/// 公開 GTFS 時刻表からの自動較正エントリ。`scripts/compute_speed_table.py --apply`
/// がマーカー間を再生成する。手動テーブル(`LINE_SPEED_OVERRIDES`)と重複するキーは
/// スクリプト側で除外される。手動編集しないこと。
const LINE_SPEED_OVERRIDES_GTFS: &[(i32, TrainTypeKind, f64)] = &[
    // --- BEGIN GENERATED (scripts/compute_speed_table.py) ---
    // 函館市電2系統 Default: 函館市電 GTFS 4本 中央値48分 (一般則 40km/h)
    (99105, TrainTypeKind::Default, 20.0),
    // 函館市電5系統 Default: 函館市電 GTFS 6本 中央値47分 (一般則 40km/h)
    (99106, TrainTypeKind::Default, 25.0),
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
    fn keikyu_express_kinds_are_overridden() {
        approx(
            line_speed_override_kmh(27001, Some(TrainTypeKind::Express as i32)),
            Some(120.0),
        );
        approx(
            line_speed_override_kmh(27001, Some(TrainTypeKind::LimitedExpress as i32)),
            Some(120.0),
        );
        // 京急でも各停・快速は一般則にフォールバック。
        assert_eq!(line_speed_override_kmh(27001, None), None);
        assert_eq!(
            line_speed_override_kmh(27001, Some(TrainTypeKind::Rapid as i32)),
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

    fn approx(actual: Option<f64>, expected: Option<f64>) {
        match (actual, expected) {
            (Some(a), Some(e)) => assert!((a - e).abs() < 1e-9, "expected {e}, got {a}"),
            (a, e) => assert_eq!(a, e),
        }
    }
}
