//! GTFS のテキストファイルを読む。
//!
//! 列順と省略される任意列はフィードごとに異なるため、列名で解決する。

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use stationapi::domain::romaji::{romaji_display_name, to_fullwidth_katakana};

use super::feed::{
    hokkaido_express_company_cd, scoped_id, uses_stop_order_patterns, GtfsFeed,
    HOKKAIDO_EXPRESS_BUS, NAGOYA_CITY_BUS,
};
use super::model::{index_stop_times_by_trip, GtfsData, Route, Stop, StopTime, Trip};
use crate::warn;

/// `translations.txt` の 1 エントリ。
#[derive(Debug, Clone, Default)]
pub struct Translation {
    pub ja_hrkt: Option<String>,
    pub en: Option<String>,
    pub zh: Option<String>,
    pub ko: Option<String>,
}

pub type Translations = HashMap<String, Translation>;

/// 1 フィードを読み込んで `data` へ足す。
pub fn load_feed(data: &mut GtfsData, feed: &GtfsFeed) -> Result<()> {
    let dir = Path::new(feed.path);
    if !dir.exists() {
        warn!("{} のディレクトリが無いので読み飛ばす", feed.name);
        return Ok(());
    }

    let scheduled_services = (feed.id == NAGOYA_CITY_BUS.id)
        .then(|| load_scheduled_services(dir))
        .transpose()?;
    let translations = load_translations(dir)?;
    load_routes(data, dir, feed)?;
    load_stops(data, dir, feed, &translations)?;
    let trip_start = data.trips.len();
    let stop_time_start = data.stop_times.len();
    load_trips(data, dir, feed, scheduled_services.as_ref())?;
    load_stop_times(data, dir, feed)?;
    if uses_stop_order_patterns(feed.id) {
        derive_stop_order_patterns(
            &mut data.trips[trip_start..],
            &data.stop_times[stop_time_start..],
        );
    }
    if feed.id == "nagoya_srt" {
        normalize_srt_headsigns(data);
    }
    Ok(())
}

/// shape_id が無い便は、停留所の並びを運行パターンのキーにする。
fn derive_stop_order_patterns(trips: &mut [Trip], stop_times: &[StopTime]) {
    let by_trip = index_stop_times_by_trip(stop_times);
    for trip in trips.iter_mut().filter(|trip| trip.shape_id.is_none()) {
        let Some(times) = by_trip.get(trip.trip_id.as_str()) else {
            continue;
        };
        let mut pattern = String::from("stop_order:");
        for time in times {
            // ID に区切り文字が含まれても並びが衝突しないよう、長さを先に書く。
            pattern.push_str(&time.stop_id.len().to_string());
            pattern.push(':');
            pattern.push_str(&time.stop_id);
        }
        trip.shape_id = Some(pattern);
    }
}

/// 運行曜日または追加運行日がある service_id を集めます。
/// 市バスの GTFS には運行休止中の便も収録されています。
fn load_scheduled_services(dir: &Path) -> Result<HashSet<String>> {
    #[derive(Deserialize)]
    struct Calendar {
        service_id: String,
        monday: u8,
        tuesday: u8,
        wednesday: u8,
        thursday: u8,
        friday: u8,
        saturday: u8,
        sunday: u8,
    }

    #[derive(Deserialize)]
    struct CalendarDate {
        service_id: String,
        exception_type: u8,
    }

    let mut services = HashSet::new();
    let calendar = dir.join("calendar.txt");
    if calendar.exists() {
        let mut rdr = csv::Reader::from_path(calendar)?;
        for row in rdr.deserialize::<Calendar>() {
            let row = row?;
            if [
                row.monday,
                row.tuesday,
                row.wednesday,
                row.thursday,
                row.friday,
                row.saturday,
                row.sunday,
            ]
            .contains(&1)
            {
                services.insert(row.service_id);
            }
        }
    }
    let calendar_dates = dir.join("calendar_dates.txt");
    if calendar_dates.exists() {
        let mut rdr = csv::Reader::from_path(calendar_dates)?;
        for row in rdr.deserialize::<CalendarDate>() {
            let row = row?;
            if row.exception_type == 1 {
                services.insert(row.service_id);
            }
        }
    }
    ensure!(!services.is_empty(), "名古屋市バスの運行日を読み取れない");
    Ok(services)
}

/// SRTのtrip_headsignは「通常便」「最終便」などの便区分です。
/// 行先として表示する名称は、循環便では路線名、それ以外では終点から補います。
fn normalize_srt_headsigns(data: &mut GtfsData) {
    let times = data.stop_times_by_trip();
    let names: HashMap<_, _> = data
        .stops
        .iter()
        .map(|stop| (stop.stop_id.as_str(), stop.stop_name.as_str()))
        .collect();
    let routes: HashMap<_, _> = data
        .routes
        .iter()
        .map(|route| (route.route_id.as_str(), route.route_long_name.as_deref()))
        .collect();
    let headsigns: HashMap<_, _> = data
        .trips
        .iter()
        .filter(|trip| trip.route_id.starts_with("nagoya_srt:"))
        .filter_map(|trip| {
            let stops = times.get(trip.trip_id.as_str())?;
            let first = stops.first()?;
            let last = stops.last()?;
            let name = if first.stop_id == last.stop_id {
                routes.get(trip.route_id.as_str()).copied().flatten()
            } else {
                names.get(last.stop_id.as_str()).copied()
            }?;
            Some((trip.trip_id.clone(), name.to_string()))
        })
        .collect();
    for trip in &mut data.trips {
        if let Some(headsign) = headsigns.get(&trip.trip_id) {
            trip.trip_headsign = Some(headsign.clone());
        }
    }
}

/// CSVを開きます。必須ファイルが無い場合は、デプロイ検証で検出する警告を出します。
fn reader(path: &Path) -> Result<Option<csv::Reader<std::fs::File>>> {
    if !path.exists() {
        warn!("{} が無いので読み飛ばす", path.display());
        return Ok(None);
    }
    Ok(Some(
        csv::ReaderBuilder::new()
            .from_path(path)
            .with_context(|| format!("{} を開けない", path.display()))?,
    ))
}

/// 空文字は列が無いのと同じ扱いにする。
fn cell<'a>(record: &ColumnRecord<'a>, index: usize) -> Option<&'a str> {
    record.get(index).filter(|value| !value.is_empty())
}

/// 任意列の省略・並べ替えが他の列の意味を変えないよう、ヘッダーを一度だけ索引する。
fn column_indices(headers: &csv::StringRecord, names: &[&str]) -> Vec<Option<usize>> {
    names
        .iter()
        .map(|name| headers.iter().position(|h| h == *name))
        .collect()
}

struct ColumnRecord<'a> {
    record: &'a csv::StringRecord,
    columns: &'a [Option<usize>],
}

impl<'a> ColumnRecord<'a> {
    /// 正規化した列位置から元のレコードを参照し、省略された任意列はNoneにします。
    fn get(&self, index: usize) -> Option<&'a str> {
        self.columns
            .get(index)
            .copied()
            .flatten()
            .and_then(|i| self.record.get(i))
    }
}

/// 路線の各列と英語の路線名を読み込み、フィード内のIDを名前空間化します。
fn load_routes(data: &mut GtfsData, dir: &Path, feed: &GtfsFeed) -> Result<()> {
    let Some(mut rdr) = reader(&dir.join("routes.txt"))? else {
        return Ok(());
    };
    let city_bus = feed.id == NAGOYA_CITY_BUS.id;
    let hokkaido_express = feed.id == HOKKAIDO_EXPRESS_BUS.id;
    // 未登録の事業者があれば、このフィードの路線を一部だけ取り込まずに失敗させる。
    let mut express_routes = Vec::new();
    let field = if city_bus {
        "route_short_name"
    } else {
        "route_long_name"
    };
    let translations = load_field_translations(dir, "routes", field)?;
    // route_id,agency_id,route_short_name,route_long_name,route_desc,
    // route_type,route_url,route_color,route_text_color,jp_parent_route_id
    let columns = column_indices(
        rdr.headers()?,
        &[
            "route_id",
            "agency_id",
            "route_short_name",
            "route_long_name",
            "route_desc",
            "route_type",
            "route_url",
            "route_color",
            "route_text_color",
            "jp_parent_route_id",
        ],
    );
    for record in rdr.records() {
        let raw = record?;
        let record = ColumnRecord {
            record: &raw,
            columns: &columns,
        };
        let route_id = record.get(0).unwrap_or("");
        let translation = translations.get(route_id).or_else(|| {
            if city_bus {
                cell(&record, 2).and_then(|name| translations.get(name))
            } else {
                None
            }
        });
        let company_cd = if hokkaido_express {
            let agency_id = record.get(1).unwrap_or_default();
            Some(hokkaido_express_company_cd(agency_id).with_context(|| {
                format!("北海道高速バスの路線 {route_id} に未登録の agency_id {agency_id:?}")
            })?)
        } else {
            None
        };
        let route = Route {
            route_id: scoped_id(feed, route_id),
            company_cd,
            route_short_name: cell(&record, 2).map(str::to_string),
            route_long_name: cell(&record, 3).map(str::to_string),
            route_long_name_r: translation.and_then(|t| t.en.clone()),
            route_type: record.get(5).unwrap_or("3").parse().unwrap_or(3),
            route_color: cell(&record, 7).map(str::to_string),
        };
        if hokkaido_express {
            express_routes.push(route);
        } else {
            data.push_route(route);
        }
    }
    for route in express_routes {
        data.push_route(route);
    }
    Ok(())
}

/// 停留所の座標・親停留所・翻訳を読み込み、英語名が無ければ読みから補います。
fn load_stops(
    data: &mut GtfsData,
    dir: &Path,
    feed: &GtfsFeed,
    translations: &Translations,
) -> Result<()> {
    let Some(mut rdr) = reader(&dir.join("stops.txt"))? else {
        return Ok(());
    };
    // stop_id,stop_code,stop_name,stop_desc,stop_lat,stop_lon,zone_id,
    // stop_url,location_type,parent_station,...
    let columns = column_indices(
        rdr.headers()?,
        &[
            "stop_id",
            "stop_code",
            "stop_name",
            "stop_desc",
            "stop_lat",
            "stop_lon",
            "zone_id",
            "stop_url",
            "location_type",
            "parent_station",
            "platform_code",
        ],
    );
    for record in rdr.records() {
        let raw = record?;
        let record = ColumnRecord {
            record: &raw,
            columns: &columns,
        };
        let original_stop_id = record.get(0).unwrap_or("");
        let stop_name = record.get(2).unwrap_or("").to_string();

        // 訳語は record_id (= stop_id) 起点のフィードと field_value (= 日本語名)
        // 起点のフィードがあるので両方引く。
        let translation = translations
            .get(original_stop_id)
            .or_else(|| translations.get(&stop_name));

        // 一部のフィード (京王・東急コミュニティ) は ja-Hrkt を半角カタカナで
        // 配るため全角へ寄せる。
        let stop_name_k = translation
            .and_then(|t| t.ja_hrkt.clone())
            .map(|k| to_fullwidth_katakana(&k));
        // 公式の英語名を優先し、無ければ読みからヘボン式へ落とす。
        // ここを空のままにすると英語表示・英語検索の双方が欠ける。
        let stop_name_r = translation
            .and_then(|t| t.en.clone())
            .filter(|s| !s.is_empty())
            .or_else(|| stop_name_k.as_deref().and_then(romaji_display_name));

        data.push_stop(Stop {
            stop_id: scoped_id(feed, original_stop_id),
            stop_name,
            stop_name_k,
            stop_name_r,
            stop_name_zh: translation.and_then(|t| t.zh.clone()),
            stop_name_ko: translation.and_then(|t| t.ko.clone()),
            stop_lat: coordinate(&record, 4, original_stop_id, "stop_lat"),
            stop_lon: coordinate(&record, 5, original_stop_id, "stop_lon"),
            platform_code: cell(&record, 10)
                .map(str::trim)
                .filter(|code| !code.is_empty())
                .map(str::to_string),
            parent_station: cell(&record, 9).map(|s| scoped_id(feed, s)),
        });
    }
    Ok(())
}

/// 座標を読む。読めない値を黙って 0.0 にすると、ギニア湾の座標として
/// 距離計算やまとめ処理へ流れてしまうので警告を出す。
fn coordinate(record: &ColumnRecord<'_>, index: usize, stop_id: &str, column: &str) -> f64 {
    let raw = record.get(index).unwrap_or("");
    match raw.trim().parse() {
        Ok(value) => value,
        Err(_) => {
            warn!("停留所 {stop_id} の {column} を読めない (値: {raw:?})。0.0 として扱う");
            0.0
        }
    }
}

/// 便の路線・行先・方向・shapeを、フィードごとの列配置から読み込みます。
fn load_trips(
    data: &mut GtfsData,
    dir: &Path,
    feed: &GtfsFeed,
    scheduled_services: Option<&HashSet<String>>,
) -> Result<()> {
    let Some(mut rdr) = reader(&dir.join("trips.txt"))? else {
        return Ok(());
    };
    // route_id,service_id,trip_id,trip_headsign,trip_short_name,
    // direction_id,block_id,shape_id,...
    let columns = column_indices(
        rdr.headers()?,
        &[
            "route_id",
            "service_id",
            "trip_id",
            "trip_headsign",
            "trip_short_name",
            "direction_id",
            "block_id",
            "shape_id",
            "jp_pattern_id",
        ],
    );
    for record in rdr.records() {
        let raw = record?;
        let record = ColumnRecord {
            record: &raw,
            columns: &columns,
        };
        if scheduled_services
            .is_some_and(|services| !services.contains(record.get(1).unwrap_or("")))
        {
            continue;
        }
        // 名古屋市バスでは shape_id が空で、jp_pattern_id が停車順を区別する。
        let pattern = cell(&record, 7).or_else(|| {
            if feed.id == NAGOYA_CITY_BUS.id {
                cell(&record, 8)
            } else {
                None
            }
        });
        data.push_trip(Trip {
            trip_id: scoped_id(feed, record.get(2).unwrap_or("")),
            route_id: scoped_id(feed, record.get(0).unwrap_or("")),
            trip_headsign: cell(&record, 3).map(str::to_string),
            direction_id: cell(&record, 5).and_then(|v| v.parse().ok()),
            shape_id: pattern.map(|id| scoped_id(feed, id)),
        });
    }
    Ok(())
}

/// 各便の停車順と走行距離を読み込みます。距離列の省略時はNoneにします。
fn load_stop_times(data: &mut GtfsData, dir: &Path, feed: &GtfsFeed) -> Result<()> {
    let Some(mut rdr) = reader(&dir.join("stop_times.txt"))? else {
        return Ok(());
    };
    // trip_id,arrival_time,departure_time,stop_id,stop_sequence,stop_headsign,
    // pickup_type,drop_off_type,shape_dist_traveled,timepoint
    let columns = column_indices(
        rdr.headers()?,
        &[
            "trip_id",
            "arrival_time",
            "departure_time",
            "stop_id",
            "stop_sequence",
            "stop_headsign",
            "pickup_type",
            "drop_off_type",
            "shape_dist_traveled",
            "timepoint",
        ],
    );
    for record in rdr.records() {
        let raw = record?;
        let record = ColumnRecord {
            record: &raw,
            columns: &columns,
        };
        data.push_stop_time(StopTime {
            trip_id: scoped_id(feed, record.get(0).unwrap_or("")),
            stop_id: scoped_id(feed, record.get(3).unwrap_or("")),
            stop_sequence: record.get(4).unwrap_or("0").parse().unwrap_or(0),
            shape_dist_traveled: cell(&record, 8).and_then(|v| v.parse().ok()),
        });
    }
    Ok(())
}

/// `translations.txt` から stop_name の訳語だけを読む。
///
/// キーは `record_id` (= stop_id、末尾に "-01" のような柱番号が付くことがある) の
/// フィードと、`field_value` (= 日本語の停留所名) のフィードがある。どちらも
/// 引けるように両方を索引する。列数はフィードによって違うので列名で解決する。
pub fn load_translations(dir: &Path) -> Result<Translations> {
    load_field_translations(dir, "stops", "stop_name")
}

/// 指定されたテーブル・フィールドの訳語を、レコードIDと日本語名で索引します。
fn load_field_translations(dir: &Path, table: &str, field: &str) -> Result<Translations> {
    let mut translations = Translations::new();
    let path = dir.join("translations.txt");
    if !path.exists() {
        return Ok(translations);
    }

    let mut rdr = csv::ReaderBuilder::new().from_path(&path)?;
    let headers = rdr.headers()?.clone();
    let column = |name: &str| headers.iter().position(|h| h == name);
    let (Some(table_idx), Some(field_idx), Some(lang_idx), Some(text_idx)) = (
        column("table_name"),
        column("field_name"),
        column("language"),
        column("translation"),
    ) else {
        warn!("translations.txt に必要な列が無いため訳語を読み飛ばす");
        return Ok(translations);
    };
    let record_id_idx = column("record_id");
    let field_value_idx = column("field_value");

    for record in rdr.records() {
        let record = record?;
        if record.get(table_idx) != Some(table) || record.get(field_idx) != Some(field) {
            continue;
        }
        let language = record.get(lang_idx).unwrap_or("");
        let text = record.get(text_idx).unwrap_or("");
        let at = |idx: Option<usize>| idx.and_then(|i| record.get(i)).unwrap_or("");
        let record_id = at(record_id_idx);
        let field_value = at(field_value_idx);

        if !record_id.is_empty() {
            set_language(&mut translations, record_id, language, text, true);
            // 柱番号を落とした親 stop_id でも引けるようにする。先に来た子が勝つ。
            if let Some(parent) = record_id.rfind('-').map(|pos| &record_id[..pos]) {
                set_language(&mut translations, parent, language, text, false);
            }
        }
        if !field_value.is_empty() {
            set_language(&mut translations, field_value, language, text, true);
        }
    }

    Ok(translations)
}

/// 対応言語の訳語を設定し、親IDへの補完時は既存の訳語を保持します。
fn set_language(
    translations: &mut Translations,
    key: &str,
    language: &str,
    text: &str,
    overwrite: bool,
) {
    let entry = translations.entry(key.to_string()).or_default();
    let slot = match language {
        "ja-Hrkt" => &mut entry.ja_hrkt,
        "en" => &mut entry.en,
        "zh-Hans" | "zh-Hant" | "zh" => &mut entry.zh,
        "ko" => &mut entry.ko,
        _ => return,
    };
    if overwrite || slot.is_none() {
        *slot = Some(text.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn express_routes_map_each_agency_and_reject_an_unmapped_feed() {
        let dir = std::env::temp_dir().join(format!(
            "stationapi_pp_express_routes_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let routes = dir.join("routes.txt");
        let known = "route_id,agency_id,route_long_name,route_type\n\
                     chuo,4430001050451,札幌～函館,3\n\
                     dohoku,8450001002208,札幌～旭川,3\n";
        std::fs::write(&routes, known).unwrap();
        let mut valid = GtfsData::default();
        load_routes(&mut valid, &dir, &HOKKAIDO_EXPRESS_BUS).unwrap();
        assert_eq!(valid.routes[0].company_cd, Some(258));
        assert_eq!(valid.routes[1].company_cd, Some(261));

        std::fs::write(&routes, format!("{known}unknown,0,未登録,3\n")).unwrap();
        let mut invalid = GtfsData::default();
        let result = load_routes(&mut invalid, &dir, &HOKKAIDO_EXPRESS_BUS);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(result.unwrap_err().to_string().contains("agency_id"));
        assert!(invalid.routes.is_empty());
    }

    #[test]
    fn derived_patterns_follow_stop_sequence_and_reuse_identical_orders() {
        let mut trips: Vec<_> = ["a", "b", "reverse"]
            .into_iter()
            .map(|id| Trip {
                trip_id: id.to_string(),
                route_id: "route".to_string(),
                trip_headsign: None,
                direction_id: None,
                shape_id: None,
            })
            .collect();
        let stop_times: Vec<_> = [
            ("a", "first", 1),
            ("a", "last", 2),
            ("b", "last", 20),
            ("b", "first", 10),
            ("reverse", "last", 1),
            ("reverse", "first", 2),
        ]
        .into_iter()
        .map(|(trip, stop, sequence)| StopTime {
            trip_id: trip.to_string(),
            stop_id: stop.to_string(),
            stop_sequence: sequence,
            shape_dist_traveled: None,
        })
        .collect();

        derive_stop_order_patterns(&mut trips, &stop_times);

        assert!(trips[0].shape_id.is_some());
        assert_eq!(trips[0].shape_id, trips[1].shape_id);
        assert_ne!(trips[0].shape_id, trips[2].shape_id);
    }

    #[test]
    fn nagoya_city_bus_keeps_scheduled_patterns_and_their_stop_order() {
        use crate::gtfs::integrate;
        use crate::rail::Dataset;

        let feed = GtfsFeed {
            path: concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/nagoya_city_bus"),
            ..NAGOYA_CITY_BUS
        };
        let mut gtfs = GtfsData::default();
        load_feed(&mut gtfs, &feed).unwrap();
        assert_eq!(gtfs.trips.len(), 3);
        assert!(gtfs
            .trips
            .iter()
            .all(|trip| trip.route_id != "nagoya_city_bus:6201"));
        let outbound = gtfs
            .trips
            .iter()
            .find(|trip| trip.shape_id.as_deref() == Some("nagoya_city_bus:6303031"))
            .unwrap();
        let times = gtfs.stop_times_by_trip();
        let stops = &times[outbound.trip_id.as_str()];
        assert_eq!(stops.len(), 27);
        assert_eq!(stops.first().unwrap().stop_id, "nagoya_city_bus:050600G1");
        assert_eq!(stops.last().unwrap().stop_id, "nagoya_city_bus:15033901");

        let mut dataset =
            Dataset::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../data")).unwrap();
        let line_count = dataset.lines.len();
        let type_count = dataset.types.len();
        let station_count = dataset.stations.len();
        integrate::integrate(&mut dataset, &gtfs).unwrap();
        let company = dataset.lines.col("company_cd");
        let line_name = dataset.lines.col("line_name");
        let english_name = dataset.lines.col("line_name_r");
        let lines: Vec<_> = dataset.lines.rows()[line_count..]
            .iter()
            .map(|row| {
                (
                    row[company].as_deref(),
                    row[line_name].as_deref(),
                    row[english_name].as_deref(),
                )
            })
            .collect();
        assert_eq!(
            lines,
            [
                (Some("257"), Some("Ｃ－７５８"), Some("C-758")),
                (
                    Some("257"),
                    Some("ゆとりーとライン"),
                    Some("ゆとりーとライン")
                ),
            ]
        );
        assert_eq!(dataset.types.len() - type_count, 3);
        let type_name = dataset.types.col("type_name");
        assert!(dataset.types.rows()[type_count..]
            .iter()
            .any(|row| row[type_name]
                .as_deref()
                .is_some_and(|name| name.ends_with("(循環)"))));
        let prefecture = dataset.stations.col("pref_cd");
        let stop_name = dataset.stations.col("station_name");
        let stop_name_r = dataset.stations.col("station_name_r");
        let platform_code = dataset.stations.col("platform_code");
        let nagoya_platforms: HashSet<_> = dataset.stations.rows()[station_count..]
            .iter()
            .filter(|row| row[stop_name].as_deref() == Some("名古屋駅"))
            .filter_map(|row| row[platform_code].as_deref())
            .collect();
        assert!(nagoya_platforms.contains("9"));
        assert!(nagoya_platforms.contains("21"));
        assert!(dataset.stations.rows()[station_count..]
            .iter()
            .all(|row| row[prefecture].as_deref() == Some("23")));
        assert!(dataset.stations.rows()[station_count..]
            .iter()
            .any(|row| row[stop_name].as_deref() == Some("大曽根")
                && row[stop_name_r].as_deref() == Some("Ozone")));
    }

    #[test]
    fn nagoya_srt_feed_preserves_columns_translations_and_loop_patterns() {
        use crate::gtfs::{feed::GTFS_FEEDS, integrate};
        use crate::rail::Dataset;

        let feed = GtfsFeed {
            path: concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/nagoya_srt"),
            ..*GTFS_FEEDS
                .iter()
                .find(|feed| feed.id == "nagoya_srt")
                .unwrap()
        };
        let mut gtfs = GtfsData::default();
        load_feed(&mut gtfs, &feed).unwrap();
        assert_eq!(gtfs.routes.len(), 2);
        assert_eq!(gtfs.stops.len(), 11);
        assert_eq!(gtfs.trips.len(), 20);
        for (shape, headsign) in [
            ("101", "名駅－栄ルート"),
            ("102", "栄"),
            ("103", "名駅－栄ルート"),
            ("104", "名古屋駅"),
            ("201", "名駅－名城ルート"),
            ("202", "名駅－名城ルート"),
        ] {
            assert!(gtfs
                .trips
                .iter()
                .filter(
                    |trip| trip.shape_id.as_deref() == Some(format!("nagoya_srt:{shape}").as_str())
                )
                .all(|trip| trip.trip_headsign.as_deref() == Some(headsign)));
        }
        assert_eq!(gtfs.routes[0].route_color.as_deref(), Some("E26B0A"));
        assert_eq!(gtfs.routes[1].route_color.as_deref(), Some("FFFF00"));
        assert_eq!(
            gtfs.routes[0].route_long_name_r.as_deref(),
            Some("Nagoya Sta. - Sakae Route")
        );
        assert!(gtfs.stops.iter().all(|stop| stop.parent_station.is_none()));
        let castle = gtfs
            .stops
            .iter()
            .find(|stop| stop.stop_id == "nagoya_srt:20_01")
            .unwrap();
        assert_eq!(castle.stop_name_r.as_deref(), Some("Nagoya Castle"));
        assert_eq!(castle.stop_name_k.as_deref(), Some("ナゴヤジョウ"));
        assert_eq!(castle.stop_name_ko.as_deref(), Some("나고야성"));
        assert_eq!(castle.stop_lat, 35.181405);
        assert!(gtfs.trips.iter().all(|trip| trip.direction_id == Some(0)));
        assert!(gtfs.trips.iter().all(|trip| trip
            .shape_id
            .as_deref()
            .is_some_and(|id| id.starts_with("nagoya_srt:"))));
        // timepoint=1 is not a distance: the optional shape_dist_traveled column is absent.
        assert!(gtfs
            .stop_times
            .iter()
            .all(|stop| stop.shape_dist_traveled.is_none()));

        let mut dataset = Dataset::load(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../data")
                .as_path(),
        )
        .unwrap();
        let line_count = dataset.lines.len();
        let station_count = dataset.stations.len();
        let type_count = dataset.types.len();
        let sst_count = dataset.sst.len();
        integrate::integrate(&mut dataset, &gtfs).unwrap();
        assert_eq!(dataset.lines.len() - line_count, 2);
        assert_eq!(dataset.stations.len() - station_count, 12);
        let prefecture = dataset.stations.col("pref_cd");
        assert!(dataset.stations.rows()[station_count..]
            .iter()
            .all(|row| row[prefecture].as_deref() == Some("23")));
        assert_eq!(dataset.types.len() - type_count, 6);
        let company = dataset.lines.col("company_cd");
        let color = dataset.lines.col("line_color_c");
        let transport = dataset.lines.col("transport_type");
        let lines = &dataset.lines.rows()[line_count..];
        assert!(lines
            .iter()
            .all(|row| row[company].as_deref() == Some("256")
                && row[transport].as_deref() == Some("1")));
        assert_eq!(lines[0][color].as_deref(), Some("#E26B0A"));
        assert_eq!(lines[1][color].as_deref(), Some("#FFFF00"));
        let station_id = dataset.stations.col("station_cd");
        let known: std::collections::HashSet<_> = dataset.stations.rows()[station_count..]
            .iter()
            .map(|row| row[station_id].clone())
            .collect();
        let sst_station = dataset.sst.col("station_cd");
        assert!(dataset.sst.rows()[sst_count..]
            .iter()
            .all(|row| known.contains(&row[sst_station])));
        let station_name = dataset.stations.col("station_name");
        let names: std::collections::HashMap<_, _> = dataset.stations.rows()[station_count..]
            .iter()
            .map(|row| {
                (
                    row[station_id].clone(),
                    row[station_name].as_deref().unwrap(),
                )
            })
            .collect();
        let group = dataset.sst.col("line_group_cd");
        let mut patterns: std::collections::HashMap<_, Vec<&str>> =
            std::collections::HashMap::new();
        for row in &dataset.sst.rows()[sst_count..] {
            patterns
                .entry(row[group].clone())
                .or_default()
                .push(names[&row[sst_station]]);
        }
        let mut actual: Vec<_> = patterns.into_values().collect();
        actual.sort();
        let mut expected = vec![
            vec![
                "栄",
                "広小路本町",
                "納屋橋",
                "柳橋中央市場",
                "名古屋駅桜通",
                "名古屋駅",
                "柳橋",
                "広小路本町",
            ],
            vec![
                "栄",
                "広小路本町",
                "納屋橋",
                "名古屋駅桜通",
                "名古屋駅",
                "柳橋",
                "広小路本町",
            ],
            vec!["名古屋駅桜通", "名古屋駅", "柳橋", "広小路本町", "栄"],
            vec![
                "栄",
                "広小路本町",
                "納屋橋",
                "柳橋中央市場",
                "名古屋駅桜通",
                "名古屋駅",
            ],
            vec!["名古屋駅太閤通口", "円頓寺", "名古屋城"],
            vec!["名古屋駅太閤通口", "柳橋中央市場", "円頓寺", "名古屋城"],
        ];
        expected.sort();
        assert_eq!(actual, expected);
    }

    fn write_translations(name: &str, contents: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("translations.txt"), contents).unwrap();
        dir
    }

    #[test]
    fn field_value_keyed_translations_resolve_by_stop_name() {
        // 京王・東急コミュニティは record_id を空にして field_value (日本語名) で引かせる。
        // 列は 7 列で record_sub_id を含み、ja-Hrkt は半角カタカナで来る。
        let dir = write_translations(
            "stationapi_pp_translations_fv",
            "table_name,field_name,language,translation,record_id,record_sub_id,field_value\n\
             stops,stop_name,ja,西八王子駅南口,,,西八王子駅南口\n\
             stops,stop_name,ja-Hrkt,ﾆｼﾊﾁｵｳｼﾞｴｷﾐﾅﾐｸﾞﾁ,,,西八王子駅南口\n\
             stops,stop_name,en,Nishi-hachioji Sta. South,,,西八王子駅南口\n",
        );
        let translations = load_translations(&dir).unwrap();
        let t = translations
            .get("西八王子駅南口")
            .expect("停留所名で引ける");
        assert_eq!(t.ja_hrkt.as_deref(), Some("ﾆｼﾊﾁｵｳｼﾞｴｷﾐﾅﾐｸﾞﾁ"));
        assert_eq!(t.en.as_deref(), Some("Nishi-hachioji Sta. South"));
        assert_eq!(
            to_fullwidth_katakana(t.ja_hrkt.as_deref().unwrap()),
            "ニシハチオウジエキミナミグチ"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_id_keyed_translations_resolve_by_stop_id_and_parent() {
        // 西武は record_id (stop_id + 柱番号) で引かせる 6 列構成。
        let dir = write_translations(
            "stationapi_pp_translations_rid",
            "table_name,field_name,language,translation,record_id,field_value\n\
             stops,stop_name,ja,西武百貨店前,20001-01,\n\
             stops,stop_name,en,Seibu hyakkaten-mae,20001-01,\n",
        );
        let translations = load_translations(&dir).unwrap();
        assert_eq!(
            translations.get("20001-01").unwrap().en.as_deref(),
            Some("Seibu hyakkaten-mae")
        );
        // 柱番号を落とした親でも引ける。
        assert!(translations.contains_key("20001"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
