//! CSV をバイナリに埋め込み、isolate 起動時に一度だけパースしてメモリに保持する。
//! PostgreSQL のクエリを全件走査・HashMap 参照で置き換える。

use stationapi::domain::entity::company::Company;
use stationapi::domain::entity::gtfs::TransportType;
use stationapi::domain::entity::line::Line;
use stationapi::domain::entity::station::Station;
use stationapi::domain::normalize::normalize_for_search;
use stationapi::model::StopCondition;
use std::collections::HashMap;
use std::sync::OnceLock;

const STATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/stations.csv"));
const LINES_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/lines.csv"));
const COMPANIES_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/companies.csv"));

// ---------------------------------------------------------------- CSV ヘルパー

struct Cols {
    headers: csv::StringRecord,
}

impl Cols {
    fn of(headers: &csv::StringRecord) -> Self {
        Self {
            headers: headers.clone(),
        }
    }
    fn at(&self, name: &str) -> Option<usize> {
        self.headers.iter().position(|h| h.trim() == name)
    }
}

fn text(r: &csv::StringRecord, i: Option<usize>) -> String {
    i.and_then(|i| r.get(i)).unwrap_or("").to_string()
}

/// 空文字は NULL 相当として扱う (CSV には NULL 表現がないため)
fn opt_text(r: &csv::StringRecord, i: Option<usize>) -> Option<String> {
    i.and_then(|i| r.get(i))
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn opt_i32(r: &csv::StringRecord, i: Option<usize>) -> Option<i32> {
    i.and_then(|i| r.get(i)).and_then(|v| v.trim().parse().ok())
}

fn i32_or(r: &csv::StringRecord, i: Option<usize>, default: i32) -> i32 {
    opt_i32(r, i).unwrap_or(default)
}

fn opt_f64(r: &csv::StringRecord, i: Option<usize>) -> Option<f64> {
    i.and_then(|i| r.get(i)).and_then(|v| v.trim().parse().ok())
}

fn reader(csv_text: &'static str) -> csv::Reader<&'static [u8]> {
    csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_text.as_bytes())
}

// ---------------------------------------------------------------- 駅

/// 検索に必要な列だけを持つ軽量レコード。
/// Station エンティティ (66 フィールド) は応答生成時にのみ組み立てる。
pub struct StationRecord {
    pub station_cd: i32,
    pub station_g_cd: i32,
    pub name: String,
    pub name_katakana: String,
    pub name_roman: Option<String>,
    /// station_name_rn (マクロンを含まない綴り) を小文字化したもの。
    /// ILIKE 相当の比較を全件走査で行うため、検索のたびに 11,148 件分
    /// to_lowercase() を呼ばないよう索引時に持つ。応答には使わないので
    /// 小文字版だけを保持する。
    name_roman_lower: Option<String>,
    pub name_chinese: Option<String>,
    pub name_korean: Option<String>,
    pub station_number1: Option<String>,
    pub station_number2: Option<String>,
    pub station_number3: Option<String>,
    pub station_number4: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: i32,
    pub pref_cd: i32,
    pub postal_code: String,
    pub address: String,
    pub lat: f64,
    pub lon: f64,
    pub opened_at: String,
    pub closed_at: String,
    pub e_status: i32,
    pub e_sort: i32,
    /// 0 = 鉄道, 1 = バス。GTFS 統合後の DB から書き出した CSV に含まれる。
    /// data/*.csv にフォールバックした場合は列が無いので Rail 扱いになる。
    pub transport_type: TransportType,
}

impl StationRecord {
    /// 既存 SQL の `stations JOIN lines` 相当。路線側の属性を埋めた Station を返す。
    /// 列車種別 (type_* / line_group_cd / pass) は UseCase 層が後から付与する。
    ///
    /// 路線名の 7 列は `line_aliases` / `aliases` による別名があればそちらを使う。
    /// 既存 SQL の `COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL)`
    /// に対応する (別名が空文字なら元の値へフォールバックする)。
    pub fn to_entity(&self, line: Option<&Line>) -> Station {
        let alias = alias_by_station(self.station_cd);
        // 既存 SQL の `COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL)` 相当。
        // aliases 側は CSV の空欄が NULL なので、別名が空なら路線の値へ落ちる。
        // 最後に空文字を NULL へ寄せるのが NULLIF の分で、これが無いと
        // 路線名を持たないバス路線の駅で null ではなく空文字が出る。
        let pick = |a: Option<&String>, l: Option<String>| -> Option<String> {
            a.cloned()
                .filter(|v| !v.is_empty())
                .or(l)
                .filter(|v| !v.is_empty())
        };
        Station {
            station_cd: self.station_cd,
            station_g_cd: self.station_g_cd,
            station_name: self.name.clone(),
            station_name_k: self.name_katakana.clone(),
            station_name_r: self.name_roman.clone(),
            station_name_zh: self.name_chinese.clone(),
            station_name_ko: self.name_korean.clone(),
            station_numbers: vec![],
            station_number1: self.station_number1.clone(),
            station_number2: self.station_number2.clone(),
            station_number3: self.station_number3.clone(),
            station_number4: self.station_number4.clone(),
            three_letter_code: self.three_letter_code.clone(),
            line_cd: self.line_cd,
            line: None,
            lines: vec![],
            pref_cd: self.pref_cd,
            post: self.postal_code.clone(),
            address: self.address.clone(),
            lon: self.lon,
            lat: self.lat,
            open_ymd: self.opened_at.clone(),
            close_ymd: self.closed_at.clone(),
            e_status: self.e_status,
            e_sort: self.e_sort,
            stop_condition: StopCondition::All,
            distance: None,
            train_type: None,
            has_train_types: false,
            company_cd: line.map(|l| l.company_cd),
            line_name: pick(
                alias.and_then(|a| a.line_name.as_ref()),
                line.map(|l| l.line_name.clone()),
            ),
            line_name_k: pick(
                alias.and_then(|a| a.line_name_k.as_ref()),
                line.map(|l| l.line_name_k.clone()),
            ),
            line_name_h: pick(
                alias.and_then(|a| a.line_name_h.as_ref()),
                line.map(|l| l.line_name_h.clone()),
            ),
            line_name_r: pick(
                alias.and_then(|a| a.line_name_r.as_ref()),
                line.and_then(|l| l.line_name_r.clone()),
            ),
            line_name_zh: pick(
                alias.and_then(|a| a.line_name_zh.as_ref()),
                line.and_then(|l| l.line_name_zh.clone()),
            ),
            line_name_ko: pick(
                alias.and_then(|a| a.line_name_ko.as_ref()),
                line.and_then(|l| l.line_name_ko.clone()),
            ),
            line_color_c: pick(
                alias.and_then(|a| a.line_color_c.as_ref()),
                line.and_then(|l| l.line_color_c.clone()),
            ),
            line_type: line.and_then(|l| l.line_type),
            line_symbol1: line.and_then(|l| l.line_symbol1.clone()),
            line_symbol2: line.and_then(|l| l.line_symbol2.clone()),
            line_symbol3: line.and_then(|l| l.line_symbol3.clone()),
            line_symbol4: line.and_then(|l| l.line_symbol4.clone()),
            line_symbol1_color: line.and_then(|l| l.line_symbol1_color.clone()),
            line_symbol2_color: line.and_then(|l| l.line_symbol2_color.clone()),
            line_symbol3_color: line.and_then(|l| l.line_symbol3_color.clone()),
            line_symbol4_color: line.and_then(|l| l.line_symbol4_color.clone()),
            line_symbol1_shape: line.and_then(|l| l.line_symbol1_shape.clone()),
            line_symbol2_shape: line.and_then(|l| l.line_symbol2_shape.clone()),
            line_symbol3_shape: line.and_then(|l| l.line_symbol3_shape.clone()),
            line_symbol4_shape: line.and_then(|l| l.line_symbol4_shape.clone()),
            average_distance: line.and_then(|l| l.average_distance),
            type_id: None,
            sst_id: None,
            type_cd: None,
            line_group_cd: None,
            pass: None,
            type_name: None,
            type_name_k: None,
            type_name_r: None,
            type_name_zh: None,
            type_name_ko: None,
            color: None,
            direction: None,
            kind: None,
            transport_type: self.transport_type,
        }
    }
}

static STATIONS: OnceLock<Vec<StationRecord>> = OnceLock::new();

pub fn stations() -> &'static [StationRecord] {
    STATIONS.get_or_init(build_stations)
}

fn build_stations() -> Vec<StationRecord> {
    let mut rdr = reader(STATIONS_CSV);
    let Ok(headers) = rdr.headers().cloned() else {
        return Vec::new();
    };
    let c = Cols::of(&headers);
    // 応答に出る列が欠けたまま索引を作ると、名前が空の駅が黙って並ぶ。
    // 生成 CSV は information_schema から列を取るので、DB のスキーマ変更が
    // そのまま伝播する。列名の変化を検出できるよう必須列を明示する。
    let (Some(i_cd), Some(i_gcd), Some(i_lat), Some(i_lon)) = (
        c.at("station_cd"),
        c.at("station_g_cd"),
        c.at("lat"),
        c.at("lon"),
    ) else {
        panic!("stations の CSV に station_cd / station_g_cd / lat / lon が必要です");
    };
    for required in ["station_name", "station_name_k"] {
        assert!(
            c.at(required).is_some(),
            "stations の CSV に {required} 列がありません"
        );
    }

    let mut out = Vec::with_capacity(12_000);
    for r in rdr.records().flatten() {
        // 座標か ID が壊れている行は索引に載せない
        let (Some(station_cd), Some(station_g_cd), Some(lat), Some(lon)) = (
            opt_i32(&r, Some(i_cd)),
            opt_i32(&r, Some(i_gcd)),
            opt_f64(&r, Some(i_lat)),
            opt_f64(&r, Some(i_lon)),
        ) else {
            continue;
        };
        out.push(StationRecord {
            station_cd,
            station_g_cd,
            name: text(&r, c.at("station_name")),
            name_katakana: text(&r, c.at("station_name_k")),
            name_roman: opt_text(&r, c.at("station_name_r")),
            name_roman_lower: opt_text(&r, c.at("station_name_rn")).map(|v| v.to_lowercase()),
            name_chinese: opt_text(&r, c.at("station_name_zh")),
            name_korean: opt_text(&r, c.at("station_name_ko")),
            station_number1: opt_text(&r, c.at("station_number1")),
            station_number2: opt_text(&r, c.at("station_number2")),
            station_number3: opt_text(&r, c.at("station_number3")),
            station_number4: opt_text(&r, c.at("station_number4")),
            three_letter_code: opt_text(&r, c.at("three_letter_code")),
            line_cd: i32_or(&r, c.at("line_cd"), 0),
            pref_cd: i32_or(&r, c.at("pref_cd"), 0),
            postal_code: text(&r, c.at("post")),
            address: text(&r, c.at("address")),
            lat,
            lon,
            opened_at: text(&r, c.at("open_ymd")),
            closed_at: text(&r, c.at("close_ymd")),
            e_status: i32_or(&r, c.at("e_status"), 0),
            e_sort: i32_or(&r, c.at("e_sort"), 0),
            transport_type: TransportType::from(i32_or(&r, c.at("transport_type"), 0)),
        });
    }
    out
}

/// station_cd -> stations() の添字
static STATION_BY_CD: OnceLock<HashMap<i32, usize>> = OnceLock::new();
/// station_g_cd -> stations() の添字リスト
static STATION_BY_GROUP: OnceLock<HashMap<i32, Vec<usize>>> = OnceLock::new();

pub fn station_by_cd(station_cd: i32) -> Option<&'static StationRecord> {
    let idx = STATION_BY_CD.get_or_init(|| {
        stations()
            .iter()
            .enumerate()
            .map(|(i, s)| (s.station_cd, i))
            .collect()
    });
    idx.get(&station_cd).map(|&i| &stations()[i])
}

pub fn stations_by_group(station_g_cd: i32) -> impl Iterator<Item = &'static StationRecord> {
    let idx = STATION_BY_GROUP.get_or_init(|| {
        let mut map: HashMap<i32, Vec<usize>> = HashMap::with_capacity(10_000);
        for (i, s) in stations().iter().enumerate() {
            map.entry(s.station_g_cd).or_default().push(i);
        }
        map
    });
    idx.get(&station_g_cd)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .map(|&i| &stations()[i])
}

// ---------------------------------------------------------------- 路線

static LINES: OnceLock<Vec<Line>> = OnceLock::new();
static LINE_BY_CD: OnceLock<HashMap<i32, usize>> = OnceLock::new();

pub fn lines() -> &'static [Line] {
    LINES.get_or_init(build_lines)
}

pub fn line_by_cd(line_cd: i32) -> Option<&'static Line> {
    let idx = LINE_BY_CD.get_or_init(|| {
        lines()
            .iter()
            .enumerate()
            .map(|(i, l)| (l.line_cd, i))
            .collect()
    });
    idx.get(&line_cd).map(|&i| &lines()[i])
}

fn build_lines() -> Vec<Line> {
    let mut rdr = reader(LINES_CSV);
    let Ok(headers) = rdr.headers().cloned() else {
        return Vec::new();
    };
    let c = Cols::of(&headers);
    let Some(i_cd) = c.at("line_cd") else {
        panic!("lines の CSV に line_cd 列がありません");
    };
    for required in ["line_name", "line_name_k", "line_name_h"] {
        assert!(
            c.at(required).is_some(),
            "lines の CSV に {required} 列がありません"
        );
    }

    let mut out = Vec::with_capacity(1024);
    for r in rdr.records().flatten() {
        let Some(line_cd) = opt_i32(&r, Some(i_cd)) else {
            continue;
        };
        out.push(Line {
            line_cd,
            company_cd: i32_or(&r, c.at("company_cd"), 0),
            company: None,
            line_name: text(&r, c.at("line_name")),
            line_name_k: text(&r, c.at("line_name_k")),
            line_name_h: text(&r, c.at("line_name_h")),
            // lines のこの 3 列は DB 側で既定値 `''` を持つ (line_name_r は NOT NULL)。
            // CSV は NULL と空文字を区別できないが、空になるのは値を入れない
            // バス路線だけで、そちらは既定値の `''` が入る。null にすると
            // 本番と食い違うので空文字として扱う。
            line_name_r: Some(text(&r, c.at("line_name_r"))),
            line_name_zh: Some(text(&r, c.at("line_name_zh"))),
            line_name_ko: Some(text(&r, c.at("line_name_ko"))),
            line_color_c: opt_text(&r, c.at("line_color_c")),
            line_type: opt_i32(&r, c.at("line_type")),
            line_symbols: vec![],
            line_symbol1: opt_text(&r, c.at("line_symbol1")),
            line_symbol2: opt_text(&r, c.at("line_symbol2")),
            line_symbol3: opt_text(&r, c.at("line_symbol3")),
            line_symbol4: opt_text(&r, c.at("line_symbol4")),
            line_symbol1_color: opt_text(&r, c.at("line_symbol1_color")),
            line_symbol2_color: opt_text(&r, c.at("line_symbol2_color")),
            line_symbol3_color: opt_text(&r, c.at("line_symbol3_color")),
            line_symbol4_color: opt_text(&r, c.at("line_symbol4_color")),
            line_symbol1_shape: opt_text(&r, c.at("line_symbol1_shape")),
            line_symbol2_shape: opt_text(&r, c.at("line_symbol2_shape")),
            line_symbol3_shape: opt_text(&r, c.at("line_symbol3_shape")),
            line_symbol4_shape: opt_text(&r, c.at("line_symbol4_shape")),
            e_status: i32_or(&r, c.at("e_status"), 0),
            e_sort: i32_or(&r, c.at("e_sort"), 0),
            average_distance: opt_f64(&r, c.at("average_distance")),
            station: None,
            train_type: None,
            line_group_cd: None,
            station_cd: None,
            station_g_cd: None,
            type_cd: None,
            transport_type: TransportType::from(i32_or(&r, c.at("transport_type"), 0)),
        });
    }
    out
}

/// `stations JOIN lines ON s.line_cd = l.line_cd` (INNER) 相当。
/// lines に無い line_cd の駅は既存 SQL でも結果に出ない。座標検索がこの条件。
fn joins_line(record: &StationRecord) -> bool {
    line_by_cd(record.line_cd).is_some()
}

/// 名前検索の JOIN は `AND l.e_status = 0` を追加で要求する。
/// これを見ないと廃止路線 (例: 成田エクスプレス, e_status=3) の駅が混ざる。
fn joins_active_line(record: &StationRecord) -> bool {
    line_by_cd(record.line_cd).is_some_and(|l| l.e_status == 0)
}

// ---------------------------------------------------------------- 事業者

static COMPANIES: OnceLock<Vec<Company>> = OnceLock::new();

pub fn companies() -> &'static [Company] {
    COMPANIES.get_or_init(build_companies)
}

fn build_companies() -> Vec<Company> {
    let mut rdr = reader(COMPANIES_CSV);
    let Ok(headers) = rdr.headers().cloned() else {
        return Vec::new();
    };
    let c = Cols::of(&headers);
    let Some(i_cd) = c.at("company_cd") else {
        panic!("companies の CSV に company_cd 列がありません");
    };
    for required in ["company_name", "company_name_k"] {
        assert!(
            c.at(required).is_some(),
            "companies の CSV に {required} 列がありません"
        );
    }

    let mut out = Vec::with_capacity(256);
    for r in rdr.records().flatten() {
        let Some(company_cd) = opt_i32(&r, Some(i_cd)) else {
            continue;
        };
        out.push(Company {
            company_cd,
            rr_cd: i32_or(&r, c.at("rr_cd"), 0),
            company_name: text(&r, c.at("company_name")),
            company_name_k: text(&r, c.at("company_name_k")),
            company_name_h: text(&r, c.at("company_name_h")),
            company_name_r: text(&r, c.at("company_name_r")),
            company_name_en: text(&r, c.at("company_name_en")),
            company_name_full_en: text(&r, c.at("company_name_full_en")),
            company_url: opt_text(&r, c.at("company_url")),
            company_type: i32_or(&r, c.at("company_type"), 0),
            e_status: i32_or(&r, c.at("e_status"), 0),
            e_sort: i32_or(&r, c.at("e_sort"), 0),
        });
    }
    out
}

// ---------------------------------------------------------------- 検索

/// 球面距離 (km)。
///
/// 既存 SQL は `point(lat,lon) <-> point($1,$2)` で度単位のユークリッド距離を使うため、
/// 緯度と経度を同じスケールで扱い東西方向を過大評価する。こちらは実距離で並ぶ。
/// 実測では上位14件が同着 (同一駅の路線別レコード) で、順序差が実質的に出るのは
/// 18位以降・距離にして50-100m程度。集合としては既存と一致する。
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * a.sqrt().clamp(-1.0, 1.0).asin()
}

/// 全件走査で最近傍 limit 件を返す。11,148 駅なので索引なしで十分速い。
///
/// `want` は既存 SQL の `$4::int IS NULL OR COALESCE(s.transport_type, 0) = $4` に対応する。
/// 未指定 (RailAndBus) のときは
/// `ORDER BY CASE WHEN $4 IS NULL THEN COALESCE(s.transport_type, 0) ELSE 0 END, 距離`
/// となり、鉄道を先・バスを後に並べたうえで距離順になる。
pub fn nearest(
    lat: f64,
    lon: f64,
    limit: usize,
    want: Option<i32>,
) -> Vec<(&'static StationRecord, f64)> {
    nearest_inner(lat, lon, limit, want, true)
}

/// 路線の存在を条件にしないまま最近傍を取る。
///
/// `get_bus_stops_near_stations` の SQL は LATERAL の中で先に LIMIT を掛け、
/// そのあと `JOIN lines` で絞る。先に路線で絞ると件数が変わるため、
/// この順序を保つ用途で使う。
pub fn nearest_without_line_join(
    lat: f64,
    lon: f64,
    limit: usize,
    want: Option<i32>,
) -> Vec<(&'static StationRecord, f64)> {
    nearest_inner(lat, lon, limit, want, false)
}

fn nearest_inner(
    lat: f64,
    lon: f64,
    limit: usize,
    want: Option<i32>,
    require_line: bool,
) -> Vec<(&'static StationRecord, f64)> {
    let mut scored: Vec<(&StationRecord, f64)> = stations()
        .iter()
        .filter(|s| s.e_status == 0)
        .filter(|s| !require_line || joins_line(s))
        .filter(|s| want.is_none_or(|w| s.transport_type as i32 == w))
        .map(|s| (s, haversine_km(lat, lon, s.lat, s.lon)))
        .collect();

    // 種別指定がある場合は第1キーが定数 0 になるので距離だけで並ぶ
    let rank = move |s: &StationRecord| -> i32 {
        if want.is_none() {
            s.transport_type as i32
        } else {
            0
        }
    };
    let cmp = move |a: &(&StationRecord, f64), b: &(&StationRecord, f64)| {
        rank(a.0)
            .cmp(&rank(b.0))
            .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
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
pub fn search_by_name(query: &str, limit: usize, want: Option<i32>) -> Vec<&'static StationRecord> {
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
        .filter(|s| want.is_none_or(|w| s.transport_type as i32 == w))
        .filter(|s| {
            s.name.contains(query)
                || s.name_roman_lower
                    .as_deref()
                    .is_some_and(|v| v.contains(&lowered))
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

// ---------------------------------------------------------------- 列車種別

const TYPES_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/types.csv"));
/// build.rs が生成した固定長バイナリ (1 行 = i32 x 4, リトルエンディアン)。
/// CSV のままだと 41,250 行のパースがコールドスタートの大半を占めるため。
const SST_BIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sst.bin"));
/// build.rs 側と揃えた欠損値表現
const NULL_I32: i32 = i32::MIN;

/// types.csv の 1 行。`id` 列は CSV 上 DEFAULT で、PostgreSQL では SERIAL が
/// 行順に採番する (`t.id AS type_id` として応答に出る)。ここでも同じ順で振る。
pub struct TypeRecord {
    pub id: i32,
    pub type_cd: i32,
    pub type_name: String,
    pub type_name_k: String,
    pub type_name_r: Option<String>,
    pub type_name_zh: Option<String>,
    pub type_name_ko: Option<String>,
    pub color: String,
    pub direction: Option<i32>,
    pub kind: Option<i32>,
    /// ORDER BY t.priority DESC に使う
    pub priority: i32,
}

/// station_station_types.csv の 1 行。
///
/// `id` 列は CSV 上 "DEFAULT" で、PostgreSQL では SERIAL が行順に採番する。
/// この id は停車順序そのものとして使われる (`ORDER BY sst.id`) ため、
/// ここでも取り込み順に 1 始まりの連番を振って一致させる。
pub struct SstRecord {
    pub id: i32,
    pub station_cd: i32,
    pub type_cd: i32,
    pub line_group_cd: Option<i32>,
    pub pass: Option<i32>,
}

static TYPES: OnceLock<Vec<TypeRecord>> = OnceLock::new();
static TYPE_BY_CD: OnceLock<HashMap<i32, usize>> = OnceLock::new();
static SSTS: OnceLock<Vec<SstRecord>> = OnceLock::new();
static SST_BY_STATION: OnceLock<HashMap<i32, Vec<usize>>> = OnceLock::new();

pub fn types() -> &'static [TypeRecord] {
    TYPES.get_or_init(build_types)
}

pub fn type_by_cd(type_cd: i32) -> Option<&'static TypeRecord> {
    let idx = TYPE_BY_CD.get_or_init(|| {
        types()
            .iter()
            .enumerate()
            .map(|(i, t)| (t.type_cd, i))
            .collect()
    });
    idx.get(&type_cd).map(|&i| &types()[i])
}

fn build_types() -> Vec<TypeRecord> {
    let mut rdr = reader(TYPES_CSV);
    let Ok(headers) = rdr.headers().cloned() else {
        return Vec::new();
    };
    let c = Cols::of(&headers);
    let Some(i_cd) = c.at("type_cd") else {
        return Vec::new();
    };

    // 生成物には SERIAL 採番後の実 id がある。PostgreSQL は取り込んだ全行に
    // 採番するので、こちらで行をスキップすると id がずれる。この id は
    // station.type_id として応答に出るため、ずれていたら索引構築を中断する。
    let i_id = c.at("id");
    let mut out = Vec::with_capacity(512);
    let mut serial = 0i32;
    for r in rdr.records().flatten() {
        let Some(type_cd) = opt_i32(&r, Some(i_cd)) else {
            continue;
        };
        serial += 1;
        if let Some(actual) = opt_i32(&r, i_id) {
            assert_eq!(
                actual, serial,
                "types.csv の id が連番ではない (期待 {serial}, 実際 {actual})"
            );
        }
        out.push(TypeRecord {
            id: serial,
            type_cd,
            type_name: text(&r, c.at("type_name")),
            type_name_k: text(&r, c.at("type_name_k")),
            type_name_r: opt_text(&r, c.at("type_name_r")),
            type_name_zh: opt_text(&r, c.at("type_name_zh")),
            type_name_ko: opt_text(&r, c.at("type_name_ko")),
            color: text(&r, c.at("color")),
            direction: opt_i32(&r, c.at("direction")),
            kind: opt_i32(&r, c.at("kind")),
            priority: i32_or(&r, c.at("priority"), 0),
        });
    }
    out
}

pub fn ssts() -> &'static [SstRecord] {
    SSTS.get_or_init(build_ssts)
}

fn build_ssts() -> Vec<SstRecord> {
    const ROW: usize = 16; // i32 x 4
    let read = |chunk: &[u8], i: usize| -> i32 {
        i32::from_le_bytes([
            chunk[i * 4],
            chunk[i * 4 + 1],
            chunk[i * 4 + 2],
            chunk[i * 4 + 3],
        ])
    };
    let nullable = |v: i32| (v != NULL_I32).then_some(v);

    SST_BIN
        .chunks_exact(ROW)
        .enumerate()
        .map(|(i, chunk)| SstRecord {
            // SERIAL 相当。build.rs が CSV の行順を保っているのでここで採番できる
            id: i as i32 + 1,
            station_cd: read(chunk, 0),
            type_cd: read(chunk, 1),
            line_group_cd: nullable(read(chunk, 2)),
            pass: nullable(read(chunk, 3)),
        })
        .collect()
}

/// station_cd に紐づく station_station_types を sst.id 昇順で返す。
/// 取り込み順がそのまま id 順なので、添字リストは並べ替え不要。
pub fn sst_by_station(station_cd: i32) -> impl Iterator<Item = &'static SstRecord> {
    let idx = SST_BY_STATION.get_or_init(|| {
        let mut map: HashMap<i32, Vec<usize>> = HashMap::with_capacity(12_000);
        for (i, s) in ssts().iter().enumerate() {
            map.entry(s.station_cd).or_default().push(i);
        }
        map
    });
    idx.get(&station_cd)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .map(|&i| &ssts()[i])
}

/// 座標検索の has_train_types 用サブクエリ相当:
/// `SELECT line_group_cd FROM station_station_types WHERE station_cd = ?
///  AND line_group_cd IS NOT NULL ORDER BY id LIMIT 1`
pub fn first_line_group_cd(station_cd: i32) -> Option<i32> {
    sst_by_station(station_cd).find_map(|s| s.line_group_cd)
}

/// line_group_cd -> station_station_types の添字リスト (sst.id 昇順)
static SST_BY_GROUP: OnceLock<HashMap<i32, Vec<usize>>> = OnceLock::new();

pub fn sst_by_group(line_group_cd: i32) -> impl Iterator<Item = &'static SstRecord> {
    let idx = SST_BY_GROUP.get_or_init(|| {
        let mut map: HashMap<i32, Vec<usize>> = HashMap::with_capacity(4_000);
        for (i, s) in ssts().iter().enumerate() {
            if let Some(group) = s.line_group_cd {
                map.entry(group).or_default().push(i);
            }
        }
        map
    });
    idx.get(&line_group_cd)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .map(|&i| &ssts()[i])
}

/// line_cd -> line_name_rn。
/// Line エンティティは line_name_rn を持たない (検索専用列) ため別に保持する。
static LINE_NAME_RN: OnceLock<HashMap<i32, String>> = OnceLock::new();

pub fn line_name_rn(line_cd: i32) -> Option<&'static str> {
    LINE_NAME_RN
        .get_or_init(|| {
            let mut rdr = reader(LINES_CSV);
            let Ok(headers) = rdr.headers().cloned() else {
                return HashMap::new();
            };
            let c = Cols::of(&headers);
            let (Some(i_cd), Some(i_rn)) = (c.at("line_cd"), c.at("line_name_rn")) else {
                return HashMap::new();
            };
            let mut map = HashMap::with_capacity(1024);
            for r in rdr.records().flatten() {
                let (Some(cd), Some(rn)) = (opt_i32(&r, Some(i_cd)), opt_text(&r, Some(i_rn)))
                else {
                    continue;
                };
                map.insert(cd, rn);
            }
            map
        })
        .get(&line_cd)
        .map(String::as_str)
}

/// line_cd -> stations() の添字リスト
static STATION_BY_LINE: OnceLock<HashMap<i32, Vec<usize>>> = OnceLock::new();

pub fn stations_by_line(line_cd: i32) -> impl Iterator<Item = &'static StationRecord> {
    let idx = STATION_BY_LINE.get_or_init(|| {
        let mut map: HashMap<i32, Vec<usize>> = HashMap::with_capacity(1_024);
        for (i, s) in stations().iter().enumerate() {
            map.entry(s.line_cd).or_default().push(i);
        }
        map
    });
    idx.get(&line_cd)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .map(|&i| &stations()[i])
}

// ---------------------------------------------------------------- 路線名の別名

const ALIASES_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/aliases.csv"));
const LINE_ALIASES_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/line_aliases.csv"));

/// aliases.csv の 1 行。路線名を差し替えるための別名。
/// 例: 東武伊勢崎線 -> 東武スカイツリーライン
pub struct AliasRecord {
    pub line_name: Option<String>,
    pub line_name_k: Option<String>,
    pub line_name_h: Option<String>,
    pub line_name_r: Option<String>,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: Option<String>,
}

/// station_cd -> 別名。
///
/// 既存 SQL は
/// `LEFT JOIN line_aliases la ON la.station_cd = s.station_cd`
/// `LEFT JOIN aliases a ON a.id = la.alias_cd`
/// として駅ごとに引き、`COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL)`
/// で路線名 7 列を差し替える。
static ALIAS_BY_STATION: OnceLock<HashMap<i32, AliasRecord>> = OnceLock::new();

pub fn alias_by_station(station_cd: i32) -> Option<&'static AliasRecord> {
    ALIAS_BY_STATION
        .get_or_init(build_alias_index)
        .get(&station_cd)
}

fn build_alias_index() -> HashMap<i32, AliasRecord> {
    // まず alias 本体を id で引けるようにする
    let mut by_id: HashMap<i32, AliasRecord> = HashMap::new();
    {
        let mut rdr = reader(ALIASES_CSV);
        let Ok(headers) = rdr.headers().cloned() else {
            return HashMap::new();
        };
        let c = Cols::of(&headers);
        let Some(i_id) = c.at("id") else {
            panic!("aliases の CSV に id 列がありません");
        };
        for r in rdr.records().flatten() {
            let Some(id) = opt_i32(&r, Some(i_id)) else {
                continue;
            };
            by_id.insert(
                id,
                AliasRecord {
                    line_name: opt_text(&r, c.at("line_name")),
                    line_name_k: opt_text(&r, c.at("line_name_k")),
                    line_name_h: opt_text(&r, c.at("line_name_h")),
                    line_name_r: opt_text(&r, c.at("line_name_r")),
                    line_name_zh: opt_text(&r, c.at("line_name_zh")),
                    line_name_ko: opt_text(&r, c.at("line_name_ko")),
                    line_color_c: opt_text(&r, c.at("line_color_c")),
                },
            );
        }
    }

    // 駅ごとの紐付けを引く
    let mut out: HashMap<i32, AliasRecord> = HashMap::new();
    let mut rdr = reader(LINE_ALIASES_CSV);
    let Ok(headers) = rdr.headers().cloned() else {
        return out;
    };
    let c = Cols::of(&headers);
    let (Some(i_station), Some(i_alias)) = (c.at("station_cd"), c.at("alias_cd")) else {
        panic!("line_aliases の CSV に station_cd / alias_cd が必要です");
    };
    for r in rdr.records().flatten() {
        let (Some(station_cd), Some(alias_cd)) =
            (opt_i32(&r, Some(i_station)), opt_i32(&r, Some(i_alias)))
        else {
            continue;
        };
        if let Some(alias) = by_id.get(&alias_cd) {
            out.insert(
                station_cd,
                AliasRecord {
                    line_name: alias.line_name.clone(),
                    line_name_k: alias.line_name_k.clone(),
                    line_name_h: alias.line_name_h.clone(),
                    line_name_r: alias.line_name_r.clone(),
                    line_name_zh: alias.line_name_zh.clone(),
                    line_name_ko: alias.line_name_ko.clone(),
                    line_color_c: alias.line_color_c.clone(),
                },
            );
        }
    }
    out
}

/// 駅に紐づく別名を Line へ反映する。
///
/// 既存 SQL は Line を返すクエリでも `line_aliases` / `aliases` を LEFT JOIN し、
/// 路線名 7 列を差し替える。Line 単体で引く API (find_by_id / get_by_ids) には
/// この JOIN が無いので、駅を伴う経路でのみ適用する。
pub fn apply_line_alias(line: &mut Line, station_cd: i32) {
    let Some(alias) = alias_by_station(station_cd) else {
        return;
    };
    let pick = |a: Option<&String>, current: Option<String>| -> Option<String> {
        a.cloned().filter(|v| !v.is_empty()).or(current)
    };
    if let Some(v) = pick(alias.line_name.as_ref(), Some(line.line_name.clone())) {
        line.line_name = v;
    }
    if let Some(v) = pick(alias.line_name_k.as_ref(), Some(line.line_name_k.clone())) {
        line.line_name_k = v;
    }
    if let Some(v) = pick(alias.line_name_h.as_ref(), Some(line.line_name_h.clone())) {
        line.line_name_h = v;
    }
    line.line_name_r = pick(alias.line_name_r.as_ref(), line.line_name_r.clone());
    line.line_name_zh = pick(alias.line_name_zh.as_ref(), line.line_name_zh.clone());
    line.line_name_ko = pick(alias.line_name_ko.as_ref(), line.line_name_ko.clone());
    line.line_color_c = pick(alias.line_color_c.as_ref(), line.line_color_c.clone());
}
