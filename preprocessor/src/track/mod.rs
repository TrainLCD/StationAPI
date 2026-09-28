//! 隣り合う駅のあいだの線路の長さ (`connections`)。
//!
//! 国土数値情報の鉄道データ (N02、国土交通省、CC BY 4.0) の線路区間から求める。
//! 駅の並び (路線の `e_sort` 順と、系統の `station_station_types.id` 順) で
//! 隣り合う鉄道駅の組すべてについて、2 駅を線路へ寄せ、その間の最短経路の長さを
//! 書き出す。Worker はこれを `Station.trackDistanceFromPrevious` として返す。
//!
//! `data/8!connections.csv` に書いた値は計算結果より優先する。N02 の線形が
//! 実際と違う区間を手で直すため。

mod network;

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use stationapi::domain::arrival_estimation::haversine_distance;
use zip::ZipArchive;

use crate::rail::Dataset;
use crate::table::{cell_i32, int, text};
use crate::{info, warn};
use network::{RailNetwork, Section};

/// 国土数値情報 鉄道データ (令和 7 年度)。版を上げるときは [`CACHE_DIR`] と
/// [`GEOJSON_NAME`] も揃えて変える。
const N02_URL: &str = "https://nlftp.mlit.go.jp/ksj/gml/data/N02/N02-25/N02-25_GML.zip";
const CACHE_DIR: &str = "data/N02-25";
/// ZIP の中の線路区間。Shapefile と GeoJSON が Shift-JIS 版と UTF-8 版の両方で
/// 入っているが、読むのは UTF-8 の GeoJSON だけ。
const GEOJSON_NAME: &str = "N02-25_RailroadSection.geojson";

/// `companies.company_name_h` から「株式会社」を除いた名前が N02 の事業者名
/// (`N02_004`) と一致しない会社。公営は N02 では自治体名になる。
///
/// ここに無く名前も一致しない会社 (神戸高速鉄道など、自社で列車を走らせない
/// 会社) は、事業者で絞らずに全事業者の線路で測る。
const OPERATOR_ALIASES: &[(i32, &str)] = &[
    (16, "東急電鉄"),
    (101, "札幌市"),
    (102, "函館市"),
    (107, "アイジーアールいわて銀河鉄道"),
    (115, "仙台市"),
    (119, "東京都"),
    (130, "横浜市"),
    (157, "上田電鉄"),
    (170, "岳南電車"),
    (176, "JR東海交通事業"),
    (179, "名古屋市"),
    (194, "WILLER\u{3000}TRAINS"),
    (195, "京都市"),
    (211, "神戸市"),
    (228, "とさでん交通"),
    (231, "福岡市"),
    (241, "熊本市"),
    (242, "鹿児島市"),
    (252, "一般社団法人札幌市交通事業振興公社"),
];

/// 線路上の距離がこれを超える組は測れなかったものとして扱う。直線距離の 3 倍か、
/// 直線距離 + 5km の大きい方。実在の最大は木次線の出雲坂根〜三井野原
/// (三段スイッチバック) の約 3.6 倍。これより長い経路は、線形が途切れて
/// 別の路線を大回りしたものとみなす。
fn max_track_length(straight: f64) -> f64 {
    (3.0 * straight).max(straight + 5_000.0)
}

/// 環状運転の継ぎ目 (末尾駅 -> 先頭駅) も組にする直線距離の上限。
/// `arrival_estimation::is_circular_route` の上限と揃える。
const SEAM_MAX_METERS: f64 = 3_000.0;
const SEAM_MIN_STATIONS: usize = 6;

pub fn load() -> Result<RailNetwork> {
    let path = Path::new(CACHE_DIR).join(GEOJSON_NAME);
    if path.is_file() {
        info!("国土数値情報 (鉄道) は取得済みなのでダウンロードを省略する");
    } else {
        download(&path)?;
    }

    #[derive(Deserialize)]
    struct FeatureCollection {
        features: Vec<Feature>,
    }
    #[derive(Deserialize)]
    struct Feature {
        properties: Properties,
        geometry: Geometry,
    }
    #[derive(Deserialize)]
    struct Properties {
        #[serde(rename = "N02_004")]
        operator: String,
    }
    /// 線路区間はすべて LineString。
    #[derive(Deserialize)]
    struct Geometry {
        coordinates: Vec<[f64; 2]>,
    }

    let file = File::open(&path).with_context(|| format!("{} を開けない", path.display()))?;
    let collection: FeatureCollection = serde_json::from_reader(BufReader::new(file))
        .with_context(|| format!("{} を読めない", path.display()))?;
    let sections: Vec<Section> = collection
        .features
        .into_iter()
        .map(|f| Section {
            operator: f.properties.operator,
            coordinates: f.geometry.coordinates,
        })
        .collect();
    let network = RailNetwork::build(&sections);
    info!(
        "国土数値情報 (鉄道): 線路区間 {} / 頂点 {} / 辺 {}",
        sections.len(),
        network.vertex_count(),
        network.edge_count()
    );
    Ok(network)
}

/// ZIP を取得して線路区間の GeoJSON だけを取り出す。一時ファイルへ書いてから
/// 置き換えるので、途中で落ちても壊れたキャッシュは残らない。
fn download(path: &Path) -> Result<()> {
    info!("国土数値情報 (鉄道) を取得する");
    let response = reqwest::blocking::get(N02_URL)?;
    if !response.status().is_success() {
        bail!(
            "国土数値情報 (鉄道) の取得に失敗: HTTP {}",
            response.status()
        );
    }
    let bytes = response.bytes()?;

    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let name = archive
        .file_names()
        .find(|name| name.ends_with(&format!("UTF-8/{GEOJSON_NAME}")))
        .map(str::to_string)
        .with_context(|| format!("ZIP に UTF-8/{GEOJSON_NAME} が無い"))?;
    let mut contents = Vec::new();
    archive.by_name(&name)?.read_to_end(&mut contents)?;

    fs::create_dir_all(CACHE_DIR)?;
    let temporary = path.with_extension("geojson.tmp");
    fs::write(&temporary, &contents)?;
    fs::rename(&temporary, path)?;
    info!("{} を書き出した", path.display());
    Ok(())
}

struct StationPoint {
    station_g_cd: i32,
    line_cd: i32,
    active: bool,
    lat: f64,
    lon: f64,
}

/// 隣り合う駅の組ごとの線路の長さを `dataset.connections` へ書き出す。
///
/// `dataset.connections` には `data/8!connections.csv` の行 (手で直した値) が
/// 読み込まれており、同じ組ではそちらを残す。
pub fn generate_connections(dataset: &mut Dataset, network: &RailNetwork) -> Result<()> {
    let stations = rail_stations(dataset);
    let pairs = adjacent_pairs(dataset, &stations);
    let operators = line_operators(dataset, network);

    let c_station1 = dataset.connections.col("station_cd1");
    let c_station2 = dataset.connections.col("station_cd2");
    let c_distance = dataset.connections.col("distance");

    // 手で書いた値。距離の書式は揃えて整数メートルにする。
    let mut distances: HashMap<(i32, i32), i64> = HashMap::new();
    for row in dataset.connections.rows() {
        let (Some(a), Some(b)) = (cell_i32(row, c_station1), cell_i32(row, c_station2)) else {
            bail!("8!connections.csv に駅コードの無い行がある");
        };
        let Some(distance) = row[c_distance]
            .as_deref()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v >= 0.0)
        else {
            bail!("8!connections.csv の {a} - {b} の distance が 0 以上の数値ではない");
        };
        distances.insert(ordered(a, b), distance.round() as i64);
    }
    let manual = distances.len();

    let (mut computed, mut same_group, mut fallback) = (0, 0, 0);
    // 測れなかった組。廃線の駅は N02 に線路が無いので、営業中の駅どうしを分けて数える。
    let (mut missing, mut missing_active) = (0, 0);
    for &(a, b) in &pairs {
        if distances.contains_key(&(a, b)) {
            continue;
        }
        let (sa, sb) = (&stations[&a], &stations[&b]);
        // 直通運転の境界駅は、同じ駅グループの 2 つの駅が系統の中で隣り合う。
        if sa.station_g_cd == sb.station_g_cd {
            distances.insert((a, b), 0);
            same_group += 1;
            continue;
        }

        let straight = haversine_distance(sa.lat, sa.lon, sb.lat, sb.lon);
        let limit = max_track_length(straight);
        // まず両駅の路線の事業者の線路だけで測る。他社線を走る路線
        // (北陸新幹線の上越妙高以西、相鉄・JR直通線など) はそれで線路に寄せられない
        // ので、全事業者の線路で測り直す。
        let own: Option<HashSet<u16>> =
            match (operators.get(&sa.line_cd), operators.get(&sb.line_cd)) {
                (Some(&x), Some(&y)) => Some([x, y].into_iter().collect()),
                _ => None,
            };
        let mut length = own.as_ref().and_then(|ops| {
            network.track_length((sa.lat, sa.lon), (sb.lat, sb.lon), Some(ops), limit)
        });
        if length.is_none() {
            length = network.track_length((sa.lat, sa.lon), (sb.lat, sb.lon), None, limit);
            if length.is_some() && own.is_some() {
                fallback += 1;
            }
        }
        match length {
            // 線路の長さは直線距離より短くならない。短く出るのは、駅の座標と線路の
            // 位置がずれて寄せた位置どうしが近づいたときなので、直線距離で抑える。
            Some(length) => {
                distances.insert((a, b), length.max(straight).round() as i64);
                computed += 1;
            }
            None => {
                missing += 1;
                if sa.active && sb.active {
                    missing_active += 1;
                }
            }
        }
    }

    let mut rows: Vec<((i32, i32), i64)> = distances.into_iter().collect();
    rows.sort_unstable();
    let blank = dataset.connections.blank_row();
    let c_id = dataset.connections.col("id");
    dataset.connections.clear();
    for (i, ((a, b), distance)) in rows.into_iter().enumerate() {
        let mut row = blank.clone();
        row[c_id] = int(i as i32 + 1);
        row[c_station1] = int(a);
        row[c_station2] = int(b);
        row[c_distance] = text(distance.to_string());
        dataset.connections.push(row);
    }

    info!(
        "線路の長さ: 駅の組 {} / 計算 {computed} (うち全事業者で測り直し {fallback}) / \
         同じ駅グループ {same_group} / 手入力 {manual} / 測れず {missing} (うち営業中の駅どうし {missing_active})",
        pairs.len()
    );
    if missing_active > pairs.len() / 100 {
        warn!(
            "営業中の駅どうしで線路の長さを測れなかった組が {missing_active} 件ある。\
             N02 の版や OPERATOR_ALIASES を確認すること"
        );
    }
    Ok(())
}

fn ordered(a: i32, b: i32) -> (i32, i32) {
    (a.min(b), a.max(b))
}

/// 鉄道駅 (バス停を除く) を station_cd で引けるようにする。
fn rail_stations(dataset: &Dataset) -> HashMap<i32, StationPoint> {
    let t = &dataset.stations;
    let (c_cd, c_g_cd, c_line, c_status, c_transport) = (
        t.col("station_cd"),
        t.col("station_g_cd"),
        t.col("line_cd"),
        t.col("e_status"),
        t.col("transport_type"),
    );
    let (c_lat, c_lon) = (t.col("lat"), t.col("lon"));
    let coordinate = |row: &[Option<String>], idx: usize| -> Option<f64> {
        row[idx].as_deref().and_then(|v| v.trim().parse().ok())
    };
    t.rows()
        .iter()
        .filter(|row| cell_i32(row, c_transport) == Some(0))
        .filter_map(|row| {
            Some((
                cell_i32(row, c_cd)?,
                StationPoint {
                    station_g_cd: cell_i32(row, c_g_cd)?,
                    line_cd: cell_i32(row, c_line)?,
                    active: cell_i32(row, c_status) == Some(0),
                    lat: coordinate(row, c_lat)?,
                    lon: coordinate(row, c_lon)?,
                },
            ))
        })
        .collect()
}

/// API が返す駅の並びで隣り合う組 (小さい station_cd が先)。
///
/// 並びは路線の (e_sort, station_cd) 順 (`lineStations` で系統を選べない路線) と、
/// 系統の sst.id 順 (`lineGroupStations` / `trainRoute` / 系統を選べた
/// `lineStations`)。Worker は廃止駅 (e_status != 0) を除いて返すので、除いた並びと
/// 除かない並びの両方から組を取る。環状運転の系統は末尾駅から先頭駅へ戻る組も取る
/// (`trainRoute` は継ぎ目をまたいで切り出す)。
fn adjacent_pairs(dataset: &Dataset, stations: &HashMap<i32, StationPoint>) -> Vec<(i32, i32)> {
    let mut sequences: Vec<Vec<i32>> = Vec::new();

    let t = &dataset.stations;
    let (c_cd, c_line, c_sort) = (t.col("station_cd"), t.col("line_cd"), t.col("e_sort"));
    let mut by_line: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();
    for row in t.rows() {
        let (Some(cd), Some(line)) = (cell_i32(row, c_cd), cell_i32(row, c_line)) else {
            continue;
        };
        if stations.contains_key(&cd) {
            by_line
                .entry(line)
                .or_default()
                .push((cell_i32(row, c_sort).unwrap_or(0), cd));
        }
    }
    for mut line in by_line.into_values() {
        line.sort_unstable();
        sequences.push(line.into_iter().map(|(_, cd)| cd).collect());
    }

    // sst は id 順 (= 停車順) に並んでいる。
    let s = &dataset.sst;
    let (c_station, c_group) = (s.col("station_cd"), s.col("line_group_cd"));
    let mut by_group: HashMap<i32, Vec<i32>> = HashMap::new();
    for row in s.rows() {
        let (Some(cd), Some(group)) = (cell_i32(row, c_station), cell_i32(row, c_group)) else {
            continue;
        };
        if stations.contains_key(&cd) {
            by_group.entry(group).or_default().push(cd);
        }
    }
    sequences.extend(by_group.into_values());

    let mut pairs: HashSet<(i32, i32)> = HashSet::new();
    for all in sequences {
        let active: Vec<i32> = all
            .iter()
            .copied()
            .filter(|cd| stations[cd].active)
            .collect();
        for sequence in [all, active] {
            for w in sequence.windows(2) {
                if w[0] != w[1] {
                    pairs.insert(ordered(w[0], w[1]));
                }
            }
            if let (true, Some(&first), Some(&last)) = (
                sequence.len() >= SEAM_MIN_STATIONS,
                sequence.first(),
                sequence.last(),
            ) {
                let (f, l) = (&stations[&first], &stations[&last]);
                if first != last
                    && haversine_distance(f.lat, f.lon, l.lat, l.lon) <= SEAM_MAX_METERS
                {
                    pairs.insert(ordered(first, last));
                }
            }
        }
    }

    let mut pairs: Vec<(i32, i32)> = pairs.into_iter().collect();
    pairs.sort_unstable();
    pairs
}

/// 路線 -> その路線の会社の、N02 での事業者番号。N02 に無い会社の路線は入らない。
fn line_operators(dataset: &Dataset, network: &RailNetwork) -> HashMap<i32, u16> {
    let c = &dataset.companies;
    let (c_cd, c_name) = (c.col("company_cd"), c.col("company_name_h"));
    let aliases: HashMap<i32, &str> = OPERATOR_ALIASES.iter().copied().collect();
    let company_operator: HashMap<i32, u16> = c
        .rows()
        .iter()
        .filter_map(|row| {
            let cd = cell_i32(row, c_cd)?;
            let name = match aliases.get(&cd) {
                Some(alias) => (*alias).to_string(),
                None => row[c_name]
                    .as_deref()?
                    .replace("株式会社", "")
                    .trim()
                    .to_string(),
            };
            Some((cd, network.operator_id(&name)?))
        })
        .collect();

    let l = &dataset.lines;
    let (l_cd, l_company) = (l.col("line_cd"), l.col("company_cd"));
    l.rows()
        .iter()
        .filter_map(|row| {
            let operator = company_operator.get(&cell_i32(row, l_company)?)?;
            Some((cell_i32(row, l_cd)?, *operator))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rail::{
        ALIAS_COLUMNS, COMPANY_COLUMNS, CONNECTION_COLUMNS, LINE_ALIAS_COLUMNS, LINE_COLUMNS,
        SST_COLUMNS, STATION_COLUMNS, TYPE_COLUMNS,
    };
    use crate::table::Table;

    fn push(table: &mut Table, cells: &[(&str, &str)]) {
        let mut row = table.blank_row();
        for (column, value) in cells {
            row[table.col(column)] = text(*value);
        }
        table.push(row);
    }

    /// 経度 139.00〜139.03 を東西に走る「テスト鉄道」の線路と、その上の駅。
    ///
    /// - 路線 10: 101 (139.00) - 102 (139.01) - 103 (139.02) - 104 (線路から約 2.2km 北)
    /// - 路線 20: 201 (103 と同じ駅グループ)
    /// - 系統 900: 101 - 102 - 103 - 201 (直通の境界駅で同じ駅グループが隣り合う)
    fn fixture() -> (Dataset, RailNetwork) {
        let mut dataset = Dataset {
            companies: Table::new(COMPANY_COLUMNS, Some("company_cd")),
            lines: Table::new(LINE_COLUMNS, Some("line_cd")),
            stations: Table::new(STATION_COLUMNS, Some("station_cd")),
            types: Table::new(TYPE_COLUMNS, Some("type_cd")),
            sst: Table::new(SST_COLUMNS, None),
            aliases: Table::new(ALIAS_COLUMNS, Some("id")),
            line_aliases: Table::new(LINE_ALIAS_COLUMNS, Some("id")),
            connections: Table::new(CONNECTION_COLUMNS, None),
        };
        push(
            &mut dataset.companies,
            &[
                ("company_cd", "1"),
                ("company_name_h", "テスト鉄道株式会社"),
            ],
        );
        for line in ["10", "20"] {
            push(
                &mut dataset.lines,
                &[("line_cd", line), ("company_cd", "1")],
            );
        }
        for (cd, g_cd, line, sort, lat, lon) in [
            ("101", "101", "10", "1", "35.0", "139.00"),
            ("102", "102", "10", "2", "35.0", "139.01"),
            ("103", "103", "10", "3", "35.0", "139.02"),
            ("104", "104", "10", "4", "35.02", "139.03"),
            ("201", "103", "20", "1", "35.0", "139.02"),
        ] {
            push(
                &mut dataset.stations,
                &[
                    ("station_cd", cd),
                    ("station_g_cd", g_cd),
                    ("line_cd", line),
                    ("e_sort", sort),
                    ("e_status", "0"),
                    ("transport_type", "0"),
                    ("lat", lat),
                    ("lon", lon),
                ],
            );
        }
        for cd in ["101", "102", "103", "201"] {
            push(
                &mut dataset.sst,
                &[
                    ("station_cd", cd),
                    ("type_cd", "100"),
                    ("line_group_cd", "900"),
                ],
            );
        }
        let network = RailNetwork::build(&[Section {
            operator: "テスト鉄道".to_string(),
            coordinates: vec![
                [139.00, 35.0],
                [139.01, 35.0],
                [139.02, 35.0],
                [139.03, 35.0],
            ],
        }]);
        (dataset, network)
    }

    fn connections(dataset: &Dataset) -> Vec<(i32, i32, String)> {
        let t = &dataset.connections;
        let (a, b, d) = (
            t.col("station_cd1"),
            t.col("station_cd2"),
            t.col("distance"),
        );
        t.rows()
            .iter()
            .map(|row| {
                (
                    cell_i32(row, a).unwrap(),
                    cell_i32(row, b).unwrap(),
                    row[d].clone().unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn measures_adjacent_stations_and_keeps_manual_values() {
        let (mut dataset, network) = fixture();
        // 手で直した値は向きを問わず計算結果より優先し、整数メートルに揃える
        push(
            &mut dataset.connections,
            &[
                ("station_cd1", "102"),
                ("station_cd2", "101"),
                ("distance", "1234.4"),
            ],
        );

        generate_connections(&mut dataset, &network).unwrap();

        let straight = haversine_distance(35.0, 139.01, 35.0, 139.02).round();
        assert_eq!(
            connections(&dataset),
            vec![
                (101, 102, "1234".to_string()),
                (102, 103, straight.to_string()),
                // 104 は線路から遠いので測れない。同じ駅グループの 103 - 201 は 0
                (103, 201, "0".to_string()),
            ]
        );
        let ids: Vec<Option<i32>> = dataset
            .connections
            .rows()
            .iter()
            .map(|row| cell_i32(row, dataset.connections.col("id")))
            .collect();
        assert_eq!(ids, vec![Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn rejects_manual_rows_without_a_valid_distance() {
        let (mut dataset, network) = fixture();
        push(
            &mut dataset.connections,
            &[
                ("station_cd1", "101"),
                ("station_cd2", "102"),
                ("distance", "-1"),
            ],
        );
        assert!(generate_connections(&mut dataset, &network).is_err());
    }

    #[test]
    fn pairs_come_from_line_order_and_group_order() {
        let (dataset, _) = fixture();
        let stations = rail_stations(&dataset);
        assert_eq!(
            adjacent_pairs(&dataset, &stations),
            vec![(101, 102), (102, 103), (103, 104), (103, 201)]
        );
    }
}
