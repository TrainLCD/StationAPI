//! `generated/*.csv` の書き出し。
//!
//! Worker のビルドがそのまま読むので、列の並びと行の並びを固定する。
//! 特に `station_station_types` は `id` の昇順が停車順序そのものなので、
//! ここで崩すと停車駅の並びが壊れる。

use std::path::Path;

use anyhow::{Context, Result};

use crate::info;
use crate::rail::Dataset;
use crate::table::Table;

/// 書き出すテーブルと、行を並べる列。
const OUTPUTS: &[(&str, &str)] = &[
    ("companies", "company_cd"),
    ("lines", "line_cd"),
    ("stations", "station_cd"),
    ("types", "id"),
    // sst.id は停車順序そのものなので必ず id 昇順で出す。
    ("station_station_types", "id"),
    ("aliases", "id"),
    ("line_aliases", "id"),
];

pub fn write_all(dataset: &mut Dataset, out_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("{} を作れない", out_dir.display()))?;

    for (name, order_by) in OUTPUTS {
        let table = match *name {
            "companies" => &mut dataset.companies,
            "lines" => &mut dataset.lines,
            "stations" => &mut dataset.stations,
            "types" => &mut dataset.types,
            "station_station_types" => &mut dataset.sst,
            "aliases" => &mut dataset.aliases,
            "line_aliases" => &mut dataset.line_aliases,
            other => unreachable!("未知のテーブル {other}"),
        };
        table.sort_by_int_col(order_by);
        let path = out_dir.join(format!("{name}.csv"));
        write_table(table, &path)?;
        info!("{} へ {} 行書き出した", path.display(), table.len());
    }
    Ok(())
}

fn write_table(table: &Table, path: &Path) -> Result<()> {
    let mut writer =
        csv::Writer::from_path(path).with_context(|| format!("{} を作れない", path.display()))?;
    writer.write_record(table.columns())?;
    for row in table.rows() {
        // NULL と空文字はどちらも空欄として書く。
        writer.write_record(row.iter().map(|cell| cell.as_deref().unwrap_or("")))?;
    }
    writer.flush()?;
    Ok(())
}
