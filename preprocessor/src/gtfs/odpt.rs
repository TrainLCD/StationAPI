//! 東急バスの ODPT JSON を GTFS 相当へ変換して取り込む。
//!
//! 東急バスは GTFS ZIP ではなく ODPT の JSON API で配信されている
//! (コミュニティバス 3 区だけは GTFS でも配られており、そちらは重複するので外す)。
//! 応答が 130MB 超あるため、取得したものはキャッシュして 7 日間使い回す。

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Result};
use serde::de::{DeserializeOwned, DeserializeSeed, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use stationapi::domain::romaji::romaji_display_name;

use super::model::{GtfsData, Route, Stop, StopTime, Trip};
use crate::codes::hiragana_to_katakana;
use crate::{info, warn};

/// ODPT 由来の ID に付ける前置。GTFS フィードの ID と衝突させない。
const PREFIX: &str = "tokyu_json";
const OPERATOR: &str = "odpt.Operator:TokyuBus";
const CACHE_DIR: &str = "data/TokyuBus-ODPT";
const CACHE_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// ODPT JSON には路線色が無いため、東急バスの色を決め打ちで入れる。
const ROUTE_COLOR: &str = "DD1133";

#[derive(Debug, Deserialize, Serialize)]
struct BusroutePattern {
    #[serde(rename = "owl:sameAs")]
    same_as: String,
    #[serde(rename = "dc:title", default)]
    title: String,
    #[serde(rename = "odpt:operator")]
    operator: String,
    #[serde(rename = "odpt:busroute")]
    busroute: String,
    #[serde(rename = "odpt:direction")]
    direction: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct BusstopPole {
    #[serde(rename = "owl:sameAs")]
    same_as: String,
    #[serde(rename = "dc:title", default)]
    title: String,
    #[serde(rename = "odpt:kana", default)]
    kana: String,
    #[serde(rename = "odpt:operator", default)]
    operators: Vec<String>,
    #[serde(rename = "geo:lat", default)]
    lat: f64,
    #[serde(rename = "geo:long", default)]
    lon: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct BusTimetable {
    #[serde(rename = "owl:sameAs")]
    same_as: String,
    #[serde(rename = "odpt:operator")]
    operator: String,
    #[serde(rename = "odpt:busroutePattern")]
    busroute_pattern: String,
    #[serde(rename = "odpt:busTimetableObject", default)]
    objects: Vec<BusTimetableObject>,
}

#[derive(Debug, Deserialize, Serialize)]
struct BusTimetableObject {
    #[serde(rename = "odpt:index")]
    index: i32,
    #[serde(rename = "odpt:busstopPole")]
    busstop_pole: String,
    #[serde(rename = "odpt:arrivalTime")]
    arrival_time: Option<String>,
    #[serde(rename = "odpt:departureTime")]
    departure_time: Option<String>,
    #[serde(rename = "odpt:destinationSign")]
    destination_sign: Option<String>,
}

pub struct OdptData {
    patterns: Vec<BusroutePattern>,
    stops: Vec<BusstopPole>,
    timetables: Vec<BusTimetable>,
}

/// 東急バス以外のレコードを読み飛ばしながら配列を読む。
///
/// `BusTimetable` は全事業者ぶんで 130MB を超えるので、`Vec<Value>` に
/// 一度受けてから絞ると無駄が大きい。ストリームのまま選別する。
struct TokyuOnlySeed<T>(PhantomData<T>);

impl<'de, T: DeserializeOwned> DeserializeSeed<'de> for TokyuOnlySeed<T> {
    type Value = Vec<T>;

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Vec<T>, D::Error> {
        deserializer.deserialize_seq(TokyuOnlyVisitor(PhantomData))
    }
}

struct TokyuOnlyVisitor<T>(PhantomData<T>);

impl<'de, T: DeserializeOwned> Visitor<'de> for TokyuOnlyVisitor<T> {
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ODPT の JSON 配列")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Vec<T>, A::Error> {
        let mut selected = Vec::new();
        while let Some(value) = sequence.next_element::<serde_json::Value>()? {
            let is_tokyu = match value.get("odpt:operator") {
                Some(serde_json::Value::String(operator)) => operator == OPERATOR,
                Some(serde_json::Value::Array(operators)) => operators
                    .iter()
                    .any(|operator| operator.as_str() == Some(OPERATOR)),
                _ => false,
            };
            if is_tokyu {
                selected.push(serde_json::from_value(value).map_err(serde::de::Error::custom)?);
            }
        }
        Ok(selected)
    }
}

fn read_items<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    let mut deserializer = serde_json::Deserializer::from_reader(BufReader::new(File::open(path)?));
    Ok(TokyuOnlySeed(PhantomData).deserialize(&mut deserializer)?)
}

fn strip_prefix(value: &str) -> &str {
    value.split_once(':').map_or(value, |(_, id)| id)
}

fn scoped(value: &str) -> String {
    format!("{PREFIX}:{}", strip_prefix(value))
}

/// ODPT の direction を GTFS の direction_id へ寄せる。
fn direction_id(direction: Option<&str>) -> Option<i32> {
    match direction {
        Some("1") => Some(0),
        Some("2") => Some(1),
        _ => None,
    }
}

/// `dc:title` は「渋41 渋谷駅前行」のような表記なので、先頭の系統名だけ採る。
fn route_name(title: &str) -> &str {
    title.split_whitespace().next().unwrap_or(title)
}

/// コミュニティバス 3 系統は GTFS フィード側でも配信されるため、
/// JSON 側からは取り込まない。
fn is_community_route(busroute: &str) -> bool {
    matches!(
        strip_prefix(busroute),
        "TokyuBus.Tamachan" | "TokyuBus.Shinabasu" | "TokyuBus.Sanma"
    )
}

/// GTFS の HH:MM:SS。24 時を超える表記もそのまま通す。
fn parse_gtfs_time(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let parts: Vec<&str> = value.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    parts[0].parse::<u32>().ok()?;
    parts[1].parse::<u32>().ok()?;
    parts[2].parse::<u32>().ok()?;
    Some(value.to_string())
}

/// ODPT は HH:MM なので秒を補う。
fn parse_odpt_time(value: &str) -> Option<String> {
    if let Some(time) = parse_gtfs_time(value) {
        return Some(time);
    }
    let (hours, minutes) = value.split_once(':')?;
    let hours: u32 = hours.parse().ok()?;
    let minutes: u32 = minutes.parse().ok()?;
    if minutes >= 60 {
        return None;
    }
    Some(format!("{hours:02}:{minutes:02}:00"))
}

/// 資源を取得する。7 日以内のキャッシュがあればそれを使う。
///
/// トークンが無いときはキャッシュだけを頼りにする。ここで諦めてしまうと
/// トークン未設定の環境で東急バスのデータが丸ごと欠ける。
fn download_resource<T: DeserializeOwned + Serialize>(
    client: &reqwest::blocking::Client,
    resource: &str,
    token: Option<&str>,
) -> Result<Vec<T>> {
    let cache_dir = PathBuf::from(CACHE_DIR);
    let cache_path = cache_dir.join(format!(
        "{}.tokyu.json",
        resource.rsplit(':').next().unwrap_or(resource)
    ));

    if let Ok(metadata) = fs::metadata(&cache_path) {
        let fresh = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < CACHE_MAX_AGE);
        if fresh {
            info!("東急バス {resource} はキャッシュを使う");
            return read_items(&cache_path);
        }
    }

    let Some(token) = token else {
        bail!("東急バス {resource} の取得には ODPT_ACCESS_TOKEN が要る (使えるキャッシュも無い)");
    };

    let url = format!("https://api.odpt.org/api/v4/{resource}.json");
    let mut response = client
        .get(url)
        .query(&[("odpt:operator", OPERATOR), ("acl:consumerKey", token)])
        .send()
        .map_err(|e| e.without_url())?;
    if !response.status().is_success() {
        bail!("{resource} の取得に失敗: HTTP {}", response.status());
    }

    fs::create_dir_all(&cache_dir)?;
    // 応答をいったんファイルへ落としてから選別する。メモリに載せると 130MB 超になる。
    let download_path = cache_path.with_extension("download.tmp");
    let mut download = BufWriter::new(File::create(&download_path)?);
    std::io::copy(&mut response, &mut download)?;
    download.flush()?;
    drop(download);

    let selected = match read_items(&download_path) {
        Ok(selected) => selected,
        Err(e) => {
            let _ = fs::remove_file(&download_path);
            return Err(e);
        }
    };
    fs::remove_file(&download_path)?;

    // 選別後のものだけをキャッシュへ残す。書き換えは一時ファイル経由。
    let temporary = cache_path.with_extension("json.tmp");
    let mut cache = BufWriter::new(File::create(&temporary)?);
    serde_json::to_writer(&mut cache, &selected)?;
    cache.flush()?;
    drop(cache);
    if cache_path.exists() {
        fs::remove_file(&cache_path)?;
    }
    fs::rename(temporary, &cache_path)?;
    Ok(selected)
}

pub fn download() -> Result<OdptData> {
    let token = std::env::var("ODPT_ACCESS_TOKEN").ok();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;
    Ok(OdptData {
        patterns: download_resource(&client, "odpt:BusroutePattern", token.as_deref())?,
        stops: download_resource(&client, "odpt:BusstopPole", token.as_deref())?,
        timetables: download_resource(&client, "odpt:BusTimetable", token.as_deref())?,
    })
}

/// 取得した JSON を GTFS 相当へ変換して `data` へ足す。
pub fn load(data: &mut GtfsData, odpt: &OdptData) {
    // 運行パターン -> (系統, 方向)。時刻表から系統を引くのに使う。
    let mut pattern_map: HashMap<String, (String, Option<i32>)> = HashMap::new();
    let mut route_count = 0usize;
    for pattern in &odpt.patterns {
        if pattern.operator != OPERATOR || is_community_route(&pattern.busroute) {
            continue;
        }
        let route_id = scoped(&pattern.busroute);
        pattern_map.insert(
            scoped(&pattern.same_as),
            (route_id.clone(), direction_id(pattern.direction.as_deref())),
        );

        let name = route_name(&pattern.title).to_string();
        let before = data.routes.len();
        data.push_route(Route {
            route_id,
            route_short_name: Some(name.clone()),
            route_long_name: Some(name),
            route_long_name_r: None,
            route_type: 3,
            route_color: Some(ROUTE_COLOR.to_string()),
        });
        if data.routes.len() > before {
            route_count += 1;
        }
    }

    let mut stop_count = 0usize;
    let mut missing_coordinates = 0usize;
    for stop in &odpt.stops {
        if !stop.operators.iter().any(|operator| operator == OPERATOR) {
            continue;
        }
        if stop.lat == 0.0 || stop.lon == 0.0 {
            missing_coordinates += 1;
        }
        // JSON には英語名が無いので、読みからヘボン式を起こす。
        // GTFS 側の `en` が無いときの扱いと揃えてある。
        let katakana = hiragana_to_katakana(&stop.kana);
        data.push_stop(Stop {
            stop_id: scoped(&stop.same_as),
            stop_name: stop.title.clone(),
            stop_name_k: Some(katakana.clone()),
            stop_name_r: romaji_display_name(&katakana),
            stop_name_zh: None,
            stop_name_ko: None,
            stop_lat: stop.lat,
            stop_lon: stop.lon,
            parent_station: None,
        });
        stop_count += 1;
    }
    if missing_coordinates > 0 {
        warn!(
            "東急バスの ODPT JSON に座標を持たない停留所が {missing_coordinates} 件ある。\
             名前検索は効くが座標検索では出ない"
        );
    }

    let mut trip_count = 0usize;
    let mut stop_time_count = 0usize;
    for timetable in &odpt.timetables {
        if timetable.operator != OPERATOR {
            continue;
        }
        let Some((route_id, direction)) = pattern_map.get(&scoped(&timetable.busroute_pattern))
        else {
            continue;
        };
        let trip_id = scoped(&timetable.same_as);
        // 運行パターンを shape_id に見立てる。系統の運行パターンごとに
        // 1 つの TrainType を作る仕組みをそのまま使うため。
        let pattern_id = scoped(&timetable.busroute_pattern);
        data.push_trip(Trip {
            trip_id: trip_id.clone(),
            route_id: route_id.clone(),
            trip_headsign: timetable
                .objects
                .iter()
                .find_map(|object| object.destination_sign.clone()),
            direction_id: *direction,
            shape_id: Some(pattern_id),
        });
        trip_count += 1;

        for object in &timetable.objects {
            let stop_id = scoped(&object.busstop_pole);
            if !data.has_stop(&stop_id) {
                continue;
            }
            // 到着・出発のどちらも読めない行は時刻表として意味がないので落とす。
            let arrival = object
                .arrival_time
                .as_deref()
                .or(object.departure_time.as_deref())
                .and_then(parse_odpt_time);
            let departure = object
                .departure_time
                .as_deref()
                .or(object.arrival_time.as_deref())
                .and_then(parse_odpt_time);
            if arrival.is_none() && departure.is_none() {
                continue;
            }
            data.push_stop_time(StopTime {
                trip_id: trip_id.clone(),
                stop_id,
                stop_sequence: object.index,
                shape_dist_traveled: None,
            });
            stop_time_count += 1;
        }
    }

    info!(
        "東急バス ODPT JSON: 系統 {route_count} / 停留所 {stop_count} / 便 {trip_count} / 停車 {stop_time_count}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odpt_time_gains_seconds() {
        assert_eq!(parse_odpt_time("08:30").as_deref(), Some("08:30:00"));
        assert_eq!(parse_odpt_time("8:03").as_deref(), Some("08:03:00"));
        assert_eq!(parse_odpt_time("25:30:00").as_deref(), Some("25:30:00"));
        assert_eq!(parse_odpt_time("08:60"), None);
        assert_eq!(parse_odpt_time("invalid"), None);
    }

    #[test]
    fn gtfs_time_passes_through_past_midnight() {
        assert_eq!(parse_gtfs_time("25:30:00").as_deref(), Some("25:30:00"));
        assert_eq!(parse_gtfs_time("08:30"), None);
        assert_eq!(parse_gtfs_time(""), None);
    }

    #[test]
    fn ids_are_scoped_after_stripping_the_odpt_prefix() {
        assert_eq!(
            scoped("odpt.BusroutePattern:TokyuBus.Shibu41.1"),
            "tokyu_json:TokyuBus.Shibu41.1"
        );
        assert_eq!(route_name("渋41 渋谷駅前行"), "渋41");
        assert!(is_community_route("odpt.Busroute:TokyuBus.Tamachan"));
        assert!(!is_community_route("odpt.Busroute:TokyuBus.Shibu41"));
    }

    #[test]
    fn direction_maps_to_gtfs_values() {
        assert_eq!(direction_id(Some("1")), Some(0));
        assert_eq!(direction_id(Some("2")), Some(1));
        assert_eq!(direction_id(None), None);
    }
}
