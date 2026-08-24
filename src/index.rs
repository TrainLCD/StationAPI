//! CSV をバイナリに埋め込み、isolate 起動時に一度だけパースしてメモリに保持する。
//! 検索は全件走査と HashMap 参照で行う。

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
    /// 路線側の属性を埋めた Station を返す。
    /// 列車種別 (type_* / line_group_cd / pass) は UseCase 層が後から付与する。
    ///
    /// 路線名の 7 列は `line_aliases` / `aliases` による別名があればそちらを使う。
    pub fn to_entity(&self, line: Option<&Line>) -> Station {
        let alias = alias_by_station(self.station_cd);
        // 別名が空なら路線の値へ落とし、それも空なら未設定にする。
        // 最後の空文字判定が無いと、路線名を持たないバス路線の駅で
        // null ではなく空文字が出る。
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

/// 路線が存在する駅だけを通す。lines に無い line_cd の駅は結果に出さない。
/// 座標検索がこの条件。
fn joins_line(record: &StationRecord) -> bool {
    line_by_cd(record.line_cd).is_some()
}

/// 名前検索は路線が有効であることも要求する。
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

/// 地球半径 (km)。距離計算と探索範囲の見積もりで同じ値を使う。
const EARTH_RADIUS_KM: f64 = 6371.0;

/// 球面距離 (km)。
///
/// 度単位のユークリッド距離だと緯度と経度を同じスケールで扱うことになり、
/// 東西方向を過大評価する。ここでは実距離で並べる。
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * a.sqrt().clamp(-1.0, 1.0).asin()
}

/// グリッド 1 マスの一辺 (度)。約 5.5km 四方。
/// 細かくするとマスの数 (= 索引の大きさ) が増え、粗くすると 1 マスあたりの
/// 走査件数が増える。近傍バス停の検索 (半径 300m) が 1 マスで収まる大きさ。
const GRID_CELL_DEG: f64 = 0.05;
/// 最初に見る半径 (km)。市街地ならこの範囲で近傍バス停 50 件がそろう。
const INITIAL_SEARCH_RADIUS_KM: f64 = 1.0;
/// 半径の内側で件数が足りなかったときに広げる倍率。
const SEARCH_RADIUS_GROWTH: f64 = 4.0;

fn cell_index(deg: f64) -> i32 {
    (deg / GRID_CELL_DEG).floor() as i32
}

/// 駅を緯度経度のマスへ割り当てた索引。
///
/// 全件走査だと 1 回の近傍検索で駅の総数ぶん距離を計算することになる。
/// `trainRoute` は経路上の駅ごとに近傍バス停を引くため、経路が長いほど
/// (駅数 × 駅総数) で効いていた。マスに区切っておけば探索半径の内側だけで済む。
///
/// 添字は `stations()` のもの。CSR 形式で、`offsets[c]..offsets[c + 1]` が
/// マス c に属する駅の `items` 上の範囲を表す。
struct Grid {
    min_i: i32,
    min_j: i32,
    rows: usize,
    cols: usize,
    offsets: Vec<u32>,
    items: Vec<u32>,
}

impl Grid {
    fn empty() -> Self {
        Grid {
            min_i: 0,
            min_j: 0,
            rows: 0,
            cols: 0,
            offsets: vec![0],
            items: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn build(want: i32) -> Self {
        let members: Vec<u32> = stations()
            .iter()
            .enumerate()
            .filter(|(_, s)| s.e_status == 0 && s.transport_type as i32 == want)
            .map(|(i, _)| i as u32)
            .collect();
        if members.is_empty() {
            return Grid::empty();
        }

        let (mut min_i, mut max_i) = (i32::MAX, i32::MIN);
        let (mut min_j, mut max_j) = (i32::MAX, i32::MIN);
        for &m in &members {
            let s = &stations()[m as usize];
            let (i, j) = (cell_index(s.lat), cell_index(s.lon));
            min_i = min_i.min(i);
            max_i = max_i.max(i);
            min_j = min_j.min(j);
            max_j = max_j.max(j);
        }
        let rows = (max_i - min_i + 1) as usize;
        let cols = (max_j - min_j + 1) as usize;

        // 度数分布 -> 累積和 -> 配置の 3 パスで CSR を組む
        let mut offsets = vec![0u32; rows * cols + 1];
        let cell_of = |s: &StationRecord| -> usize {
            let i = (cell_index(s.lat) - min_i) as usize;
            let j = (cell_index(s.lon) - min_j) as usize;
            i * cols + j
        };
        for &m in &members {
            offsets[cell_of(&stations()[m as usize]) + 1] += 1;
        }
        for c in 0..rows * cols {
            offsets[c + 1] += offsets[c];
        }
        let mut cursor = offsets.clone();
        let mut items = vec![0u32; members.len()];
        for &m in &members {
            let c = cell_of(&stations()[m as usize]);
            items[cursor[c] as usize] = m;
            cursor[c] += 1;
        }

        Grid {
            min_i,
            min_j,
            rows,
            cols,
            offsets,
            items,
        }
    }

    /// 半径 radius_km の円を必ず覆うマスの範囲 (両端を含む) を返す。
    ///
    /// 緯度差だけの距離は `EARTH_RADIUS_KM * Δφ` なので、そこから緯度の幅を出す。
    /// 経度差だけの距離は両端の緯度が高いほど短くなるため、探索帯のうち最も
    /// 極に近い緯度で見積もって幅を広めに取る。
    fn range(&self, lat: f64, lon: f64, radius_km: f64) -> (i32, i32, i32, i32) {
        let dlat_deg = (radius_km / EARTH_RADIUS_KM).to_degrees();
        let cos_phi = (lat.abs() + dlat_deg).min(90.0).to_radians().cos();
        let sin_half = radius_km / (2.0 * EARTH_RADIUS_KM * cos_phi);
        // cos_phi が 0 付近 (極) だと経度は絞れない。そのときは全周を見る。
        let dlon_deg = if cos_phi <= 0.0 || !sin_half.is_finite() || sin_half >= 1.0 {
            180.0
        } else {
            2.0 * sin_half.asin().to_degrees()
        };
        // 日付変更線をまたぐ範囲は 2 本の区間になる。分割して扱う価値がある
        // データ (日本) ではないので、その場合は経度を絞らず全周を見る。
        // 絞り込みを諦めるだけなので取りこぼしは起きない。
        let (j0, j1) = if dlon_deg >= 180.0 || lon - dlon_deg < -180.0 || lon + dlon_deg > 180.0 {
            (i32::MIN, i32::MAX)
        } else {
            (cell_index(lon - dlon_deg), cell_index(lon + dlon_deg))
        };
        (
            cell_index(lat - dlat_deg),
            cell_index(lat + dlat_deg),
            j0,
            j1,
        )
    }

    /// この半径で索引の全域を覆うか。覆っていればこれ以上広げても増えない。
    fn covers_all(&self, lat: f64, lon: f64, radius_km: f64) -> bool {
        let (i0, i1, j0, j1) = self.range(lat, lon, radius_km);
        i0 <= self.min_i
            && i1 >= self.min_i + self.rows as i32 - 1
            && j0 <= self.min_j
            && j1 >= self.min_j + self.cols as i32 - 1
    }

    /// 半径 radius_km の円を覆うマスに属する駅を渡す。円の外の駅も混ざる。
    fn for_each_near(
        &self,
        lat: f64,
        lon: f64,
        radius_km: f64,
        mut f: impl FnMut(&'static StationRecord),
    ) {
        if self.is_empty() {
            return;
        }
        let (i0, i1, j0, j1) = self.range(lat, lon, radius_km);
        let i0 = i0.max(self.min_i);
        let i1 = i1.min(self.min_i + self.rows as i32 - 1);
        let j0 = j0.max(self.min_j);
        let j1 = j1.min(self.min_j + self.cols as i32 - 1);
        for i in i0..=i1 {
            let row = (i - self.min_i) as usize * self.cols;
            for j in j0..=j1 {
                let c = row + (j - self.min_j) as usize;
                for &m in &self.items[self.offsets[c] as usize..self.offsets[c + 1] as usize] {
                    f(&stations()[m as usize]);
                }
            }
        }
    }
}

/// 種別ごとのグリッド。TransportType は 0 = 鉄道 / 1 = バスの 2 値。
static GRIDS: OnceLock<[Grid; 2]> = OnceLock::new();

fn grid_of(want: i32) -> &'static Grid {
    let grids = GRIDS.get_or_init(|| {
        [
            Grid::build(TransportType::Rail as i32),
            Grid::build(TransportType::Bus as i32),
        ]
    });
    match want {
        w if w == TransportType::Bus as i32 => &grids[1],
        w if w == TransportType::Rail as i32 => &grids[0],
        _ => EMPTY_GRID.get_or_init(Grid::empty),
    }
}

static EMPTY_GRID: OnceLock<Grid> = OnceLock::new();

/// 最近傍 limit 件を返す。路線を引けない駅は除く。
///
/// `want` は種別の絞り込み。未指定 (RailAndBus) のときは
/// 鉄道を先・バスを後に並べたうえで距離順になる。
pub fn nearest(
    lat: f64,
    lon: f64,
    limit: usize,
    want: Option<i32>,
) -> Vec<(&'static StationRecord, f64)> {
    let Some(want) = want else {
        // 種別未指定の並びは (transport_type 昇順, 距離昇順)。鉄道が全て
        // バスより前に来るので、鉄道だけで limit 件そろえば以降は見なくてよい。
        let mut out = nearest_of_type(lat, lon, limit, TransportType::Rail as i32);
        if out.len() < limit {
            let rest = limit - out.len();
            out.extend(nearest_of_type(lat, lon, rest, TransportType::Bus as i32));
        }
        return out;
    };
    nearest_of_type(lat, lon, limit, want)
}

/// 距離の昇順、同着なら station_cd の昇順。
///
/// 同じ駅グループの駅は路線ごとに行が分かれるうえ座標を共有するため、距離だけで
/// 並べると同着が多数出る。以前は不安定ソートに任せていたので、どの路線の行が
/// 先に来るかがビルドごとに変わり得た。並びを決め切っておく。
fn by_distance_then_station_cd(
    a: &(&'static StationRecord, f64),
    b: &(&'static StationRecord, f64),
) -> std::cmp::Ordering {
    a.1.partial_cmp(&b.1)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| a.0.station_cd.cmp(&b.0.station_cd))
}

/// 半径 radius_km 以内の駅を距離の昇順で返す。件数の上限は掛けない。
///
/// 「上位 N 件」ではなく半径で切る用途 (駅の近傍バス停) 向け。最寄り N 件を
/// 作ってから半径で捨てると、採用されない駅の分まで組み立てることになる。
/// `nearest` と違い、路線を引けるかどうかは見ない (呼び出し側が
/// 件数を確定させてから路線で絞るため)。
pub fn within_radius(
    lat: f64,
    lon: f64,
    radius_km: f64,
    want: i32,
) -> Vec<(&'static StationRecord, f64)> {
    let mut out: Vec<(&'static StationRecord, f64)> = Vec::new();
    if radius_km < 0.0 {
        return out;
    }
    grid_of(want).for_each_near(lat, lon, radius_km, |record| {
        let distance = haversine_km(lat, lon, record.lat, record.lon);
        if distance <= radius_km {
            out.push((record, distance));
        }
    });
    out.sort_unstable_by(by_distance_then_station_cd);
    out
}

/// 指定した種別の駅から最近傍 limit 件を距離昇順で返す。
///
/// 半径 r の範囲に limit 件そろえば、r より外に上位 limit 件は存在しない。
/// そこでグリッド索引で半径 r の内側だけを見て、足りなければ r を広げる。
/// 索引の外接矩形を覆っても足りなければ、その種別の全件がそろっている。
fn nearest_of_type(
    lat: f64,
    lon: f64,
    limit: usize,
    want: i32,
) -> Vec<(&'static StationRecord, f64)> {
    if limit == 0 {
        return Vec::new();
    }
    let grid = grid_of(want);
    if grid.is_empty() {
        return Vec::new();
    }

    let mut radius_km = INITIAL_SEARCH_RADIUS_KM;
    loop {
        let covers_all = grid.covers_all(lat, lon, radius_km);
        let mut scored: Vec<(&'static StationRecord, f64)> = Vec::new();
        let mut within = 0usize;
        grid.for_each_near(lat, lon, radius_km, |record| {
            if !joins_line(record) {
                return;
            }
            let distance = haversine_km(lat, lon, record.lat, record.lon);
            if distance <= radius_km {
                within += 1;
            }
            scored.push((record, distance));
        });

        // 半径の内側で limit 件そろっていれば、外側を見る必要はない。
        // 覆い切った場合はそれ以上広げても増えないので打ち切る。
        if within >= limit || covers_all {
            if limit < scored.len() {
                scored.select_nth_unstable_by(limit, by_distance_then_station_cd);
                scored.truncate(limit);
            }
            scored.sort_unstable_by(by_distance_then_station_cd);
            return scored;
        }
        radius_km *= SEARCH_RADIUS_GROWTH;
    }
}

/// 駅名・読み・ローマ字・中国語・韓国語のいずれかへの部分一致で引く。
/// 正規化には domain 層の `normalize_for_search` を使う。
///
/// 件数の絞り込みは呼び出し側で行う。出発駅による絞り込みの後に件数を切る
/// 必要があるため、ここで切ると結果が変わる。
pub fn search_by_name(query: &str, want: Option<i32>) -> Vec<&'static StationRecord> {
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

    hits.sort_unstable_by(|a, b| {
        a.station_g_cd
            .cmp(&b.station_g_cd)
            .then_with(|| a.name.cmp(&b.name))
    });
    hits
}

// ---------------------------------------------------------------- 列車種別

const TYPES_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/types.csv"));
/// build.rs が生成した固定長バイナリ (1 行 = i32 x 4, リトルエンディアン)。
/// CSV のままだと 41,250 行のパースがコールドスタートの大半を占めるため。
const SST_BIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sst.bin"));
/// build.rs 側と揃えた欠損値表現
const NULL_I32: i32 = i32::MIN;

/// types.csv の 1 行。`id` は行順に 1 始まりで振る
/// (`TrainType.typeId` として応答に出る)。
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
    /// 系統の優先度。大きいものを優先する。
    pub priority: i32,
}

/// station_station_types.csv の 1 行。
///
/// `id` は停車順序そのものとして使われるため、取り込み順に 1 始まりの
/// 連番を振る。
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

    // 生成物の id は全行に連番で振られている。こちらで行をスキップすると
    // ずれるが、この id は station.type_id として応答に出るため、
    // ずれていたら索引構築を中断する。
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

/// その駅が属する系統のうち、最も id の小さいものを返す。has_train_types に使う。
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
/// 別名は駅ごとに引く。
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
/// 別名があれば路線名 7 列を差し替える。Line 単体で引く API
/// (find_by_id / get_by_ids) は別名を見ないので、駅を伴う経路でのみ適用する。
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

#[cfg(test)]
mod tests {
    use super::*;

    /// グリッド索引の正解となる全件走査。索引を入れる前の実装そのもの。
    fn nearest_by_full_scan(
        lat: f64,
        lon: f64,
        limit: usize,
        want: Option<i32>,
    ) -> Vec<(&'static StationRecord, f64)> {
        let mut scored: Vec<(&StationRecord, f64)> = stations()
            .iter()
            .filter(|s| s.e_status == 0)
            .filter(|s| joins_line(s))
            .filter(|s| want.is_none_or(|w| s.transport_type as i32 == w))
            .map(|s| (s, haversine_km(lat, lon, s.lat, s.lon)))
            .collect();

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
        if limit < scored.len() {
            scored.select_nth_unstable_by(limit, cmp);
            scored.truncate(limit);
        }
        scored.sort_unstable_by(cmp);
        scored
    }

    fn assert_same_as_full_scan(lat: f64, lon: f64, limit: usize, want: Option<i32>) {
        let expected = nearest_by_full_scan(lat, lon, limit, want);
        let actual = nearest(lat, lon, limit, want);
        assert_eq!(
            expected.len(),
            actual.len(),
            "件数が違う ({lat}, {lon}) want={want:?} limit={limit}"
        );
        for (i, (e, a)) in expected.iter().zip(actual.iter()).enumerate() {
            // 距離が同着の駅は全件走査側の並びが決まらないので距離だけを見る
            assert!(
                (e.1 - a.1).abs() < 1e-9,
                "{i} 件目が違う ({lat}, {lon}) want={want:?} limit={limit}: \
                 期待 {} 実際 {}",
                e.1,
                a.1
            );
        }
    }

    /// グリッド索引は全件走査と同じ結果を返す。
    /// 索引の絞り込みが範囲を取りこぼすと最近傍が欠けるため、実データで突き合わせる。
    #[test]
    fn grid_search_matches_full_scan() {
        // 実在の駅の座標を、データの大きさによらず 40 点ほど抜き出す
        let step = (stations().len() / 40).max(1);
        let sampled = stations().iter().step_by(step).map(|s| (s.lat, s.lon));
        // 駅から離れた座標 (海上・国外)、および索引の外側
        let outside = [
            (35.0, 145.0),
            (43.5, 141.0),
            (26.2, 127.7),
            (0.0, 0.0),
            (51.5, -0.1),
            (-33.9, 151.2),
        ];
        for (lat, lon) in sampled.chain(outside) {
            for want in [None, Some(0), Some(1)] {
                for limit in [1usize, 50] {
                    assert_same_as_full_scan(lat, lon, limit, want);
                }
            }
        }
    }

    /// 極や日付変更線の付近でも打ち切れること。
    /// 経度の絞り込みが日付変更線をまたぐ場合、範囲が索引を覆えず
    /// 半径を広げ続ける (無限ループになる) 経路があった。
    #[test]
    fn grid_search_terminates_at_the_poles_and_the_antimeridian() {
        for (lat, lon) in [
            (89.9, 179.9),
            (-89.9, -179.9),
            (89.9, -179.9),
            (-89.9, 179.9),
            (35.0, 179.99),
            (35.0, -179.99),
            (90.0, 0.0),
            (-90.0, 0.0),
        ] {
            for want in [None, Some(0), Some(1)] {
                assert_same_as_full_scan(lat, lon, 5, want);
            }
        }
    }

    /// 半径 0 件要求と、存在しない種別を渡した場合。
    #[test]
    fn grid_search_handles_degenerate_requests() {
        assert!(nearest(35.681382, 139.766084, 0, None).is_empty());
        assert!(nearest(35.681382, 139.766084, 5, Some(99)).is_empty());
    }

    /// 索引が返す距離は haversine_km と一致し、距離の昇順に並ぶ。
    #[test]
    fn grid_search_returns_sorted_distances() {
        let hits = nearest(35.681382, 139.766084, 20, Some(TransportType::Rail as i32));
        assert!(!hits.is_empty());
        for pair in hits.windows(2) {
            assert!(pair[0].1 <= pair[1].1, "距離の昇順になっていない");
        }
        for (record, distance) in &hits {
            let expected = haversine_km(35.681382, 139.766084, record.lat, record.lon);
            assert!((expected - distance).abs() < 1e-9);
        }
    }
}
