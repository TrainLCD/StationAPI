//! GTFS のテキストファイルを読む。
//!
//! 列は位置で読む。GTFS-JP は列順が仕様で決まっており、既存の取り込みも
//! 位置で読んでいたため、同じ解釈を保つ。`translations.txt` だけは
//! フィードごとに列数が違うので列名で引く。

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use stationapi::domain::romaji::{romaji_display_name, to_fullwidth_katakana};

use super::feed::{scoped_id, GtfsFeed};
use super::model::{GtfsData, Route, Stop, StopTime, Trip};
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

    let translations = load_translations(dir)?;
    load_routes(data, dir, feed)?;
    load_stops(data, dir, feed, &translations)?;
    load_trips(data, dir, feed)?;
    load_stop_times(data, dir, feed)?;
    Ok(())
}

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
fn cell(record: &csv::StringRecord, index: usize) -> Option<&str> {
    record.get(index).filter(|value| !value.is_empty())
}

fn load_routes(data: &mut GtfsData, dir: &Path, feed: &GtfsFeed) -> Result<()> {
    let Some(mut rdr) = reader(&dir.join("routes.txt"))? else {
        return Ok(());
    };
    // route_id,agency_id,route_short_name,route_long_name,route_desc,
    // route_type,route_url,route_color,route_text_color,jp_parent_route_id
    for record in rdr.records() {
        let record = record?;
        data.push_route(Route {
            route_id: scoped_id(feed, record.get(0).unwrap_or("")),
            route_short_name: cell(&record, 2).map(str::to_string),
            route_long_name: cell(&record, 3).map(str::to_string),
            route_long_name_r: None,
            route_type: record.get(5).unwrap_or("3").parse().unwrap_or(3),
            route_color: cell(&record, 7).map(str::to_string),
        });
    }
    Ok(())
}

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
    for record in rdr.records() {
        let record = record?;
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
            stop_lat: record.get(4).unwrap_or("0").parse().unwrap_or(0.0),
            stop_lon: record.get(5).unwrap_or("0").parse().unwrap_or(0.0),
            parent_station: cell(&record, 9).map(|s| scoped_id(feed, s)),
        });
    }
    Ok(())
}

fn load_trips(data: &mut GtfsData, dir: &Path, feed: &GtfsFeed) -> Result<()> {
    let Some(mut rdr) = reader(&dir.join("trips.txt"))? else {
        return Ok(());
    };
    // route_id,service_id,trip_id,trip_headsign,trip_short_name,
    // direction_id,block_id,shape_id,...
    for record in rdr.records() {
        let record = record?;
        data.push_trip(Trip {
            trip_id: scoped_id(feed, record.get(2).unwrap_or("")),
            route_id: scoped_id(feed, record.get(0).unwrap_or("")),
            trip_headsign: cell(&record, 3).map(str::to_string),
            direction_id: cell(&record, 5).and_then(|v| v.parse().ok()),
            shape_id: cell(&record, 7).map(|s| scoped_id(feed, s)),
        });
    }
    Ok(())
}

fn load_stop_times(data: &mut GtfsData, dir: &Path, feed: &GtfsFeed) -> Result<()> {
    let Some(mut rdr) = reader(&dir.join("stop_times.txt"))? else {
        return Ok(());
    };
    // trip_id,arrival_time,departure_time,stop_id,stop_sequence,stop_headsign,
    // pickup_type,drop_off_type,shape_dist_traveled,timepoint
    for record in rdr.records() {
        let record = record?;
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
        if record.get(table_idx) != Some("stops") || record.get(field_idx) != Some("stop_name") {
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
