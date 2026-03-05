use core::panic;
use std::collections::HashSet;
use std::path::Path;

use csv::{ReaderBuilder, StringRecord};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut invalid_station_ids: Vec<String> = Vec::new();
    let mut invalid_type_ids: Vec<String> = Vec::new();

    let data_path: &Path = Path::new("data");
    let mut rdr = ReaderBuilder::new().from_path(data_path.join("3!stations.csv"))?;
    let records: Vec<StringRecord> = rdr.records().filter_map(|row| row.ok()).collect();
    let station_ids: HashSet<u32> = records
        .iter()
        .map(|row| row.get(0).unwrap().parse::<u32>().unwrap())
        .collect();

    let mut rdr = ReaderBuilder::new().from_path(data_path.join("4!types.csv"))?;
    let records: Vec<StringRecord> = rdr.records().filter_map(|row| row.ok()).collect();
    let type_ids: HashSet<u32> = records
        .iter()
        .map(|row| row.get(1).unwrap().parse::<u32>().unwrap())
        .collect();

    let mut rdr = ReaderBuilder::new().from_path(data_path.join("5!station_station_types.csv"))?;
    let records: Vec<StringRecord> = rdr.records().filter_map(|row| row.ok()).collect();

    for record in &records {
        let station_cd: u32 = record.get(1).unwrap().parse().unwrap();
        let type_cd: u32 = record.get(2).unwrap().parse().unwrap();
        let line = || record.iter().collect::<Vec<&str>>().join(",");

        if !station_ids.contains(&station_cd) {
            println!("[INVALID] Unrecognized Station ID {:?} Found!", station_cd);
            invalid_station_ids.push(line());
        }
        if !type_ids.contains(&type_cd) {
            println!("[INVALID] Unrecognized Type ID {:?} Found!", type_cd);
            invalid_type_ids.push(line());
        }
    }

    let has_err = !invalid_station_ids.is_empty() || !invalid_type_ids.is_empty();

    if has_err {
        let report = build_markdown_report(&invalid_station_ids, &invalid_type_ids);
        let report_path =
            std::env::var("VALIDATION_REPORT_PATH").unwrap_or("/tmp/validation_report.md".into());
        std::fs::write(&report_path, &report)?;
        eprintln!("Validation report written to {}", report_path);
        panic!("[FATAL] Verification hasn't been passed!");
    }

    println!("[VALID] No errors reported.");
    Ok(())
}

fn build_markdown_report(invalid_station_ids: &[String], invalid_type_ids: &[String]) -> String {
    let mut md = String::new();

    md.push_str("<!-- data-validator -->\n");
    md.push_str("## :x: データ整合性チェックに失敗しました\n\n");
    md.push_str("`5!station_station_types.csv` に存在しない外部キーへの参照が含まれています。\n\n");

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

    md
}

fn escape_markdown_cell(s: &str) -> String {
    s.replace('`', "&#96;").replace('|', "&#124;")
}
