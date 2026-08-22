//! `data/*.csv` の読み込みと、鉄道側の派生データ生成。

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::table::{cell_i32, int, Table};
use crate::{info, warn};

/// 出力する 7 テーブルの列。PostgreSQL 版の `information_schema` 由来の並びと同じ。
/// `transport_type` は後から `ALTER TABLE` で足された列なので末尾に来る。
pub const COMPANY_COLUMNS: &[&str] = &[
    "company_cd",
    "rr_cd",
    "company_name",
    "company_name_k",
    "company_name_h",
    "company_name_r",
    "company_name_en",
    "company_name_full_en",
    "company_url",
    "company_type",
    "e_status",
    "e_sort",
];

pub const LINE_COLUMNS: &[&str] = &[
    "line_cd",
    "company_cd",
    "line_name",
    "line_name_k",
    "line_name_h",
    "line_name_r",
    "line_name_rn",
    "line_name_zh",
    "line_name_ko",
    "line_color_c",
    "line_type",
    "line_symbol1",
    "line_symbol2",
    "line_symbol3",
    "line_symbol4",
    "line_symbol1_color",
    "line_symbol2_color",
    "line_symbol3_color",
    "line_symbol4_color",
    "line_symbol1_shape",
    "line_symbol2_shape",
    "line_symbol3_shape",
    "line_symbol4_shape",
    "e_status",
    "e_sort",
    "average_distance",
    "transport_type",
];

pub const STATION_COLUMNS: &[&str] = &[
    "station_cd",
    "station_g_cd",
    "station_name",
    "station_name_k",
    "station_name_r",
    "station_name_rn",
    "station_name_zh",
    "station_name_ko",
    "station_number1",
    "station_number2",
    "station_number3",
    "station_number4",
    "three_letter_code",
    "line_cd",
    "pref_cd",
    "post",
    "address",
    "lon",
    "lat",
    "open_ymd",
    "close_ymd",
    "e_status",
    "e_sort",
    "transport_type",
];

pub const TYPE_COLUMNS: &[&str] = &[
    "id",
    "type_cd",
    "type_name",
    "type_name_k",
    "type_name_r",
    "type_name_zh",
    "type_name_ko",
    "color",
    "direction",
    "kind",
    "priority",
];

pub const SST_COLUMNS: &[&str] = &["id", "station_cd", "type_cd", "line_group_cd", "pass"];

pub const ALIAS_COLUMNS: &[&str] = &[
    "id",
    "line_name",
    "line_name_k",
    "line_name_h",
    "line_name_r",
    "line_name_zh",
    "line_name_ko",
    "line_color_c",
];

pub const LINE_ALIAS_COLUMNS: &[&str] = &["id", "station_cd", "alias_cd"];

/// 種別を持たない路線へ補う各駅停車の既定種別。
const DEFAULT_RAIL_TYPE_CD: i32 = 100;
/// 「各駅停車」と呼ぶ路線に使う種別。
const LOCAL_RAIL_TYPE_CD: i32 = 101;
/// 生成した系統の line_group_cd はこの値に line_cd を足して作る。
const VIRTUAL_RAIL_LINE_GROUP_BASE: i32 = 1_000_000_000;

/// 各駅停車の呼称が「普通」ではなく「各駅停車」である路線。
///
/// 事業者単位ではなく路線単位にしているのは、JR 東日本のように路線によって
/// 呼称が違う事業者があるため。新しい路線を足すときはここも見直す。
const LOCAL_SERVICE_RAIL_LINE_IDS: &[i32] = &[
    11309, // JR東日本 相模線
    11318, // JR東日本 八高線
    11345, // ディズニーリゾートライン
    99101, // 札幌市営地下鉄東西線
    99102, // 札幌市営地下鉄南北線
    99103, // 札幌市営地下鉄東豊線
    99301, // 都営大江戸線
    99305, // 東京さくらトラム
    99342, // 日暮里・舎人ライナー
    99649, // 六甲ライナー
];

/// 出力対象のテーブル一式。
pub struct Dataset {
    pub companies: Table,
    pub lines: Table,
    pub stations: Table,
    pub types: Table,
    pub sst: Table,
    pub aliases: Table,
    pub line_aliases: Table,
}

impl Dataset {
    /// `data/` 配下の CSV を読み込む。
    ///
    /// PostgreSQL 版は `INSERT INTO <table> VALUES (...)` を列指定なしで流していたため、
    /// CSV に無い末尾の列 (`transport_type`) は DDL の既定値で埋まっていた。ここでも
    /// 同じ既定値を入れる。`#` 始まりの列は取り込み対象外。
    pub fn load(data_dir: &Path) -> Result<Self> {
        let mut dataset = Dataset {
            companies: Table::new(COMPANY_COLUMNS, Some("company_cd")),
            lines: Table::new(LINE_COLUMNS, Some("line_cd")),
            stations: Table::new(STATION_COLUMNS, Some("station_cd")),
            types: Table::new(TYPE_COLUMNS, Some("type_cd")),
            sst: Table::new(SST_COLUMNS, None),
            aliases: Table::new(ALIAS_COLUMNS, Some("id")),
            line_aliases: Table::new(LINE_ALIAS_COLUMNS, Some("id")),
        };

        load_csv(&mut dataset.companies, &data_dir.join("1!companies.csv"))?;
        load_csv(&mut dataset.lines, &data_dir.join("2!lines.csv"))?;
        load_csv(&mut dataset.stations, &data_dir.join("3!stations.csv"))?;
        load_csv(&mut dataset.types, &data_dir.join("4!types.csv"))?;
        load_csv(
            &mut dataset.sst,
            &data_dir.join("5!station_station_types.csv"),
        )?;
        load_csv(&mut dataset.aliases, &data_dir.join("6!aliases.csv"))?;
        load_csv(
            &mut dataset.line_aliases,
            &data_dir.join("7!line_aliases.csv"),
        )?;

        // transport_type は CSV に無いので既定値を入れる (0 = 鉄道)。
        fill_default(&mut dataset.lines, "transport_type", "0");
        fill_default(&mut dataset.stations, "transport_type", "0");
        // average_distance は real (4 バイト浮動小数) なので、その精度へ丸める。
        // 例: 31664.84112 -> 31664.842
        normalize_real(&mut dataset.lines, "average_distance");
        // id は SERIAL。CSV では "DEFAULT" と書かれているので採番し直す。
        assign_serial(&mut dataset.types, "id");
        assign_serial(&mut dataset.sst, "id");

        info!(
            "取り込み: companies={} lines={} stations={} types={} sst={} aliases={} line_aliases={}",
            dataset.companies.len(),
            dataset.lines.len(),
            dataset.stations.len(),
            dataset.types.len(),
            dataset.sst.len(),
            dataset.aliases.len(),
            dataset.line_aliases.len(),
        );

        Ok(dataset)
    }

    /// 列車種別を 1 つも持たない駅がある鉄道路線へ、全駅停車の系統を 1 本補う。
    ///
    /// `data/*.csv` には無い行なので、CSV をそのまま読むだけでは
    /// `has_train_types` が偽になる駅が大量に出る。
    ///
    /// `station_station_types.id` は停車順序そのものとして参照されるため、
    /// 路線・駅順に並べて追加する。
    pub fn generate_virtual_local_rail_services(&mut self) -> Result<()> {
        let type_cd_col = self.types.col("type_cd");
        let present: HashSet<i32> = self
            .types
            .rows()
            .iter()
            .filter_map(|row| cell_i32(row, type_cd_col))
            .collect();
        let missing: Vec<i32> = [DEFAULT_RAIL_TYPE_CD, LOCAL_RAIL_TYPE_CD]
            .into_iter()
            .filter(|cd| !present.contains(cd))
            .collect();
        if !missing.is_empty() {
            bail!("各駅停車の系統を生成できない: types に type_cd {missing:?} が無い");
        }

        let s_line_cd = self.stations.col("line_cd");
        let s_station_cd = self.stations.col("station_cd");
        let s_e_status = self.stations.col("e_status");
        let s_e_sort = self.stations.col("e_sort");
        let s_transport = self.stations.col("transport_type");
        let l_line_cd = self.lines.col("line_cd");
        let l_e_status = self.lines.col("e_status");
        let l_transport = self.lines.col("transport_type");
        let sst_station_cd = self.sst.col("station_cd");
        let sst_type_cd = self.sst.col("type_cd");
        let sst_line_group = self.sst.col("line_group_cd");

        // 既に何らかの系統に属している駅。
        let typed_stations: HashSet<i32> = self
            .sst
            .rows()
            .iter()
            .filter_map(|row| cell_i32(row, sst_station_cd))
            .collect();

        // 有効な鉄道駅を路線ごとに集める。(e_sort, station_cd) は挿入順序に使う。
        let mut stations_by_line: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();
        for row in self.stations.rows() {
            if cell_i32(row, s_e_status) != Some(0) || cell_i32(row, s_transport) != Some(0) {
                continue;
            }
            let (Some(line_cd), Some(station_cd)) =
                (cell_i32(row, s_line_cd), cell_i32(row, s_station_cd))
            else {
                continue;
            };
            stations_by_line
                .entry(line_cd)
                .or_default()
                .push((cell_i32(row, s_e_sort).unwrap_or(0), station_cd));
        }

        // 駅ごとに付いている 100 / 101 の系統を数える。既定種別の推定に使う。
        let mut station_to_line: HashMap<i32, i32> = HashMap::new();
        for row in self.stations.rows() {
            if cell_i32(row, s_e_status) != Some(0) || cell_i32(row, s_transport) != Some(0) {
                continue;
            }
            if let (Some(station_cd), Some(line_cd)) =
                (cell_i32(row, s_station_cd), cell_i32(row, s_line_cd))
            {
                station_to_line.insert(station_cd, line_cd);
            }
        }
        let mut local_type_counts: HashMap<(i32, i32), usize> = HashMap::new();
        for row in self.sst.rows() {
            let (Some(station_cd), Some(type_cd)) =
                (cell_i32(row, sst_station_cd), cell_i32(row, sst_type_cd))
            else {
                continue;
            };
            if type_cd != DEFAULT_RAIL_TYPE_CD && type_cd != LOCAL_RAIL_TYPE_CD {
                continue;
            }
            if let Some(line_cd) = station_to_line.get(&station_cd) {
                *local_type_counts.entry((*line_cd, type_cd)).or_default() += 1;
            }
        }

        // 生成対象の路線: 有効な鉄道路線で、系統を持たない有効駅が 1 つ以上あるもの。
        let mut target_lines: Vec<(i32, i32)> = Vec::new();
        for row in self.lines.rows() {
            if cell_i32(row, l_e_status) != Some(0) || cell_i32(row, l_transport) != Some(0) {
                continue;
            }
            let Some(line_cd) = cell_i32(row, l_line_cd) else {
                continue;
            };
            let Some(stations) = stations_by_line.get(&line_cd) else {
                continue;
            };
            if !stations
                .iter()
                .any(|(_, station_cd)| !typed_stations.contains(station_cd))
            {
                continue;
            }

            // 既にその路線で使われている 100 / 101 のうち多い方を採る。
            // 同数なら type_cd の小さい方。どちらも無ければ路線ごとの既定値。
            let candidates = [DEFAULT_RAIL_TYPE_CD, LOCAL_RAIL_TYPE_CD];
            let type_cd = candidates
                .iter()
                .filter_map(|cd| {
                    local_type_counts
                        .get(&(line_cd, *cd))
                        .map(|count| (*count, *cd))
                })
                .max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)))
                .map(|(_, cd)| cd)
                .unwrap_or(if LOCAL_SERVICE_RAIL_LINE_IDS.contains(&line_cd) {
                    LOCAL_RAIL_TYPE_CD
                } else {
                    DEFAULT_RAIL_TYPE_CD
                });
            target_lines.push((line_cd, type_cd));
        }

        // line_group_cd が既存と衝突していないか確認する。衝突したまま入れると
        // 別路線の停車順序に混ざる。
        let existing_groups: HashSet<i32> = self
            .sst
            .rows()
            .iter()
            .filter_map(|row| cell_i32(row, sst_line_group))
            .collect();
        let colliding: Vec<i32> = target_lines
            .iter()
            .map(|(line_cd, _)| VIRTUAL_RAIL_LINE_GROUP_BASE + line_cd)
            .filter(|group| existing_groups.contains(group))
            .collect();
        if !colliding.is_empty() {
            bail!("各駅停車の系統を生成できない: line_group_cd {colliding:?} が衝突している");
        }

        target_lines.sort_by_key(|(line_cd, _)| *line_cd);

        let mut generated = 0usize;
        for (line_cd, type_cd) in target_lines {
            let line_group_cd = VIRTUAL_RAIL_LINE_GROUP_BASE + line_cd;
            let mut stations = stations_by_line.get(&line_cd).cloned().unwrap_or_default();
            stations.sort_by_key(|(e_sort, station_cd)| (*e_sort, *station_cd));
            for (_, station_cd) in stations {
                let mut row = self.sst.blank_row();
                row[sst_station_cd] = int(station_cd);
                row[sst_type_cd] = int(type_cd);
                row[sst_line_group] = int(line_group_cd);
                row[self.sst.col("pass")] = int(0);
                self.sst.push(row);
                generated += 1;
            }
        }

        assign_serial(&mut self.sst, "id");
        info!("各駅停車の系統を {generated} 行生成した");
        Ok(())
    }
}

/// CSV を読んでテーブルへ流し込む。`#` 始まりの列は捨てる。
fn load_csv(table: &mut Table, path: &Path) -> Result<()> {
    let mut reader = csv::ReaderBuilder::new()
        .from_path(path)
        .with_context(|| format!("{} を開けない", path.display()))?;
    let headers = reader.headers()?.clone();

    // CSV の列 -> テーブルの列。`#` 始まりと、テーブルに無い列は None。
    let mapping: Vec<Option<usize>> = headers
        .iter()
        .map(|header| {
            if header.starts_with('#') {
                return None;
            }
            table.columns().iter().position(|column| column == header)
        })
        .collect();

    for (i, header) in headers.iter().enumerate() {
        if !header.starts_with('#') && mapping[i].is_none() {
            warn!(
                "{} の列 {header} は出力対象に無いため無視する",
                path.display()
            );
        }
    }

    for record in reader.records() {
        let record = record?;
        let mut row = table.blank_row();
        for (i, value) in record.iter().enumerate() {
            let Some(Some(target)) = mapping.get(i) else {
                continue;
            };
            // 空欄は NULL、"DEFAULT" は後段の採番で埋める。
            row[*target] = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
        }
        table.push(row);
    }
    Ok(())
}

/// 値が入っていない列を既定値で埋める。
fn fill_default(table: &mut Table, column: &str, default: &str) {
    let idx = table.col(column);
    for row in table.rows_mut() {
        if row[idx].is_none() {
            row[idx] = Some(default.to_string());
        }
    }
}

/// `real` 列を 4 バイト浮動小数へ丸める。
///
/// この列は API では `Float` (倍精度) として返る。元は PostgreSQL の `real` 列
/// だったので、まず `f32` へ落としたうえで、その値を `f64` として書き出す。
/// `f32` の最短表記 (31664.842) で書くと読み直したときに別の値になり、
/// API の応答が 31664.842 と 31664.841796875 でずれる。
fn normalize_real(table: &mut Table, column: &str) {
    let idx = table.col(column);
    for row in table.rows_mut() {
        if let Some(value) = row[idx].as_deref().and_then(|v| v.parse::<f64>().ok()) {
            row[idx] = Some(((value as f32) as f64).to_string());
        }
    }
}

/// SERIAL 列を 1 始まりの連番で振り直す。
pub fn assign_serial(table: &mut Table, column: &str) {
    let idx = table.col(column);
    for (i, row) in table.rows_mut().iter_mut().enumerate() {
        row[idx] = Some((i + 1).to_string());
    }
}
