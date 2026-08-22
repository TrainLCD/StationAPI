//! CSV をバイナリに埋め込み、isolate 起動時に一度だけパースしてメモリに保持する。
//! PostgreSQL の `point(lat,lon) <-> point($1,$2)` によるソートを全件走査で置き換える。

use stationapi::domain::normalize::normalize_for_search;
use std::collections::HashMap;
use std::sync::OnceLock;

/// 鉄道駅データ。ビルド時にバイナリへ埋め込まれる (raw 2.2MB / gzip 684KB)。
const STATIONS_CSV: &str = include_str!("../../data/3!stations.csv");
/// 路線データ。駅の所属路線を JOIN 相当で絞り込むために使う。
const LINES_CSV: &str = include_str!("../../data/2!lines.csv");

/// 検索と応答生成に必要な列だけを持つ軽量レコード。
/// Station エンティティ (66 フィールド) はレスポンス生成時にのみ組み立てる。
#[allow(dead_code)] // line_cd は路線付与を実装する次段階で使う
pub struct StationRecord {
    pub station_cd: u32,
    pub station_g_cd: u32,
    pub name: String,
    pub name_katakana: String,
    pub name_roman: Option<String>,
    /// マクロンを含まない正規化ローマ字 (station_name_rn)。ILIKE 検索の対象。
    pub name_roman_normalized: Option<String>,
    pub name_chinese: Option<String>,
    pub name_korean: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: u32,
    pub pref_cd: u32,
    pub postal_code: String,
    pub address: String,
    pub lat: f64,
    pub lon: f64,
    pub opened_at: String,
    pub closed_at: String,
    pub e_status: i32,
}

static INDEX: OnceLock<Vec<StationRecord>> = OnceLock::new();
static LINE_STATUS: OnceLock<HashMap<u32, i32>> = OnceLock::new();

fn line_status() -> &'static HashMap<u32, i32> {
    LINE_STATUS.get_or_init(build_line_status)
}

fn build_line_status() -> HashMap<u32, i32> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(LINES_CSV.as_bytes());
    let headers = match reader.headers() {
        Ok(h) => h.clone(),
        Err(_) => return HashMap::new(),
    };
    let col = |name: &str| headers.iter().position(|h| h.trim() == name);
    let (Some(c_cd), Some(c_status)) = (col("line_cd"), col("e_status")) else {
        return HashMap::new();
    };

    let mut map = HashMap::with_capacity(1024);
    for record in reader.records().flatten() {
        let (Some(cd), Some(status)) = (
            record.get(c_cd).and_then(|v| v.trim().parse::<u32>().ok()),
            record.get(c_status).and_then(|v| v.trim().parse::<i32>().ok()),
        ) else {
            continue;
        };
        map.insert(cd, status);
    }
    map
}

/// `stations JOIN lines ON s.line_cd = l.line_cd` (INNER) 相当。
/// lines に無い line_cd の駅は既存 SQL でも結果に出ない。座標検索がこの条件。
fn joins_line(record: &StationRecord) -> bool {
    line_status().contains_key(&record.line_cd)
}

/// 名前検索の JOIN は `AND l.e_status = 0` を追加で要求する。
/// これを見ないと廃止路線 (例: 成田エクスプレス, e_status=3) の駅が混ざる。
fn joins_active_line(record: &StationRecord) -> bool {
    line_status().get(&record.line_cd) == Some(&0)
}

pub fn stations() -> &'static [StationRecord] {
    INDEX.get_or_init(build)
}

/// 列順の変更に耐えるようヘッダ名から位置を引く。
fn build() -> Vec<StationRecord> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(STATIONS_CSV.as_bytes());

    let headers = match reader.headers() {
        Ok(h) => h.clone(),
        Err(_) => return Vec::new(),
    };
    let col = |name: &str| headers.iter().position(|h| h.trim() == name);

    let (Some(c_cd), Some(c_gcd), Some(c_name), Some(c_kana), Some(c_line), Some(c_lat), Some(c_lon)) = (
        col("station_cd"),
        col("station_g_cd"),
        col("station_name"),
        col("station_name_k"),
        col("line_cd"),
        col("lat"),
        col("lon"),
    ) else {
        return Vec::new();
    };
    let c_roman = col("station_name_r");
    let c_roman_n = col("station_name_rn");
    let c_zh = col("station_name_zh");
    let c_ko = col("station_name_ko");
    let c_tlc = col("three_letter_code");
    let c_pref = col("pref_cd");
    let c_post = col("post");
    let c_addr = col("address");
    let c_open = col("open_ymd");
    let c_close = col("close_ymd");
    let c_status = col("e_status");

    let text = |r: &csv::StringRecord, i: usize| r.get(i).unwrap_or("").to_string();
    let opt = |r: &csv::StringRecord, i: Option<usize>| {
        i.and_then(|i| r.get(i))
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    let num = |r: &csv::StringRecord, i: Option<usize>| -> Option<f64> {
        i.and_then(|i| r.get(i)).and_then(|v| v.trim().parse().ok())
    };

    let mut out = Vec::with_capacity(12_000);
    for record in reader.records().flatten() {
        // 座標か ID が壊れている行は索引から落とす (検索対象にならない)
        let (Some(station_cd), Some(station_g_cd), Some(lat), Some(lon)) = (
            record.get(c_cd).and_then(|v| v.trim().parse::<u32>().ok()),
            record.get(c_gcd).and_then(|v| v.trim().parse::<u32>().ok()),
            record.get(c_lat).and_then(|v| v.trim().parse::<f64>().ok()),
            record.get(c_lon).and_then(|v| v.trim().parse::<f64>().ok()),
        ) else {
            continue;
        };

        out.push(StationRecord {
            station_cd,
            station_g_cd,
            name: text(&record, c_name),
            name_katakana: text(&record, c_kana),
            name_roman: opt(&record, c_roman),
            name_roman_normalized: opt(&record, c_roman_n),
            name_chinese: opt(&record, c_zh),
            name_korean: opt(&record, c_ko),
            three_letter_code: opt(&record, c_tlc),
            line_cd: record
                .get(c_line)
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0),
            pref_cd: num(&record, c_pref).unwrap_or(0.0) as u32,
            postal_code: opt(&record, c_post).unwrap_or_default(),
            address: opt(&record, c_addr).unwrap_or_default(),
            lat,
            lon,
            opened_at: opt(&record, c_open).unwrap_or_default(),
            closed_at: opt(&record, c_close).unwrap_or_default(),
            e_status: num(&record, c_status).unwrap_or(0.0) as i32,
        });
    }
    out
}

/// 球面距離 (km)。既存 SQL のユークリッド距離より正確で、順序もほぼ一致する。
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * a.sqrt().clamp(-1.0, 1.0).asin()
}

/// 全件走査で最近傍 limit 件を返す。11,148 駅なので索引なしで十分速い。
pub fn nearest(lat: f64, lon: f64, limit: usize) -> Vec<(&'static StationRecord, f64)> {
    let mut scored: Vec<(&StationRecord, f64)> = stations()
        .iter()
        .filter(|s| s.e_status == 0)
        .filter(|s| joins_line(s))
        .map(|s| (s, haversine_km(lat, lon, s.lat, s.lon)))
        .collect();

    let cmp = |a: &(&StationRecord, f64), b: &(&StationRecord, f64)| {
        a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
    };
    // 全体ソートを避け、上位 limit 件だけを確定させる
    if limit < scored.len() {
        scored.select_nth_unstable_by(limit, cmp);
        scored.truncate(limit);
    }
    scored.sort_unstable_by(cmp);
    scored
}

/// 既存 SQL の WHERE 句と同じ部分一致セマンティクス。
///
/// PostgreSQL 側の `pg_trgm` は `LIKE '%...%'` を高速化する GIN インデックスであって
/// 類似度検索ではないため、`contains()` で論理的に等価な結果が得られる。
/// 正規化には domain 層の `normalize_for_search` をそのまま使うので挙動も一致する。
pub fn search_by_name(query: &str, limit: usize) -> Vec<&'static StationRecord> {
    if query.is_empty() {
        return Vec::new();
    }
    // s.station_name_k LIKE $4 用: ひらがな→カタカナ、全角→半角
    let katakana = normalize_for_search(query);
    // s.station_name_rn ILIKE $3 用
    let lowered = query.to_lowercase();

    let mut hits: Vec<&'static StationRecord> = stations()
        .iter()
        .filter(|s| s.e_status == 0)
        .filter(|s| joins_active_line(s))
        .filter(|s| {
            s.name.contains(query)
                || s
                    .name_roman_normalized
                    .as_deref()
                    .is_some_and(|v| v.to_lowercase().contains(&lowered))
                || s.name_katakana.contains(&katakana)
                || s.name_chinese.as_deref().is_some_and(|v| v.contains(query))
                || s.name_korean.as_deref().is_some_and(|v| v.contains(query))
        })
        .collect();

    // ORDER BY station_g_cd, station_name
    hits.sort_unstable_by(|a, b| {
        a.station_g_cd
            .cmp(&b.station_g_cd)
            .then_with(|| a.name.cmp(&b.name))
    });
    hits.truncate(limit);
    hits
}
