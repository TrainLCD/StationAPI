use core::panic;
use std::collections::HashSet;
use std::path::Path;

use csv::{ReaderBuilder, StringRecord};

/// `3!stations.csv` の列インデックス。
const STATIONS_COL_STATION_CD: usize = 0;
const STATIONS_COL_LINE_CD: usize = 13;
const STATIONS_COL_E_STATUS: usize = 21;
const STATIONS_COL_E_SORT: usize = 22;

/// 並び順(`ORDER BY e_sort, station_cd`)が崩れると API の経路スライスから
/// 駅が欠落する箇所の期待並び。`(line_cd, 連続して並ぶべき station_cd 列)`。
///
/// 都営大江戸線は都庁前が環状部始端(9930100)と放射部接続側(9930101)の
/// 2レコードで表現される「6の字」路線で、放射部側の都庁前が
/// 「…→新宿→都庁前→西新宿五丁目→…」の位置から外れると、新宿⇔放射部を
/// 跨ぐ区間の到着推定から都庁前が丸ごと欠落し、ETAが一駅分短くなる
/// (#1589 で e_sort 重複時の station_cd タイブレークが反転して発生、
/// #1595 で修正)。この退行を CI で検知する。
const EXPECTED_CONSECUTIVE_STATION_ORDERS: &[(u32, &[u32])] = &[
    // 都営大江戸線: 環状部始端の都庁前の直後は新宿西口。
    (99301, &[9930100, 9930102]),
    // 都営大江戸線: 新宿 → 都庁前(放射部側) → 西新宿五丁目。
    (99301, &[9930128, 9930101, 9930129]),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut invalid_station_ids: Vec<String> = Vec::new();
    let mut invalid_type_ids: Vec<String> = Vec::new();

    let data_path: &Path = Path::new("data");
    let mut rdr = ReaderBuilder::new().from_path(data_path.join("3!stations.csv"))?;
    let station_records: Vec<StringRecord> = rdr.records().collect::<Result<Vec<_>, _>>()?;
    let station_ids: HashSet<u32> = station_records
        .iter()
        .map(|row| {
            row.get(STATIONS_COL_STATION_CD)
                .unwrap()
                .parse::<u32>()
                .unwrap()
        })
        .collect();

    let mut rdr = ReaderBuilder::new().from_path(data_path.join("4!types.csv"))?;
    let records: Vec<StringRecord> = rdr.records().collect::<Result<Vec<_>, _>>()?;
    let type_ids: HashSet<u32> = records
        .iter()
        .map(|row| row.get(1).unwrap().parse::<u32>().unwrap())
        .collect();

    let mut rdr = ReaderBuilder::new().from_path(data_path.join("5!station_station_types.csv"))?;
    let records: Vec<StringRecord> = rdr.records().collect::<Result<Vec<_>, _>>()?;

    for record in &records {
        let line = || record.iter().collect::<Vec<&str>>().join(",");

        let station_cd: u32 = match record.get(1).and_then(|v| v.parse().ok()) {
            Some(id) => id,
            None => {
                println!("[INVALID] Failed to parse station_cd from row: {}", line());
                invalid_station_ids.push(line());
                continue;
            }
        };
        let type_cd: u32 = match record.get(2).and_then(|v| v.parse().ok()) {
            Some(id) => id,
            None => {
                println!("[INVALID] Failed to parse type_cd from row: {}", line());
                invalid_type_ids.push(line());
                continue;
            }
        };

        if !station_ids.contains(&station_cd) {
            println!("[INVALID] Unrecognized Station ID {:?} Found!", station_cd);
            invalid_station_ids.push(line());
        }
        if !type_ids.contains(&type_cd) {
            println!("[INVALID] Unrecognized Type ID {:?} Found!", type_cd);
            invalid_type_ids.push(line());
        }
    }

    let invalid_station_orders = validate_station_orders(&station_records);
    for message in &invalid_station_orders {
        println!("[INVALID] {message}");
    }

    let has_err = !invalid_station_ids.is_empty()
        || !invalid_type_ids.is_empty()
        || !invalid_station_orders.is_empty();

    if has_err {
        let report = build_markdown_report(
            &invalid_station_ids,
            &invalid_type_ids,
            &invalid_station_orders,
        );
        let report_path =
            std::env::var("VALIDATION_REPORT_PATH").unwrap_or("/tmp/validation_report.md".into());
        std::fs::write(&report_path, &report)?;
        eprintln!("Validation report written to {}", report_path);
        panic!("[FATAL] Verification hasn't been passed!");
    }

    println!("[VALID] No errors reported.");
    Ok(())
}

/// `EXPECTED_CONSECUTIVE_STATION_ORDERS` の各エントリについて、対象路線を
/// API と同じ `ORDER BY e_sort, station_cd` で並べたとき期待の駅列が
/// この順で連続して現れることを検証する。
fn validate_station_orders(station_records: &[StringRecord]) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();

    for &(line_cd, expected) in EXPECTED_CONSECUTIVE_STATION_ORDERS {
        // (e_sort, station_cd) で昇順ソート(SQL の ORDER BY と同じタイブレーク)。
        let mut line_stations: Vec<(u32, u32)> = station_records
            .iter()
            .filter(|row| {
                row.get(STATIONS_COL_LINE_CD)
                    .and_then(|v| v.parse::<u32>().ok())
                    == Some(line_cd)
                    && row.get(STATIONS_COL_E_STATUS) == Some("0")
            })
            .filter_map(|row| {
                let e_sort = row
                    .get(STATIONS_COL_E_SORT)
                    .and_then(|v| v.parse::<u32>().ok())?;
                let station_cd = row
                    .get(STATIONS_COL_STATION_CD)
                    .and_then(|v| v.parse::<u32>().ok())?;
                Some((e_sort, station_cd))
            })
            .collect();
        line_stations.sort_unstable();
        let ordered: Vec<u32> = line_stations.iter().map(|&(_, cd)| cd).collect();

        let Some(start) = ordered.iter().position(|&cd| cd == expected[0]) else {
            errors.push(format!(
                "line_cd {line_cd}: station_cd {} が見つかりません",
                expected[0]
            ));
            continue;
        };
        let actual = &ordered[start..(start + expected.len()).min(ordered.len())];
        if actual != expected {
            errors.push(format!(
                "line_cd {line_cd}: e_sort 順で {expected:?} の並びを期待しましたが {actual:?} でした。\
                 e_sort の重複や採番ミスで経路の並び順が崩れると、到着推定(ETA)から駅が欠落します"
            ));
        }
    }

    errors
}

fn build_markdown_report(
    invalid_station_ids: &[String],
    invalid_type_ids: &[String],
    invalid_station_orders: &[String],
) -> String {
    let mut md = String::new();

    md.push_str("<!-- data-validator -->\n");
    md.push_str("## :x: データ整合性チェックに失敗しました\n\n");

    if !invalid_station_ids.is_empty() || !invalid_type_ids.is_empty() {
        md.push_str(
            "`5!station_station_types.csv` に存在しない外部キーへの参照が含まれています。\n\n",
        );
    }

    if !invalid_station_ids.is_empty() {
        md.push_str(&format!(
            "### 不正な Station ID ({} 件)\n\n",
            invalid_station_ids.len()
        ));
        md.push_str("`3!stations.csv` に存在しない `station_cd` が参照されています。\n\n");
        md.push_str("<details>\n<summary>該当レコード一覧</summary>\n\n");
        md.push_str("| 行データ |\n|---|\n");
        for line in invalid_station_ids {
            md.push_str(&format!("| `{}` |\n", escape_markdown_cell(line)));
        }
        md.push_str("\n</details>\n\n");
    }

    if !invalid_type_ids.is_empty() {
        md.push_str(&format!(
            "### 不正な Type ID ({} 件)\n\n",
            invalid_type_ids.len()
        ));
        md.push_str("`4!types.csv` に存在しない `type_cd` が参照されています。\n\n");
        md.push_str("<details>\n<summary>該当レコード一覧</summary>\n\n");
        md.push_str("| 行データ |\n|---|\n");
        for line in invalid_type_ids {
            md.push_str(&format!("| `{}` |\n", escape_markdown_cell(line)));
        }
        md.push_str("\n</details>\n\n");
    }

    if !invalid_station_orders.is_empty() {
        md.push_str(&format!(
            "### 駅の並び順エラー ({} 件)\n\n",
            invalid_station_orders.len()
        ));
        md.push_str(
            "`3!stations.csv` の `e_sort` 順(同値は `station_cd` 順)が期待する駅の並びと一致しません。\n\n",
        );
        for message in invalid_station_orders {
            md.push_str(&format!("- {}\n", escape_markdown_cell(message)));
        }
        md.push('\n');
    }

    md
}

fn escape_markdown_cell(s: &str) -> String {
    s.replace('`', "&#96;").replace('|', "&#124;")
}
