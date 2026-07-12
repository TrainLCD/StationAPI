//! Data import module for CSV and GTFS data

use csv::{ReaderBuilder, StringRecord};
use serde::de::{DeserializeOwned, DeserializeSeed, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use sqlx::{Connection, PgConnection};
use stationapi::config::fetch_database_url;
use stationapi::domain::arrival_estimation::haversine_distance;
use stationapi::domain::romaji::{romaji_display_name, strip_macrons, to_fullwidth_katakana};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::path::Path;
use std::{env, fs};
use tracing::{info, warn};
use zip::ZipArchive;

/// Type alias for GTFS trips batch row
type TripBatchRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<i32>,
);

/// Type alias for GTFS stop_times batch row
type StopTimeBatchRow = (
    String,
    Option<String>,
    Option<String>,
    String,
    i32,
    Option<String>,
    Option<i32>,
    Option<i32>,
    Option<f64>,
    Option<i32>,
);

/// 検索性能に直結するインデックス。create_table.sql内のDOブロックは
/// 拡張が使えない環境向けに例外をNOTICEで握り潰すため、作成に失敗しても
/// 起動ログからは分からない(実際に稼働DBでtrigramインデックスだけが
/// 欠落する事例があった)。必要な拡張はcreate_schemaの冒頭で必須として
/// 作成しているので、ここに列挙したインデックスは明示的に作成し直し、
/// 欠落があればERRORログで可視化する。
const PERFORMANCE_INDEXES: &[(&str, &str)] = &[
    (
        "idx_performance_stations_point",
        "CREATE INDEX IF NOT EXISTS idx_performance_stations_point ON public.stations USING gist ((point(lat, lon)))",
    ),
    (
        "idx_performance_stations_bus_point",
        "CREATE INDEX IF NOT EXISTS idx_performance_stations_bus_point ON public.stations USING gist ((point(lat, lon))) WHERE e_status = 0 AND transport_type = 1",
    ),
    (
        "idx_performance_station_name_trgm",
        "CREATE INDEX IF NOT EXISTS idx_performance_station_name_trgm ON public.stations USING gin (station_name gin_trgm_ops)",
    ),
    (
        "idx_performance_station_name_k_trgm",
        "CREATE INDEX IF NOT EXISTS idx_performance_station_name_k_trgm ON public.stations USING gin (station_name_k gin_trgm_ops)",
    ),
    (
        "idx_performance_station_name_rn_trgm",
        "CREATE INDEX IF NOT EXISTS idx_performance_station_name_rn_trgm ON public.stations USING gin (station_name_rn gin_trgm_ops)",
    ),
    (
        "idx_performance_station_name_zh_trgm",
        "CREATE INDEX IF NOT EXISTS idx_performance_station_name_zh_trgm ON public.stations USING gin (station_name_zh gin_trgm_ops)",
    ),
    (
        "idx_performance_station_name_ko_trgm",
        "CREATE INDEX IF NOT EXISTS idx_performance_station_name_ko_trgm ON public.stations USING gin (station_name_ko gin_trgm_ops)",
    ),
    (
        "idx_gtfs_stops_point",
        "CREATE INDEX IF NOT EXISTS idx_gtfs_stops_point ON public.gtfs_stops USING gist ((point(stop_lat, stop_lon)))",
    ),
    (
        "idx_gtfs_stops_name_trgm",
        "CREATE INDEX IF NOT EXISTS idx_gtfs_stops_name_trgm ON public.gtfs_stops USING gin (stop_name gin_trgm_ops)",
    ),
    (
        "idx_gtfs_stops_name_k_trgm",
        "CREATE INDEX IF NOT EXISTS idx_gtfs_stops_name_k_trgm ON public.gtfs_stops USING gin (stop_name_k gin_trgm_ops)",
    ),
];

/// Create required extensions and tables before running data imports.
/// Must be called before `import_csv` and `import_gtfs` can run in parallel.
pub async fn create_schema() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = fetch_database_url();
    let mut conn = PgConnection::connect(&db_url).await?;
    let data_path = Path::new("data");

    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_trgm")
        .execute(&mut conn)
        .await?;

    sqlx::query("CREATE EXTENSION IF NOT EXISTS btree_gist")
        .execute(&mut conn)
        .await?;

    let create_sql_path = data_path.join("create_table.sql");
    let create_sql_content = fs::read(&create_sql_path).map_err(|e| {
        tracing::error!("Failed to read create_table.sql: {}", e);
        Box::new(e) as Box<dyn std::error::Error>
    })?;
    let create_sql: String = String::from_utf8_lossy(&create_sql_content).parse()?;
    sqlx::raw_sql(&create_sql).execute(&mut conn).await?;

    for (name, ddl) in PERFORMANCE_INDEXES {
        if let Err(e) = sqlx::query(ddl).execute(&mut conn).await {
            tracing::error!("Failed to create performance index {}: {}", name, e);
        }
    }

    // 作成結果を検証し、欠落があれば起動ログに残す
    let expected: Vec<String> = PERFORMANCE_INDEXES
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();
    let existing: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT indexname FROM pg_indexes WHERE indexname = ANY($1)",
    )
    .bind(&expected)
    .fetch_all(&mut conn)
    .await?;
    let missing: Vec<&str> = PERFORMANCE_INDEXES
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !existing.iter().any(|e| e == name))
        .collect();
    if missing.is_empty() {
        info!(
            "Schema creation completed. All {} performance indexes are present.",
            expected.len()
        );
    } else {
        tracing::error!(
            "Schema creation completed, but performance indexes are missing: {:?}",
            missing
        );
    }

    Ok(())
}

/// Import CSV data from the data directory.
/// Requires `create_schema` to have been called beforehand.
pub async fn import_csv() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = fetch_database_url();
    let mut conn = PgConnection::connect(&db_url).await?;
    let data_path = Path::new("data");

    let entries = fs::read_dir(data_path).map_err(|e| {
        tracing::error!("Failed to read data directory: {}", e);
        Box::new(e) as Box<dyn std::error::Error>
    })?;

    let mut file_list: Vec<_> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() && path.extension()? == "csv" && path.to_string_lossy().contains('!')
            {
                Some(path.file_name()?.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect();
    file_list.sort();

    for file_name in &file_list {
        let mut rdr = ReaderBuilder::new().from_path(data_path.join(file_name))?;

        let headers_record = rdr.headers()?;
        let headers: Vec<String> = headers_record
            .into_iter()
            .map(|row| row.to_string())
            .collect();

        let mut csv_data: Vec<StringRecord> = Vec::new();
        let records: Vec<StringRecord> = rdr.records().filter_map(|row| row.ok()).collect();
        csv_data.extend(records);

        let table_name = match file_name.split('!').nth(1) {
            Some(part) => match part.split('.').next() {
                Some(name) if !name.is_empty() => name,
                _ => {
                    tracing::warn!("Invalid file name format: {}", file_name);
                    continue;
                }
            },
            None => {
                tracing::warn!("Invalid file name format: {}", file_name);
                continue;
            }
        };

        // Skip empty CSV files to avoid generating invalid INSERT statements
        if csv_data.is_empty() {
            tracing::warn!("Skipping empty CSV file: {}", file_name);
            continue;
        }

        let mut sql_lines_inner = Vec::new();
        sql_lines_inner.push(format!("INSERT INTO public.{table_name} VALUES "));

        for (idx, data) in csv_data.iter().enumerate() {
            let cols: Vec<_> = data
                .iter()
                .enumerate()
                .filter_map(|(col_idx, col)| {
                    if headers
                        .get(col_idx)
                        .unwrap_or(&String::new())
                        .starts_with('#')
                    {
                        return None;
                    }

                    if col.is_empty() {
                        Some("NULL".to_string())
                    } else if col == "DEFAULT" {
                        Some("DEFAULT".to_string())
                    } else {
                        Some(format!("'{}'", escape_sql_string(col)))
                    }
                })
                .collect();

            let values_part = cols.join(",");
            let separator = if idx == csv_data.len() - 1 {
                ");"
            } else {
                "),"
            };
            sql_lines_inner.push(format!("({values_part}{separator}"));
        }

        sqlx::query(&sql_lines_inner.concat())
            .execute(&mut conn)
            .await?;
    }

    sqlx::query("ANALYZE;").execute(&mut conn).await?;

    info!("CSV import completed successfully.");

    Ok(())
}

/// Represents a translation entry from translations.txt
#[derive(Debug, Clone, Default)]
struct Translation {
    ja: Option<String>,      // Japanese (default)
    ja_hrkt: Option<String>, // Hiragana/Katakana
    en: Option<String>,      // English (used for romanized name)
    zh: Option<String>,      // Chinese
    ko: Option<String>,      // Korean
}

#[derive(Clone, Copy)]
struct GtfsFeed {
    id: &'static str,
    name: &'static str,
    path: &'static str,
    url: &'static str,
    requires_consumer_key: bool,
}

const GTFS_FEEDS: &[GtfsFeed] = &[
    GtfsFeed {
        id: "toei",
        name: "Toei Bus",
        path: "data/ToeiBus-GTFS",
        url: "https://api-public.odpt.org/api/v4/files/Toei/data/ToeiBus-GTFS.zip",
        requires_consumer_key: false,
    },
    GtfsFeed {
        id: "seibu",
        name: "Seibu Bus",
        path: "data/SeibuBus-GTFS",
        url: "https://api.odpt.org/api/v4/files/SeibuBus/data/SeibuBus-GTFS.zip",
        requires_consumer_key: true,
    },
    GtfsFeed {
        id: "tokyu_ota",
        name: "Tokyu Bus (Ota City Community Bus)",
        path: "data/TokyuBus-OtaCity-GTFS",
        url: "https://api.odpt.org/api/v4/files/odpt/TokyuBus/tokyubus_community_OtaCity.zip?date=current",
        requires_consumer_key: true,
    },
    GtfsFeed {
        id: "tokyu_shinagawa",
        name: "Tokyu Bus (Shinagawa City Community Bus)",
        path: "data/TokyuBus-ShinagawaCity-GTFS",
        url: "https://api.odpt.org/api/v4/files/odpt/TokyuBus/tokyubus_community_ShinagawaCity.zip?date=current",
        requires_consumer_key: true,
    },
    GtfsFeed {
        id: "tokyu_meguro",
        name: "Tokyu Bus (Meguro City Community Bus)",
        path: "data/TokyuBus-MeguroCity-GTFS",
        url: "https://api.odpt.org/api/v4/files/odpt/TokyuBus/tokyubus_community_MeguroCity.zip?date=current",
        requires_consumer_key: true,
    },
    GtfsFeed {
        // ODPT's versioned files endpoint requires a `date` selector; `current`
        // always resolves to the timetable in effect today (omitting it 404s).
        id: "keio",
        name: "Keio Bus",
        path: "data/KeioBus-GTFS",
        url: "https://api.odpt.org/api/v4/files/odpt/KeioBus/AllLines.zip?date=current",
        requires_consumer_key: true,
    },
];

const DEFAULT_GTFS_BUS_LINE_COLOR: &str = "#1f63c6";
const TOKYU_ODPT_PREFIX: &str = "tokyu_json";
const TOKYU_ODPT_OPERATOR: &str = "odpt.Operator:TokyuBus";
const TOKYU_ODPT_CACHE_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Debug, Deserialize, Serialize)]
struct OdptBusroutePattern {
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
struct OdptBusstopPole {
    #[serde(rename = "owl:sameAs")]
    same_as: String,
    #[serde(rename = "dc:title", default)]
    title: String,
    #[serde(rename = "odpt:kana", default)]
    kana: String,
    #[serde(rename = "odpt:busstopPoleNumber")]
    number: Option<String>,
    #[serde(rename = "odpt:operator", default)]
    operators: Vec<String>,
    #[serde(rename = "geo:lat", default)]
    lat: f64,
    #[serde(rename = "geo:long", default)]
    lon: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct OdptBusTimetable {
    #[serde(rename = "owl:sameAs")]
    same_as: String,
    #[serde(rename = "odpt:operator")]
    operator: String,
    #[serde(rename = "odpt:busroutePattern")]
    busroute_pattern: String,
    #[serde(rename = "odpt:calendar")]
    calendar: String,
    #[serde(rename = "odpt:busTimetableObject", default)]
    objects: Vec<OdptBusTimetableObject>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OdptBusTimetableObject {
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
    #[serde(rename = "odpt:canGetOn")]
    can_get_on: Option<bool>,
    #[serde(rename = "odpt:canGetOff")]
    can_get_off: Option<bool>,
}

struct TokyuOdptData {
    patterns: Vec<OdptBusroutePattern>,
    stops: Vec<OdptBusstopPole>,
    timetables: Vec<OdptBusTimetable>,
}

struct TokyuOnlySeed<T>(std::marker::PhantomData<T>);

impl<'de, T> DeserializeSeed<'de> for TokyuOnlySeed<T>
where
    T: DeserializeOwned,
{
    type Value = Vec<T>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(TokyuOnlyVisitor(std::marker::PhantomData))
    }
}

struct TokyuOnlyVisitor<T>(std::marker::PhantomData<T>);

impl<'de, T> Visitor<'de> for TokyuOnlyVisitor<T>
where
    T: DeserializeOwned,
{
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an ODPT JSON array")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut selected = Vec::new();
        while let Some(value) = sequence.next_element::<serde_json::Value>()? {
            let belongs_to_tokyu = match value.get("odpt:operator") {
                Some(serde_json::Value::String(operator)) => operator == TOKYU_ODPT_OPERATOR,
                Some(serde_json::Value::Array(operators)) => operators
                    .iter()
                    .any(|operator| operator.as_str() == Some(TOKYU_ODPT_OPERATOR)),
                _ => false,
            };
            if belongs_to_tokyu {
                selected.push(serde_json::from_value(value).map_err(|error| {
                    serde::de::Error::custom(format!("invalid Tokyu Bus record: {}", error))
                })?);
            }
        }
        Ok(selected)
    }
}

fn read_tokyu_odpt_items<T>(path: &Path) -> Result<Vec<T>, Box<dyn std::error::Error + Send + Sync>>
where
    T: DeserializeOwned,
{
    read_tokyu_odpt_items_from_reader(BufReader::new(File::open(path)?))
}

fn read_tokyu_odpt_items_from_reader<T, R>(
    reader: R,
) -> Result<Vec<T>, Box<dyn std::error::Error + Send + Sync>>
where
    T: DeserializeOwned,
    R: Read,
{
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    Ok(TokyuOnlySeed(std::marker::PhantomData).deserialize(&mut deserializer)?)
}

fn strip_odpt_prefix(value: &str) -> &str {
    value.split_once(':').map_or(value, |(_, id)| id)
}

fn scoped_tokyu_odpt_id(value: &str) -> String {
    format!("{}:{}", TOKYU_ODPT_PREFIX, strip_odpt_prefix(value))
}

fn odpt_direction_id(direction: Option<&str>) -> Option<i32> {
    match direction {
        Some("1") => Some(0),
        Some("2") => Some(1),
        _ => None,
    }
}

fn tokyu_route_name(title: &str) -> &str {
    title.split_whitespace().next().unwrap_or(title)
}

fn is_tokyu_community_route(busroute: &str) -> bool {
    matches!(
        strip_odpt_prefix(busroute),
        "TokyuBus.Tamachan" | "TokyuBus.Shinabasu" | "TokyuBus.Sanma"
    )
}

fn download_odpt_json<T>(
    client: &reqwest::blocking::Client,
    resource: &str,
    token: &str,
) -> Result<Vec<T>, Box<dyn std::error::Error + Send + Sync>>
where
    T: DeserializeOwned + Serialize,
{
    let cache_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/TokyuBus-ODPT");
    let cache_path = cache_dir.join(format!(
        "{}.tokyu.json",
        resource.rsplit(':').next().unwrap_or(resource)
    ));
    if let Ok(metadata) = fs::metadata(&cache_path) {
        if metadata
            .modified()
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < TOKYU_ODPT_CACHE_MAX_AGE)
        {
            info!("Using cached Tokyu Bus {} JSON.", resource);
            return read_tokyu_odpt_items(&cache_path);
        }
    }

    let url = format!("https://api.odpt.org/api/v4/{}.json", resource);
    let mut response = client
        .get(url)
        .query(&[
            ("odpt:operator", TOKYU_ODPT_OPERATOR),
            ("acl:consumerKey", token),
        ])
        .send()?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to download {}: HTTP {}",
            resource,
            response.status()
        )
        .into());
    }
    fs::create_dir_all(&cache_dir)?;
    let download_path = cache_path.with_extension("download.tmp");
    let mut download_file = BufWriter::new(File::create(&download_path)?);
    std::io::copy(&mut response, &mut download_file)?;
    download_file.flush()?;
    drop(download_file);

    let selected = match read_tokyu_odpt_items(&download_path) {
        Ok(selected) => selected,
        Err(error) => {
            let _ = fs::remove_file(&download_path);
            return Err(error);
        }
    };
    fs::remove_file(&download_path)?;
    let temporary_path = cache_path.with_extension("json.tmp");
    let mut cache_file = BufWriter::new(File::create(&temporary_path)?);
    serde_json::to_writer(&mut cache_file, &selected)?;
    cache_file.flush()?;
    drop(cache_file);
    if cache_path.exists() {
        fs::remove_file(&cache_path)?;
    }
    fs::rename(temporary_path, &cache_path)?;
    Ok(selected)
}

fn download_tokyu_odpt_data() -> Result<TokyuOdptData, Box<dyn std::error::Error + Send + Sync>> {
    let token = env::var("ODPT_ACCESS_TOKEN")
        .map_err(|_| "Tokyu Bus ODPT JSON requires ODPT_ACCESS_TOKEN in the environment")?;
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    Ok(TokyuOdptData {
        patterns: download_odpt_json(&client, "odpt:BusroutePattern", &token)?,
        stops: download_odpt_json(&client, "odpt:BusstopPole", &token)?,
        timetables: download_odpt_json(&client, "odpt:BusTimetable", &token)?,
    })
}

fn scoped_gtfs_id(feed: &GtfsFeed, id: &str) -> String {
    format!("{}:{}", feed.id, id)
}

fn scoped_gtfs_id_opt(feed: &GtfsFeed, id: Option<&str>) -> Option<String> {
    id.filter(|s| !s.is_empty())
        .map(|s| scoped_gtfs_id(feed, s))
}

/// Append the ODPT `acl:consumerKey` query parameter to a feed URL, using the
/// correct separator (`?` for the first parameter, `&` when the URL already
/// carries a query string such as `?date=current`).
fn append_consumer_key(url: &str, token: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{}{}acl:consumerKey={}", url, separator, token)
}

fn gtfs_download_url(feed: &GtfsFeed) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if !feed.requires_consumer_key {
        return Ok(feed.url.to_string());
    }

    let token = env::var("ODPT_ACCESS_TOKEN").map_err(|_| {
        format!(
            "{} GTFS requires ODPT_ACCESS_TOKEN in the environment",
            feed.name
        )
    })?;

    Ok(append_consumer_key(feed.url, &token))
}

/// Download and extract GTFS data from ODPT API.
///
/// If the download or extraction fails partway, the (possibly partial) target
/// directory is removed so a later run does not treat the incomplete extraction
/// as a valid, importable feed.
fn download_gtfs(feed: GtfsFeed) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let gtfs_path = Path::new(feed.path);

    // Skip if directory already exists
    if gtfs_path.exists() {
        info!(
            "{} GTFS directory already exists, skipping download.",
            feed.name
        );
        return Ok(());
    }

    match download_and_extract_gtfs(&feed, gtfs_path) {
        Ok(()) => Ok(()),
        Err(e) => {
            if gtfs_path.exists() {
                if let Err(cleanup_err) = fs::remove_dir_all(gtfs_path) {
                    warn!(
                        "Failed to remove partial {} GTFS directory after error: {}",
                        feed.name, cleanup_err
                    );
                }
            }
            Err(e)
        }
    }
}

fn download_and_extract_gtfs(
    feed: &GtfsFeed,
    gtfs_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Downloading {} GTFS data from ODPT API...", feed.name);

    // Download the ZIP file. `without_url()` strips the request URL (which carries
    // the `acl:consumerKey` token) from any transport error so the token cannot
    // leak into logs or propagated error messages.
    let request_start = std::time::Instant::now();
    let response = reqwest::blocking::get(gtfs_download_url(feed)?).map_err(|e| e.without_url())?;
    info!(
        "[gtfs-download:{}] response received in {:?} (status={})",
        feed.id,
        request_start.elapsed(),
        response.status()
    );

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download {} GTFS: HTTP {}",
            feed.name,
            response.status()
        )
        .into());
    }

    let body_start = std::time::Instant::now();
    let bytes = response.bytes().map_err(|e| e.without_url())?;
    info!(
        "[gtfs-download:{}] body read in {:?} ({} bytes), extracting...",
        feed.id,
        body_start.elapsed(),
        bytes.len()
    );

    // Create the target directory
    fs::create_dir_all(gtfs_path)?;

    // Extract the ZIP file
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_name = match file.enclosed_name() {
            Some(name) => name.to_owned(),
            None => continue,
        };

        // Skip directories and hidden files
        if file.is_dir() || file_name.to_string_lossy().starts_with('.') {
            continue;
        }

        // Get just the file name (strip any directory prefix from ZIP)
        let output_name = file_name
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| file_name.to_string_lossy().to_string());

        let output_path = gtfs_path.join(&output_name);

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        fs::write(&output_path, &contents)?;

        info!("Extracted {}: {}", feed.name, output_name);
    }

    info!("{} GTFS extraction completed.", feed.name);
    Ok(())
}

/// Import GTFS data from ToeiBus-GTFS directory
/// All imports are wrapped in a transaction - if any step fails, all changes are rolled back
pub async fn import_gtfs() -> Result<(), Box<dyn std::error::Error>> {
    let total_start = std::time::Instant::now();
    info!("[gtfs] entering import_gtfs");

    // Check if bus feature is disabled
    if is_bus_feature_disabled() {
        info!("Bus feature is disabled, skipping GTFS import.");
        return Ok(());
    }
    info!("[gtfs] bus feature enabled, starting download/extract");
    let enabled_feeds = GTFS_FEEDS.to_vec();
    info!(
        "[gtfs] enabled feeds: {}",
        enabled_feeds
            .iter()
            .map(|feed| feed.name)
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Download GTFS data if not present (use spawn_blocking to avoid blocking async runtime).
    // A single feed's download failure (e.g. an operator's ODPT outage or a moved URL)
    // must not abort the whole pipeline: log it and continue so the other feeds still
    // import. The per-feed import loop below skips any feed whose directory is missing.
    let download_start = std::time::Instant::now();
    for feed in enabled_feeds.iter().copied() {
        match tokio::task::spawn_blocking(move || download_gtfs(feed)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                warn!(
                    "Failed to download {} GTFS: {}. Skipping this feed; other feeds continue.",
                    feed.name, e
                );
            }
            // A panic or cancellation of the blocking task must not abort the whole
            // pipeline either: log it and let the remaining feeds proceed.
            Err(join_err) => {
                warn!(
                    "{} GTFS download task did not complete ({}). Skipping this feed; other feeds continue.",
                    feed.name, join_err
                );
            }
        }
    }
    let tokyu_odpt_data = tokio::task::spawn_blocking(download_tokyu_odpt_data)
        .await
        .map_err(|e| format!("Failed to spawn Tokyu Bus JSON download: {}", e))?
        .map_err(|e| -> Box<dyn std::error::Error> { e })?;
    info!(
        "[gtfs:{}] downloaded ODPT JSON: {} patterns, {} stops, {} timetables",
        TOKYU_ODPT_PREFIX,
        tokyu_odpt_data.patterns.len(),
        tokyu_odpt_data.stops.len(),
        tokyu_odpt_data.timetables.len()
    );
    info!(
        "[gtfs] download/extract finished in {:?}",
        download_start.elapsed()
    );

    info!("[gtfs] connecting to database");
    let connect_start = std::time::Instant::now();
    let db_url = fetch_database_url();
    let mut conn = PgConnection::connect(&db_url).await?;
    info!("[gtfs] connected in {:?}", connect_start.elapsed());

    info!("Starting GTFS import from configured feeds (using transaction)...");

    // Begin transaction - all changes will be rolled back if any step fails
    let mut tx = conn.begin().await?;
    info!("[gtfs] transaction begun, clearing existing data");

    // First, clear existing GTFS data (in reverse order of dependencies)
    let clear_start = std::time::Instant::now();
    sqlx::query("DELETE FROM gtfs_stop_times")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM gtfs_trips")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM gtfs_shapes")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM gtfs_calendar_dates")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM gtfs_calendar")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM gtfs_stops")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM gtfs_routes")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM gtfs_agencies")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM gtfs_feed_info")
        .execute(&mut *tx)
        .await?;
    info!(
        "[gtfs] cleared existing data in {:?}",
        clear_start.elapsed()
    );

    for feed in &enabled_feeds {
        let gtfs_path = Path::new(feed.path);
        if !gtfs_path.exists() {
            info!(
                "{} GTFS directory not found, skipping this feed.",
                feed.name
            );
            continue;
        }

        info!("[gtfs:{}] importing from {:?}", feed.id, gtfs_path);

        // Load translations for multi-language support (before per-feed inserts).
        let translations_start = std::time::Instant::now();
        let translations = load_gtfs_translations(gtfs_path)?;
        info!(
            "[gtfs:{}] loaded {} translation entries in {:?}",
            feed.id,
            translations.len(),
            translations_start.elapsed()
        );

        let step_start = std::time::Instant::now();
        import_gtfs_agencies(&mut tx, gtfs_path, feed).await?;
        info!(
            "[gtfs:{}] agencies imported in {:?}",
            feed.id,
            step_start.elapsed()
        );

        let step_start = std::time::Instant::now();
        import_gtfs_routes(&mut tx, gtfs_path, feed).await?;
        info!(
            "[gtfs:{}] routes imported in {:?}",
            feed.id,
            step_start.elapsed()
        );

        let step_start = std::time::Instant::now();
        import_gtfs_stops(&mut tx, gtfs_path, feed, &translations).await?;
        info!(
            "[gtfs:{}] stops imported in {:?}",
            feed.id,
            step_start.elapsed()
        );

        let step_start = std::time::Instant::now();
        import_gtfs_calendar(&mut tx, gtfs_path, feed).await?;
        info!(
            "[gtfs:{}] calendar imported in {:?}",
            feed.id,
            step_start.elapsed()
        );

        let step_start = std::time::Instant::now();
        import_gtfs_calendar_dates(&mut tx, gtfs_path, feed).await?;
        info!(
            "[gtfs:{}] calendar_dates imported in {:?}",
            feed.id,
            step_start.elapsed()
        );

        let step_start = std::time::Instant::now();
        import_gtfs_shapes(&mut tx, gtfs_path, feed).await?;
        info!(
            "[gtfs:{}] shapes imported in {:?}",
            feed.id,
            step_start.elapsed()
        );

        let step_start = std::time::Instant::now();
        import_gtfs_trips(&mut tx, gtfs_path, feed).await?;
        info!(
            "[gtfs:{}] trips imported in {:?}",
            feed.id,
            step_start.elapsed()
        );

        let step_start = std::time::Instant::now();
        import_gtfs_stop_times(&mut tx, gtfs_path, feed).await?;
        info!(
            "[gtfs:{}] stop_times imported in {:?}",
            feed.id,
            step_start.elapsed()
        );

        let step_start = std::time::Instant::now();
        import_gtfs_feed_info(&mut tx, gtfs_path, feed).await?;
        info!(
            "[gtfs:{}] feed_info imported in {:?}",
            feed.id,
            step_start.elapsed()
        );
    }

    import_tokyu_odpt_data(&mut tx, &tokyu_odpt_data).await?;

    info!("[gtfs] committing transaction");
    let commit_start = std::time::Instant::now();
    tx.commit().await?;
    info!(
        "[gtfs] transaction committed in {:?}",
        commit_start.elapsed()
    );

    info!(
        "GTFS import completed successfully (transaction committed). total={:?}",
        total_start.elapsed()
    );

    Ok(())
}

async fn import_tokyu_odpt_data(
    conn: &mut PgConnection,
    data: &TokyuOdptData,
) -> Result<(), Box<dyn std::error::Error>> {
    let agency_id = format!("{}:TokyuBus", TOKYU_ODPT_PREFIX);
    sqlx::query(
        r#"INSERT INTO gtfs_agencies
           (agency_id, agency_name, agency_name_k, agency_name_r, agency_url,
            agency_timezone, agency_lang, company_cd)
           VALUES ($1, '東急バス', 'トウキュウバス', 'Tokyu Bus',
                   'https://www.tokyubus.co.jp/', 'Asia/Tokyo', 'ja', 255)
           ON CONFLICT (agency_id) DO NOTHING"#,
    )
    .bind(&agency_id)
    .execute(&mut *conn)
    .await?;

    let mut pattern_map: HashMap<String, (String, Option<i32>)> = HashMap::new();
    let mut inserted_routes = HashSet::new();
    for pattern in data.patterns.iter().filter(|pattern| {
        pattern.operator == TOKYU_ODPT_OPERATOR && !is_tokyu_community_route(&pattern.busroute)
    }) {
        let route_id = scoped_tokyu_odpt_id(&pattern.busroute);
        let pattern_id = scoped_tokyu_odpt_id(&pattern.same_as);
        pattern_map.insert(
            pattern_id,
            (
                route_id.clone(),
                odpt_direction_id(pattern.direction.as_deref()),
            ),
        );

        if inserted_routes.insert(route_id.clone()) {
            let route_name = tokyu_route_name(&pattern.title);
            sqlx::query(
                r#"INSERT INTO gtfs_routes
                   (route_id, agency_id, route_short_name, route_long_name,
                    route_long_name_r, route_type, route_color)
                   VALUES ($1, $2, $3, $3, NULL, 3, 'DD1133')
                   ON CONFLICT (route_id) DO NOTHING"#,
            )
            .bind(&route_id)
            .bind(&agency_id)
            .bind(route_name)
            .execute(&mut *conn)
            .await?;
        }
    }

    let mut stop_ids = HashSet::new();
    let mut stop_values = Vec::new();
    let mut missing_coordinates = 0_usize;
    for stop in data.stops.iter().filter(|stop| {
        stop.operators
            .iter()
            .any(|operator| operator == TOKYU_ODPT_OPERATOR)
    }) {
        let stop_id = scoped_tokyu_odpt_id(&stop.same_as);
        stop_ids.insert(stop_id.clone());
        if stop.lat == 0.0 || stop.lon == 0.0 {
            missing_coordinates += 1;
        }
        // Tokyu Bus ODPT JSON has no English field, so romanize the kana reading
        // into a Hepburn stop_name_r (NULL when the reading has no convertible
        // kana). This mirrors the GTFS `en` fallback and feeds the same
        // English-facing station/search/route surfaces.
        let katakana = hiragana_to_katakana(&stop.kana);
        let stop_name_r = romaji_display_name(&katakana)
            .map(|r| format!("'{}'", escape_sql_string(&r)))
            .unwrap_or_else(|| "NULL".to_string());
        stop_values.push(format!(
            "('{}', '{}', '{}', '{}', {}, {}, {})",
            escape_sql_string(&stop_id),
            escape_sql_string(stop.number.as_deref().unwrap_or("")),
            escape_sql_string(&stop.title),
            escape_sql_string(&katakana),
            stop_name_r,
            stop.lat,
            stop.lon
        ));
        if stop_values.len() == 500 {
            insert_tokyu_stop_values(&mut *conn, &stop_values).await?;
            stop_values.clear();
        }
    }
    insert_tokyu_stop_values(&mut *conn, &stop_values).await?;
    if missing_coordinates > 0 {
        warn!(
            "Tokyu Bus ODPT JSON contains {} stops without coordinates; name/route queries work, but coordinate queries cannot return those stops",
            missing_coordinates
        );
    }

    let mut trips = Vec::with_capacity(2_000);
    let mut stop_times = Vec::with_capacity(1_000);
    let mut trip_count = 0_usize;
    let mut stop_time_count = 0_usize;
    for timetable in data
        .timetables
        .iter()
        .filter(|timetable| timetable.operator == TOKYU_ODPT_OPERATOR)
    {
        let pattern_id = scoped_tokyu_odpt_id(&timetable.busroute_pattern);
        let Some((route_id, direction_id)) = pattern_map.get(&pattern_id) else {
            continue;
        };
        let trip_id = scoped_tokyu_odpt_id(&timetable.same_as);
        let service_id = scoped_tokyu_odpt_id(&timetable.calendar);
        let headsign = timetable
            .objects
            .iter()
            .find_map(|object| object.destination_sign.clone());
        trips.push((
            trip_id.clone(),
            route_id.clone(),
            service_id,
            headsign,
            None,
            *direction_id,
            None,
            Some(pattern_id),
            None,
            None,
        ));
        trip_count += 1;
        if trips.len() == 2_000 {
            insert_trips_batch(&mut *conn, &trips).await?;
            trips.clear();
        }

        for object in &timetable.objects {
            let stop_id = scoped_tokyu_odpt_id(&object.busstop_pole);
            if !stop_ids.contains(&stop_id) {
                continue;
            }
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
            stop_times.push((
                trip_id.clone(),
                arrival,
                departure,
                stop_id,
                object.index,
                object.destination_sign.clone(),
                object.can_get_on.map(|allowed| if allowed { 0 } else { 1 }),
                object
                    .can_get_off
                    .map(|allowed| if allowed { 0 } else { 1 }),
                None,
                None,
            ));
            stop_time_count += 1;
            if stop_times.len() == 1_000 {
                // stop_times has a foreign key to trips. Flush every pending trip
                // before its stop-time batch, even when the trip batch is not full.
                insert_trips_batch(&mut *conn, &trips).await?;
                trips.clear();
                insert_stop_times_batch(&mut *conn, &stop_times).await?;
                stop_times.clear();
            }
        }
    }

    insert_trips_batch(&mut *conn, &trips).await?;
    insert_stop_times_batch(&mut *conn, &stop_times).await?;

    info!(
        "Imported Tokyu Bus ODPT JSON as GTFS: {} routes, {} stops, {} trips, {} stop times.",
        inserted_routes.len(),
        stop_ids.len(),
        trip_count,
        stop_time_count
    );
    Ok(())
}

async fn insert_tokyu_stop_values(
    conn: &mut PgConnection,
    values: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if values.is_empty() {
        return Ok(());
    }
    let sql = format!(
        "INSERT INTO gtfs_stops (stop_id, stop_code, stop_name, stop_name_k, stop_name_r, stop_lat, stop_lon) VALUES {} ON CONFLICT (stop_id) DO NOTHING",
        values.join(",")
    );
    sqlx::query(&sql).execute(&mut *conn).await?;
    Ok(())
}

/// Load translations from translations.txt
fn load_gtfs_translations(
    gtfs_path: &Path,
) -> Result<HashMap<(String, String), Translation>, Box<dyn std::error::Error>> {
    let translations_path = gtfs_path.join("translations.txt");
    let mut translations: HashMap<(String, String), Translation> = HashMap::new();

    if !translations_path.exists() {
        return Ok(translations);
    }

    let mut rdr = ReaderBuilder::new().from_path(&translations_path)?;

    // The GTFS-JP translations.txt column layout differs between feeds: Seibu
    // omits `record_sub_id` (6 columns), while Keio and the Tokyu community feeds
    // include it (7 columns). Resolve every column by header name so positions
    // stay correct regardless of layout.
    let headers = rdr.headers()?.clone();
    let column = |name: &str| headers.iter().position(|h| h == name);
    let (Some(table_idx), Some(field_idx), Some(lang_idx), Some(text_idx)) = (
        column("table_name"),
        column("field_name"),
        column("language"),
        column("translation"),
    ) else {
        warn!("translations.txt missing required columns; skipping translations.");
        return Ok(translations);
    };
    let record_id_idx = column("record_id");
    let field_value_idx = column("field_value");

    for result in rdr.records() {
        let record = result?;
        let cell = |idx: Option<usize>| idx.and_then(|i| record.get(i)).unwrap_or("");

        // Only process stop_name translations for now
        if record.get(table_idx) != Some("stops") || record.get(field_idx) != Some("stop_name") {
            continue;
        }

        let language = record.get(lang_idx).unwrap_or("");
        let text = record.get(text_idx).unwrap_or("");
        let record_id = cell(record_id_idx);
        let field_value = cell(field_value_idx);

        // Feeds key stop_name translations either by `record_id` (Seibu — the
        // stop_id, sometimes with a "-01" pole suffix) or by `field_value` (Keio,
        // Tokyu community — the Japanese stop_name itself, with record_id left
        // empty). Index whichever the row provides so the stop importer can look
        // the translation up by stop_id or by name.
        if !record_id.is_empty() {
            set_translation_language(&mut translations, record_id, language, text, true);
            // Parent stop_id (drop the "-NN" pole suffix); first child wins.
            if let Some(parent_id) = record_id.rfind('-').map(|pos| &record_id[..pos]) {
                set_translation_language(&mut translations, parent_id, language, text, false);
            }
        }
        if !field_value.is_empty() {
            set_translation_language(&mut translations, field_value, language, text, true);
        }
    }

    Ok(translations)
}

/// Set one language field on the `("stops", key)` translation entry. When
/// `overwrite` is false the value is only filled if currently absent (used for
/// parent-stop aggregation where the first child wins).
fn set_translation_language(
    translations: &mut HashMap<(String, String), Translation>,
    key: &str,
    language: &str,
    text: &str,
    overwrite: bool,
) {
    let entry = translations
        .entry(("stops".to_string(), key.to_string()))
        .or_default();
    let slot = match language {
        "ja" => &mut entry.ja,
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

/// Import agencies from agency.txt
async fn import_gtfs_agencies(
    conn: &mut PgConnection,
    gtfs_path: &Path,
    feed: &GtfsFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    let agency_path = gtfs_path.join("agency.txt");
    if !agency_path.exists() {
        warn!("agency.txt not found, skipping agency import.");
        return Ok(());
    }

    let mut rdr = ReaderBuilder::new().from_path(&agency_path)?;

    for result in rdr.records() {
        let record = result?;
        // agency_id,agency_name,agency_url,agency_timezone,agency_lang,agency_phone,agency_fare_url,agency_email
        let agency_id = scoped_gtfs_id(feed, record.get(0).unwrap_or(""));
        let agency_name = record.get(1).unwrap_or("");
        let agency_url = record.get(2).filter(|s| !s.is_empty());
        let agency_timezone = record.get(3).unwrap_or("Asia/Tokyo");
        let agency_lang = record.get(4).filter(|s| !s.is_empty());
        let agency_phone = record.get(5).filter(|s| !s.is_empty());
        let agency_fare_url = record.get(6).filter(|s| !s.is_empty());

        sqlx::query(
            r#"INSERT INTO gtfs_agencies
               (agency_id, agency_name, agency_url, agency_timezone, agency_lang, agency_phone, agency_fare_url)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (agency_id) DO NOTHING"#,
        )
        .bind(&agency_id)
        .bind(agency_name)
        .bind(agency_url)
        .bind(agency_timezone)
        .bind(agency_lang)
        .bind(agency_phone)
        .bind(agency_fare_url)
        .execute(&mut *conn)
        .await?;
    }

    info!("Imported agencies.");
    Ok(())
}

/// Import routes from routes.txt
async fn import_gtfs_routes(
    conn: &mut PgConnection,
    gtfs_path: &Path,
    feed: &GtfsFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    let routes_path = gtfs_path.join("routes.txt");
    if !routes_path.exists() {
        warn!("routes.txt not found, skipping routes import.");
        return Ok(());
    }

    let mut rdr = ReaderBuilder::new().from_path(&routes_path)?;

    for result in rdr.records() {
        let record = result?;
        // route_id,agency_id,route_short_name,route_long_name,route_desc,route_type,route_url,route_color,route_text_color,jp_parent_route_id
        let route_id = scoped_gtfs_id(feed, record.get(0).unwrap_or(""));
        let agency_id = scoped_gtfs_id_opt(feed, record.get(1));
        let route_short_name = record.get(2).filter(|s| !s.is_empty());
        let route_long_name = record.get(3).filter(|s| !s.is_empty());
        let route_desc = record.get(4).filter(|s| !s.is_empty());
        let route_type: i32 = record.get(5).unwrap_or("3").parse().unwrap_or(3);
        let route_url = record.get(6).filter(|s| !s.is_empty());
        let route_color = record.get(7).filter(|s| !s.is_empty());
        let route_text_color = record.get(8).filter(|s| !s.is_empty());

        sqlx::query(
            r#"INSERT INTO gtfs_routes
               (route_id, agency_id, route_short_name, route_long_name, route_desc, route_type, route_url, route_color, route_text_color)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               ON CONFLICT (route_id) DO NOTHING"#,
        )
        .bind(&route_id)
        .bind(agency_id.as_deref())
        .bind(route_short_name)
        .bind(route_long_name)
        .bind(route_desc)
        .bind(route_type)
        .bind(route_url)
        .bind(route_color)
        .bind(route_text_color)
        .execute(&mut *conn)
        .await?;
    }

    info!("Imported routes.");
    Ok(())
}

/// Type alias for GTFS stops batch row
type StopBatchRow = (
    String,         // stop_id
    Option<String>, // stop_code
    String,         // stop_name
    Option<String>, // stop_name_k
    Option<String>, // stop_name_r
    Option<String>, // stop_name_zh
    Option<String>, // stop_name_ko
    Option<String>, // stop_desc
    f64,            // stop_lat
    f64,            // stop_lon
    Option<String>, // zone_id
    Option<String>, // stop_url
    i32,            // location_type
    Option<String>, // parent_station
    Option<String>, // stop_timezone
    Option<i32>,    // wheelchair_boarding
    Option<String>, // platform_code
);

/// Import stops from stops.txt with translations
async fn import_gtfs_stops(
    conn: &mut PgConnection,
    gtfs_path: &Path,
    feed: &GtfsFeed,
    translations: &HashMap<(String, String), Translation>,
) -> Result<(), Box<dyn std::error::Error>> {
    let stops_path = gtfs_path.join("stops.txt");
    if !stops_path.exists() {
        warn!("stops.txt not found, skipping stops import.");
        return Ok(());
    }

    let mut rdr = ReaderBuilder::new().from_path(&stops_path)?;
    let mut batch: Vec<StopBatchRow> = Vec::new();
    let batch_size = 500;
    let mut count = 0;

    for result in rdr.records() {
        let record = result?;
        let original_stop_id = record.get(0).unwrap_or("");
        let stop_id = scoped_gtfs_id(feed, original_stop_id);
        let stop_code = record
            .get(1)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let stop_name = record.get(2).unwrap_or("").to_string();
        let stop_desc = record
            .get(3)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let stop_lat: f64 = record.get(4).unwrap_or("0").parse().unwrap_or(0.0);
        let stop_lon: f64 = record.get(5).unwrap_or("0").parse().unwrap_or(0.0);
        let zone_id = record
            .get(6)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let stop_url = record
            .get(7)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let location_type: i32 = record.get(8).unwrap_or("0").parse().unwrap_or(0);
        let parent_station = record
            .get(9)
            .filter(|s| !s.is_empty())
            .map(|s| scoped_gtfs_id(feed, s));
        let stop_timezone = record
            .get(10)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let wheelchair_boarding: Option<i32> = record
            .get(11)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok());
        let platform_code = record
            .get(12)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        // Get translations. Feeds key stop_name translations by either record_id
        // (== stop_id) or field_value (== the Japanese stop_name), so try both.
        let translation = translations
            .get(&("stops".to_string(), original_stop_id.to_string()))
            .or_else(|| translations.get(&("stops".to_string(), stop_name.clone())));

        // Some feeds (Keio, Tokyu community) provide ja-Hrkt readings as
        // half-width katakana, so normalize to full-width for the katakana column.
        let stop_name_k = translation
            .and_then(|t| t.ja_hrkt.clone())
            .map(|k| to_fullwidth_katakana(&k));
        // Prefer the official English translation, but fall back to a Hepburn
        // romanization of the kana reading when the feed ships no `en` row
        // (e.g. Tokyu community, or any en-less stop). This keeps every
        // English-facing surface — station name, name search, and romanized
        // route/headsign names — populated.
        let stop_name_r = translation
            .and_then(|t| t.en.clone())
            .filter(|s| !s.is_empty())
            .or_else(|| stop_name_k.as_deref().and_then(romaji_display_name));
        let stop_name_zh = translation.and_then(|t| t.zh.clone());
        let stop_name_ko = translation.and_then(|t| t.ko.clone());

        batch.push((
            stop_id,
            stop_code,
            stop_name,
            stop_name_k,
            stop_name_r,
            stop_name_zh,
            stop_name_ko,
            stop_desc,
            stop_lat,
            stop_lon,
            zone_id,
            stop_url,
            location_type,
            parent_station,
            stop_timezone,
            wheelchair_boarding,
            platform_code,
        ));

        if batch.len() >= batch_size {
            insert_stops_batch(&mut *conn, &batch).await?;
            count += batch.len();
            batch.clear();
        }
    }

    // Insert remaining
    if !batch.is_empty() {
        insert_stops_batch(&mut *conn, &batch).await?;
        count += batch.len();
    }

    info!("Imported {} stops.", count);
    Ok(())
}

async fn insert_stops_batch(
    conn: &mut PgConnection,
    batch: &[StopBatchRow],
) -> Result<(), Box<dyn std::error::Error>> {
    if batch.is_empty() {
        return Ok(());
    }

    let mut sql = String::from(
        "INSERT INTO gtfs_stops (stop_id, stop_code, stop_name, stop_name_k, stop_name_r, stop_name_zh, stop_name_ko, stop_desc, stop_lat, stop_lon, zone_id, stop_url, location_type, parent_station, stop_timezone, wheelchair_boarding, platform_code) VALUES ",
    );
    let mut values: Vec<String> = Vec::new();

    for (
        stop_id,
        stop_code,
        stop_name,
        stop_name_k,
        stop_name_r,
        stop_name_zh,
        stop_name_ko,
        stop_desc,
        stop_lat,
        stop_lon,
        zone_id,
        stop_url,
        location_type,
        parent_station,
        stop_timezone,
        wheelchair_boarding,
        platform_code,
    ) in batch
    {
        let opt_str = |o: &Option<String>| {
            o.as_ref()
                .map(|s| format!("'{}'", escape_sql_string(s)))
                .unwrap_or_else(|| "NULL".to_string())
        };
        let opt_int = |o: &Option<i32>| {
            o.map(|v| v.to_string())
                .unwrap_or_else(|| "NULL".to_string())
        };

        values.push(format!(
            "('{}', {}, '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
            escape_sql_string(stop_id),
            opt_str(stop_code),
            escape_sql_string(stop_name),
            opt_str(stop_name_k),
            opt_str(stop_name_r),
            opt_str(stop_name_zh),
            opt_str(stop_name_ko),
            opt_str(stop_desc),
            stop_lat,
            stop_lon,
            opt_str(zone_id),
            opt_str(stop_url),
            location_type,
            opt_str(parent_station),
            opt_str(stop_timezone),
            opt_int(wheelchair_boarding),
            opt_str(platform_code),
        ));
    }

    sql.push_str(&values.join(","));
    sql.push_str(" ON CONFLICT (stop_id) DO NOTHING");
    sqlx::query(&sql).execute(&mut *conn).await?;

    Ok(())
}

/// Import calendar from calendar.txt
async fn import_gtfs_calendar(
    conn: &mut PgConnection,
    gtfs_path: &Path,
    feed: &GtfsFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    let calendar_path = gtfs_path.join("calendar.txt");
    if !calendar_path.exists() {
        warn!("calendar.txt not found, skipping calendar import.");
        return Ok(());
    }

    let mut rdr = ReaderBuilder::new().from_path(&calendar_path)?;

    for result in rdr.records() {
        let record = result?;
        // service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date
        let service_id = scoped_gtfs_id(feed, record.get(0).unwrap_or(""));
        let monday: bool = record.get(1).unwrap_or("0") == "1";
        let tuesday: bool = record.get(2).unwrap_or("0") == "1";
        let wednesday: bool = record.get(3).unwrap_or("0") == "1";
        let thursday: bool = record.get(4).unwrap_or("0") == "1";
        let friday: bool = record.get(5).unwrap_or("0") == "1";
        let saturday: bool = record.get(6).unwrap_or("0") == "1";
        let sunday: bool = record.get(7).unwrap_or("0") == "1";
        let start_date = record.get(8).unwrap_or("");
        let end_date = record.get(9).unwrap_or("");

        // Parse dates (format: YYYYMMDD)
        let start_date = chrono::NaiveDate::parse_from_str(start_date, "%Y%m%d")?;
        let end_date = chrono::NaiveDate::parse_from_str(end_date, "%Y%m%d")?;

        sqlx::query(
            r#"INSERT INTO gtfs_calendar
               (service_id, monday, tuesday, wednesday, thursday, friday, saturday, sunday, start_date, end_date)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               ON CONFLICT (service_id) DO NOTHING"#,
        )
        .bind(&service_id)
        .bind(monday)
        .bind(tuesday)
        .bind(wednesday)
        .bind(thursday)
        .bind(friday)
        .bind(saturday)
        .bind(sunday)
        .bind(start_date)
        .bind(end_date)
        .execute(&mut *conn)
        .await?;
    }

    info!("Imported calendar.");
    Ok(())
}

/// Import calendar_dates from calendar_dates.txt
async fn import_gtfs_calendar_dates(
    conn: &mut PgConnection,
    gtfs_path: &Path,
    feed: &GtfsFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    let calendar_dates_path = gtfs_path.join("calendar_dates.txt");
    if !calendar_dates_path.exists() {
        warn!("calendar_dates.txt not found, skipping calendar_dates import.");
        return Ok(());
    }

    let mut rdr = ReaderBuilder::new().from_path(&calendar_dates_path)?;
    let mut batch: Vec<(String, String, i32)> = Vec::new();
    let batch_size = 1000;
    let mut count = 0;

    for result in rdr.records() {
        let record = result?;
        let service_id = scoped_gtfs_id(feed, record.get(0).unwrap_or(""));
        let date = record.get(1).unwrap_or("").to_string();
        let exception_type: i32 = record.get(2).unwrap_or("1").parse().unwrap_or(1);

        batch.push((service_id, date, exception_type));

        if batch.len() >= batch_size {
            insert_calendar_dates_batch(&mut *conn, &batch).await?;
            count += batch.len();
            batch.clear();
        }
    }

    if !batch.is_empty() {
        insert_calendar_dates_batch(&mut *conn, &batch).await?;
        count += batch.len();
    }

    info!("Imported {} calendar_dates.", count);
    Ok(())
}

async fn insert_calendar_dates_batch(
    conn: &mut PgConnection,
    batch: &[(String, String, i32)],
) -> Result<(), Box<dyn std::error::Error>> {
    if batch.is_empty() {
        return Ok(());
    }

    let mut sql =
        String::from("INSERT INTO gtfs_calendar_dates (service_id, date, exception_type) VALUES ");
    let mut values: Vec<String> = Vec::new();

    for (service_id, date, exception_type) in batch {
        // Parse and format date
        let parsed_date = chrono::NaiveDate::parse_from_str(date, "%Y%m%d")?;
        values.push(format!(
            "('{}', '{}', {})",
            escape_sql_string(service_id),
            parsed_date,
            exception_type
        ));
    }

    sql.push_str(&values.join(","));
    sqlx::query(&sql).execute(&mut *conn).await?;

    Ok(())
}

/// Import shapes from shapes.txt
async fn import_gtfs_shapes(
    conn: &mut PgConnection,
    gtfs_path: &Path,
    feed: &GtfsFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    let shapes_path = gtfs_path.join("shapes.txt");
    if !shapes_path.exists() {
        warn!("shapes.txt not found, skipping shapes import.");
        return Ok(());
    }

    let mut rdr = ReaderBuilder::new().from_path(&shapes_path)?;
    let mut batch: Vec<(String, f64, f64, i32, Option<f64>)> = Vec::new();
    let batch_size = 2000;

    for result in rdr.records() {
        let record = result?;
        // shape_id,shape_pt_lat,shape_pt_lon,shape_pt_sequence,shape_dist_traveled
        let shape_id = scoped_gtfs_id(feed, record.get(0).unwrap_or(""));
        let shape_pt_lat: f64 = record.get(1).unwrap_or("0").parse().unwrap_or(0.0);
        let shape_pt_lon: f64 = record.get(2).unwrap_or("0").parse().unwrap_or(0.0);
        let shape_pt_sequence: i32 = record.get(3).unwrap_or("0").parse().unwrap_or(0);
        let shape_dist_traveled: Option<f64> = record
            .get(4)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok());

        batch.push((
            shape_id,
            shape_pt_lat,
            shape_pt_lon,
            shape_pt_sequence,
            shape_dist_traveled,
        ));

        if batch.len() >= batch_size {
            insert_shapes_batch(&mut *conn, &batch).await?;
            batch.clear();
        }
    }

    // Insert remaining
    if !batch.is_empty() {
        insert_shapes_batch(&mut *conn, &batch).await?;
    }

    info!("Imported shapes.");
    Ok(())
}

async fn insert_shapes_batch(
    conn: &mut PgConnection,
    batch: &[(String, f64, f64, i32, Option<f64>)],
) -> Result<(), Box<dyn std::error::Error>> {
    if batch.is_empty() {
        return Ok(());
    }

    let mut sql = String::from(
        "INSERT INTO gtfs_shapes (shape_id, shape_pt_lat, shape_pt_lon, shape_pt_sequence, shape_dist_traveled) VALUES ",
    );
    let mut values: Vec<String> = Vec::new();

    for (i, (shape_id, lat, lon, seq, dist)) in batch.iter().enumerate() {
        let dist_str = dist.map_or("NULL".to_string(), |d| d.to_string());
        values.push(format!(
            "('{}', {}, {}, {}, {})",
            escape_sql_string(shape_id),
            lat,
            lon,
            seq,
            dist_str
        ));

        if (i + 1) % 500 == 0 || i == batch.len() - 1 {
            sql.push_str(&values.join(","));
            sql.push_str(" ON CONFLICT DO NOTHING");
            sqlx::query(&sql).execute(&mut *conn).await?;
            sql = String::from(
                "INSERT INTO gtfs_shapes (shape_id, shape_pt_lat, shape_pt_lon, shape_pt_sequence, shape_dist_traveled) VALUES ",
            );
            values.clear();
        }
    }

    Ok(())
}

/// Import trips from trips.txt
async fn import_gtfs_trips(
    conn: &mut PgConnection,
    gtfs_path: &Path,
    feed: &GtfsFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    let trips_path = gtfs_path.join("trips.txt");
    if !trips_path.exists() {
        warn!("trips.txt not found, skipping trips import.");
        return Ok(());
    }

    let mut rdr = ReaderBuilder::new().from_path(&trips_path)?;
    let mut count = 0;
    let mut batch: Vec<TripBatchRow> = Vec::new();
    let batch_size = 2000;

    for result in rdr.records() {
        let record = result?;
        let route_id = scoped_gtfs_id(feed, record.get(0).unwrap_or(""));
        let service_id = scoped_gtfs_id(feed, record.get(1).unwrap_or(""));
        let trip_id = scoped_gtfs_id(feed, record.get(2).unwrap_or(""));
        let trip_headsign = record
            .get(3)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let trip_short_name = record
            .get(4)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let direction_id: Option<i32> = record
            .get(5)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok());
        let block_id = record
            .get(6)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let shape_id = record
            .get(7)
            .filter(|s| !s.is_empty())
            .map(|s| scoped_gtfs_id(feed, s));
        let wheelchair_accessible: Option<i32> = record
            .get(8)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok());
        let bikes_allowed: Option<i32> = record
            .get(9)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok());

        batch.push((
            trip_id,
            route_id,
            service_id,
            trip_headsign,
            trip_short_name,
            direction_id,
            block_id,
            shape_id,
            wheelchair_accessible,
            bikes_allowed,
        ));

        if batch.len() >= batch_size {
            insert_trips_batch(&mut *conn, &batch).await?;
            count += batch.len();
            batch.clear();

            if count % 50000 == 0 {
                info!("Imported {} trips...", count);
            }
        }
    }

    // Insert remaining
    if !batch.is_empty() {
        insert_trips_batch(&mut *conn, &batch).await?;
        count += batch.len();
    }

    info!("Imported {} trips.", count);
    Ok(())
}

async fn insert_trips_batch(
    conn: &mut PgConnection,
    batch: &[TripBatchRow],
) -> Result<(), Box<dyn std::error::Error>> {
    if batch.is_empty() {
        return Ok(());
    }

    // Split into smaller chunks (500 rows) to reduce memory usage
    let mut sql = String::from(
        "INSERT INTO gtfs_trips (trip_id, route_id, service_id, trip_headsign, trip_short_name, direction_id, block_id, shape_id, wheelchair_accessible, bikes_allowed) VALUES ",
    );
    let mut values: Vec<String> = Vec::new();

    for (
        i,
        (
            trip_id,
            route_id,
            service_id,
            trip_headsign,
            trip_short_name,
            direction_id,
            block_id,
            shape_id,
            wheelchair_accessible,
            bikes_allowed,
        ),
    ) in batch.iter().enumerate()
    {
        let headsign_str = trip_headsign
            .as_ref()
            .map(|s| format!("'{}'", escape_sql_string(s)))
            .unwrap_or_else(|| "NULL".to_string());
        let short_name_str = trip_short_name
            .as_ref()
            .map(|s| format!("'{}'", escape_sql_string(s)))
            .unwrap_or_else(|| "NULL".to_string());
        let direction_str = direction_id
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".to_string());
        let block_str = block_id
            .as_ref()
            .map(|s| format!("'{}'", escape_sql_string(s)))
            .unwrap_or_else(|| "NULL".to_string());
        let shape_str = shape_id
            .as_ref()
            .map(|s| format!("'{}'", escape_sql_string(s)))
            .unwrap_or_else(|| "NULL".to_string());
        let wheelchair_str = wheelchair_accessible
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".to_string());
        let bikes_str = bikes_allowed
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".to_string());

        values.push(format!(
            "('{}', '{}', '{}', {}, {}, {}, {}, {}, {}, {})",
            escape_sql_string(trip_id),
            escape_sql_string(route_id),
            escape_sql_string(service_id),
            headsign_str,
            short_name_str,
            direction_str,
            block_str,
            shape_str,
            wheelchair_str,
            bikes_str
        ));

        // Execute every 500 rows to reduce memory usage
        if (i + 1) % 500 == 0 || i == batch.len() - 1 {
            sql.push_str(&values.join(","));
            sql.push_str(" ON CONFLICT (trip_id) DO NOTHING");
            sqlx::query(&sql).execute(&mut *conn).await?;
            sql = String::from(
                "INSERT INTO gtfs_trips (trip_id, route_id, service_id, trip_headsign, trip_short_name, direction_id, block_id, shape_id, wheelchair_accessible, bikes_allowed) VALUES ",
            );
            values.clear();
        }
    }

    Ok(())
}

/// Import stop_times from stop_times.txt (largest file, uses batch processing)
async fn import_gtfs_stop_times(
    conn: &mut PgConnection,
    gtfs_path: &Path,
    feed: &GtfsFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    let stop_times_path = gtfs_path.join("stop_times.txt");
    if !stop_times_path.exists() {
        warn!("stop_times.txt not found, skipping stop_times import.");
        return Ok(());
    }

    info!("Importing stop_times (this may take a while)...");

    let mut rdr = ReaderBuilder::new().from_path(&stop_times_path)?;
    let mut count = 0;
    let mut batch: Vec<StopTimeBatchRow> = Vec::new();
    let batch_size = 1000;

    for result in rdr.records() {
        let record = result?;
        // trip_id,arrival_time,departure_time,stop_id,stop_sequence,stop_headsign,pickup_type,drop_off_type,shape_dist_traveled,timepoint
        let trip_id = scoped_gtfs_id(feed, record.get(0).unwrap_or(""));
        let arrival_time = parse_gtfs_time(record.get(1).unwrap_or(""));
        let departure_time = parse_gtfs_time(record.get(2).unwrap_or(""));
        let stop_id = scoped_gtfs_id(feed, record.get(3).unwrap_or(""));
        let stop_sequence: i32 = record.get(4).unwrap_or("0").parse().unwrap_or(0);
        let stop_headsign = record
            .get(5)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let pickup_type: Option<i32> = record
            .get(6)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok());
        let drop_off_type: Option<i32> = record
            .get(7)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok());
        let shape_dist_traveled: Option<f64> = record
            .get(8)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok());
        let timepoint: Option<i32> = record
            .get(9)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok());

        batch.push((
            trip_id,
            arrival_time,
            departure_time,
            stop_id,
            stop_sequence,
            stop_headsign,
            pickup_type,
            drop_off_type,
            shape_dist_traveled,
            timepoint,
        ));

        if batch.len() >= batch_size {
            insert_stop_times_batch(&mut *conn, &batch).await?;
            count += batch.len();
            batch.clear();

            if count % 50000 == 0 {
                info!("Imported {} stop_times...", count);
            }
        }
    }

    // Insert remaining
    if !batch.is_empty() {
        insert_stop_times_batch(&mut *conn, &batch).await?;
        count += batch.len();
    }

    info!("Imported {} stop_times.", count);
    Ok(())
}

/// Parse GTFS time format (HH:MM:SS, can be > 24:00:00 for times past midnight)
/// Returns the time string as-is to support 24+ hour times (e.g., "25:30:00")
fn parse_gtfs_time(time_str: &str) -> Option<String> {
    if time_str.is_empty() {
        return None;
    }

    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return None;
    }

    // Validate that all parts are valid numbers
    let _hours: u32 = parts[0].parse().ok()?;
    let _minutes: u32 = parts[1].parse().ok()?;
    let _seconds: u32 = parts[2].parse().ok()?;

    // Return the original string to support times > 24:00:00
    Some(time_str.to_string())
}

/// ODPT bus timetable values use HH:MM, while GTFS requires HH:MM:SS.
fn parse_odpt_time(time_str: &str) -> Option<String> {
    if let Some(gtfs_time) = parse_gtfs_time(time_str) {
        return Some(gtfs_time);
    }

    let (hours, minutes) = time_str.split_once(':')?;
    let hours: u32 = hours.parse().ok()?;
    let minutes: u32 = minutes.parse().ok()?;
    if minutes >= 60 {
        return None;
    }
    Some(format!("{hours:02}:{minutes:02}:00"))
}

async fn insert_stop_times_batch(
    conn: &mut PgConnection,
    batch: &[StopTimeBatchRow],
) -> Result<(), Box<dyn std::error::Error>> {
    if batch.is_empty() {
        return Ok(());
    }

    // Build multi-row INSERT for better performance
    // Split into smaller chunks (500 rows) to reduce memory usage
    let mut sql = String::from(
        "INSERT INTO gtfs_stop_times (trip_id, arrival_time, departure_time, stop_id, stop_sequence, stop_headsign, pickup_type, drop_off_type, shape_dist_traveled, timepoint) VALUES ",
    );
    let mut values: Vec<String> = Vec::new();

    for (
        i,
        (
            trip_id,
            arrival_time,
            departure_time,
            stop_id,
            stop_sequence,
            stop_headsign,
            pickup_type,
            drop_off_type,
            shape_dist_traveled,
            timepoint,
        ),
    ) in batch.iter().enumerate()
    {
        let arrival_str = arrival_time
            .as_ref()
            .map(|t| format!("'{}'", t))
            .unwrap_or_else(|| "NULL".to_string());
        let departure_str = departure_time
            .as_ref()
            .map(|t| format!("'{}'", t))
            .unwrap_or_else(|| "NULL".to_string());
        let headsign_str = stop_headsign
            .as_ref()
            .map(|s| format!("'{}'", escape_sql_string(s)))
            .unwrap_or_else(|| "NULL".to_string());
        let pickup_str = pickup_type
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".to_string());
        let dropoff_str = drop_off_type
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".to_string());
        let dist_str = shape_dist_traveled
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".to_string());
        let timepoint_str = timepoint
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".to_string());

        values.push(format!(
            "('{}', {}, {}, '{}', {}, {}, {}, {}, {}, {})",
            escape_sql_string(trip_id),
            arrival_str,
            departure_str,
            escape_sql_string(stop_id),
            stop_sequence,
            headsign_str,
            pickup_str,
            dropoff_str,
            dist_str,
            timepoint_str
        ));

        // Execute every 500 rows to reduce memory usage
        if (i + 1) % 500 == 0 || i == batch.len() - 1 {
            sql.push_str(&values.join(","));
            sql.push_str(" ON CONFLICT DO NOTHING");
            sqlx::query(&sql).execute(&mut *conn).await?;
            sql = String::from(
                "INSERT INTO gtfs_stop_times (trip_id, arrival_time, departure_time, stop_id, stop_sequence, stop_headsign, pickup_type, drop_off_type, shape_dist_traveled, timepoint) VALUES ",
            );
            values.clear();
        }
    }

    Ok(())
}

/// Import feed_info from feed_info.txt
async fn import_gtfs_feed_info(
    conn: &mut PgConnection,
    gtfs_path: &Path,
    feed: &GtfsFeed,
) -> Result<(), Box<dyn std::error::Error>> {
    let feed_info_path = gtfs_path.join("feed_info.txt");
    if !feed_info_path.exists() {
        warn!("feed_info.txt not found, skipping feed_info import.");
        return Ok(());
    }

    let mut rdr = ReaderBuilder::new().from_path(&feed_info_path)?;

    for result in rdr.records() {
        let record = result?;
        // feed_publisher_name,feed_publisher_url,feed_lang,feed_start_date,feed_end_date,feed_version
        let feed_publisher_name = format!("{} ({})", record.get(0).unwrap_or(""), feed.name);
        let feed_publisher_url = record.get(1).filter(|s| !s.is_empty());
        let feed_lang = record.get(2).filter(|s| !s.is_empty());
        let feed_start_date = record
            .get(3)
            .filter(|s| !s.is_empty())
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y%m%d").ok());
        let feed_end_date = record
            .get(4)
            .filter(|s| !s.is_empty())
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y%m%d").ok());
        let feed_version = record.get(5).filter(|s| !s.is_empty());

        sqlx::query(
            r#"INSERT INTO gtfs_feed_info
               (feed_publisher_name, feed_publisher_url, feed_lang, feed_start_date, feed_end_date, feed_version)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(feed_publisher_name)
        .bind(feed_publisher_url)
        .bind(feed_lang)
        .bind(feed_start_date)
        .bind(feed_end_date)
        .bind(feed_version)
        .execute(&mut *conn)
        .await?;
    }

    info!("Imported feed_info.");
    Ok(())
}

fn is_bus_feature_disabled() -> bool {
    env_flag_enabled("DISABLE_BUS_FEATURE")
}

fn env_flag_enabled(name: &str) -> bool {
    match env::var(name) {
        Ok(s) => s.eq_ignore_ascii_case("true") || s == "1",
        Err(_) => false,
    }
}

// ============================================================
// GTFS to Stations/Lines Integration
// ============================================================

/// Escape a string for safe inclusion in SQL queries.
/// Escapes backslashes first, then single quotes, matching PostgreSQL string literal syntax.
fn escape_sql_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "''")
}

/// Convert hiragana characters to katakana
/// Hiragana range: U+3041 to U+3096
/// Katakana range: U+30A1 to U+30F6
fn hiragana_to_katakana(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{3041}'..='\u{3096}').contains(&c) {
                char::from_u32(c as u32 + 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// FNV-1a hash function for deterministic hashing across process invocations
/// Unlike DefaultHasher, this produces consistent results across runs
fn fnv1a_hash(data: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Generate deterministic line_cd from route_id
/// Uses range starting at 100,000,000 to avoid conflicts with existing rail data
fn generate_bus_line_cd(route_id: &str) -> i32 {
    let hash = fnv1a_hash(route_id.as_bytes());
    100_000_000 + (hash % 10_000_000) as i32
}

/// Generate deterministic station_cd from stop_id and route_id
/// Uses range starting at 200,000,000 to avoid conflicts with existing rail data
fn generate_bus_station_cd(stop_id: &str, route_id: &str) -> i32 {
    let combined = format!("{}-{}", stop_id, route_id);
    let hash = fnv1a_hash(combined.as_bytes());
    200_000_000 + (hash % 100_000_000) as i32
}

/// Generate deterministic station_g_cd from stop_id only (shared across routes)
/// Same bus stop on different routes will have the same station_g_cd
fn generate_bus_station_g_cd(stop_id: &str) -> i32 {
    let hash = fnv1a_hash(stop_id.as_bytes());
    200_000_000 + (hash % 100_000_000) as i32
}

/// Distance within which two like-named bus stops are treated as direction
/// poles of a single physical stop and merged into one `station_g_cd`.
const BUS_STOP_GROUPING_RADIUS_METERS: f64 = 250.0;

/// Build a `stop_id -> station_g_cd` map that collapses the separate direction
/// poles of one physical bus stop into a single group.
///
/// Some GTFS feeds encode poles with a `parent_station` hierarchy (Toei), in
/// which case only the parent is imported and grouping is already correct. Other
/// feeds (Seibu, the Tokyu community buses, and typically Keio) expose every pole
/// as its own top-level stop, so the same bus stop name would otherwise appear
/// multiple times in the app. Here we group stops that share a name *and* sit
/// within `BUS_STOP_GROUPING_RADIUS_METERS` of each other, mirroring how rail
/// stations collapse across companies via `station_g_cd`.
///
/// Proximity scoping keeps genuinely distinct stops that merely share a common
/// name (e.g. "新道" in different cities) in separate groups. A lone stop yields
/// the exact same `station_g_cd` as [`generate_bus_station_g_cd`], so feeds that
/// already group correctly are unaffected.
fn build_bus_station_g_cd_map(stops: &[GtfsStopRow]) -> HashMap<String, i32> {
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }

    // Bucket by name first; poles of one physical stop always share the name.
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for (idx, stop) in stops.iter().enumerate() {
        by_name.entry(stop.stop_name.trim()).or_default().push(idx);
    }

    let mut map: HashMap<String, i32> = HashMap::with_capacity(stops.len());
    for indices in by_name.values() {
        // Union same-name stops that are within the grouping radius.
        let mut parent: Vec<usize> = (0..indices.len()).collect();
        for a in 0..indices.len() {
            for b in (a + 1)..indices.len() {
                let sa = &stops[indices[a]];
                let sb = &stops[indices[b]];
                if haversine_distance(sa.stop_lat, sa.stop_lon, sb.stop_lat, sb.stop_lon)
                    <= BUS_STOP_GROUPING_RADIUS_METERS
                {
                    let ra = find(&mut parent, a);
                    let rb = find(&mut parent, b);
                    if ra != rb {
                        parent[ra] = rb;
                    }
                }
            }
        }

        // Representative of each cluster = lexicographically smallest stop_id, so
        // the derived station_g_cd is stable regardless of DB row ordering.
        let roots: Vec<usize> = (0..indices.len()).map(|a| find(&mut parent, a)).collect();
        let mut rep_stop_id: HashMap<usize, &str> = HashMap::new();
        for (a, &root) in roots.iter().enumerate() {
            let sid = stops[indices[a]].stop_id.as_str();
            rep_stop_id
                .entry(root)
                .and_modify(|cur| {
                    if sid < *cur {
                        *cur = sid;
                    }
                })
                .or_insert(sid);
        }

        for (a, &root) in roots.iter().enumerate() {
            let g_cd = generate_bus_station_g_cd(rep_stop_id[&root]);
            map.insert(stops[indices[a]].stop_id.clone(), g_cd);
        }
    }

    map
}

/// Generate deterministic type_cd from (route_id, shape_id).
/// Uses range starting at 100,000,000 to avoid conflicts with existing rail types.
fn generate_bus_type_cd(route_id: &str, shape_id: &str) -> i32 {
    let combined = format!("type-{}-{}", route_id, shape_id);
    let hash = fnv1a_hash(combined.as_bytes());
    100_000_000 + (hash % 100_000_000) as i32
}

/// Generate deterministic line_group_cd from (route_id, shape_id).
/// Uses range starting at 100,000,000 to avoid conflicts with existing rail line groups.
fn generate_bus_line_group_cd(route_id: &str, shape_id: &str) -> i32 {
    let combined = format!("lg-{}-{}", route_id, shape_id);
    let hash = fnv1a_hash(combined.as_bytes());
    100_000_000 + (hash % 100_000_000) as i32
}

fn company_cd_for_gtfs_route(route_id: &str) -> Option<i32> {
    if route_id.starts_with("toei:") {
        Some(119) // Toei Transportation
    } else if route_id.starts_with("seibu:") {
        Some(253) // Seibu Bus
    } else if route_id.starts_with("keio:") {
        Some(254) // Keio Bus
    } else if route_id.starts_with("tokyu_") {
        Some(255) // Tokyu Bus
    } else {
        None // Unknown/unsupported prefix
    }
}

/// `types.kind` value for bus route variations. Matches `proto::TrainTypeKind::BusRoute`.
const BUS_ROUTE_KIND: i32 = 7;

/// Row type for reading gtfs_routes
#[derive(sqlx::FromRow)]
struct GtfsRouteRow {
    route_id: String,
    #[allow(dead_code)]
    agency_id: Option<String>,
    route_short_name: Option<String>,
    route_long_name: Option<String>,
    #[allow(dead_code)]
    route_long_name_k: Option<String>,
    #[allow(dead_code)]
    route_long_name_r: Option<String>,
    #[allow(dead_code)]
    route_long_name_zh: Option<String>,
    #[allow(dead_code)]
    route_long_name_ko: Option<String>,
    #[allow(dead_code)]
    route_desc: Option<String>,
    route_type: i32,
    #[allow(dead_code)]
    route_url: Option<String>,
    route_color: Option<String>,
    #[allow(dead_code)]
    route_text_color: Option<String>,
    #[allow(dead_code)]
    route_sort_order: Option<i32>,
}

/// Row type for reading gtfs_stops
#[derive(sqlx::FromRow)]
struct GtfsStopRow {
    stop_id: String,
    #[allow(dead_code)]
    stop_code: Option<String>,
    stop_name: String,
    stop_name_k: Option<String>,
    stop_name_r: Option<String>,
    stop_name_zh: Option<String>,
    stop_name_ko: Option<String>,
    #[allow(dead_code)]
    stop_desc: Option<String>,
    stop_lat: f64,
    stop_lon: f64,
    #[allow(dead_code)]
    zone_id: Option<String>,
    #[allow(dead_code)]
    stop_url: Option<String>,
    #[allow(dead_code)]
    location_type: Option<i32>,
    #[allow(dead_code)]
    parent_station: Option<String>,
    #[allow(dead_code)]
    stop_timezone: Option<String>,
    #[allow(dead_code)]
    wheelchair_boarding: Option<i32>,
    #[allow(dead_code)]
    platform_code: Option<String>,
}

/// Integrate GTFS bus data into stations/lines tables
///
/// This function wraps all integration operations in a single database transaction.
/// If any step fails, all changes are rolled back to maintain database consistency.
pub async fn integrate_gtfs_to_stations() -> Result<(), Box<dyn std::error::Error>> {
    let total_start = std::time::Instant::now();
    info!("[integrate] entering integrate_gtfs_to_stations");

    if is_bus_feature_disabled() {
        info!("Bus feature is disabled, skipping GTFS integration.");
        return Ok(());
    }

    info!("[integrate] connecting to database");
    let db_url = fetch_database_url();
    let mut conn = PgConnection::connect(&db_url).await?;

    // Check if GTFS data exists (outside transaction for quick exit)
    let gtfs_route_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM gtfs_routes")
        .fetch_one(&mut conn)
        .await?;
    info!(
        "[integrate] gtfs_routes count = {}, total_elapsed = {:?}",
        gtfs_route_count.0,
        total_start.elapsed()
    );

    if gtfs_route_count.0 == 0 {
        info!("No GTFS routes found, skipping integration.");
        return Ok(());
    }

    info!("Starting GTFS to stations/lines integration (using transaction)...");

    // Begin transaction - all changes will be rolled back if any step fails
    let mut tx = conn.begin().await?;
    info!("[integrate] transaction begun, clearing existing bus data");

    // Step 1: Clear existing bus data from stations/lines/types/sst.
    // station_station_types references both types (FK) and stations (FK), so delete
    // bus sst rows before bus types and before stations.
    let step_start = std::time::Instant::now();
    sqlx::query(
        "DELETE FROM station_station_types WHERE type_cd IN (SELECT type_cd FROM types WHERE kind = $1)",
    )
    .bind(BUS_ROUTE_KIND)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM types WHERE kind = $1")
        .bind(BUS_ROUTE_KIND)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM stations WHERE transport_type = 1")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM lines WHERE transport_type = 1")
        .execute(&mut *tx)
        .await?;
    info!(
        "[integrate] cleared existing bus data in {:?}",
        step_start.elapsed()
    );

    let step_start = std::time::Instant::now();
    integrate_gtfs_routes_to_lines(&mut tx).await?;
    info!(
        "[integrate] routes_to_lines done in {:?}",
        step_start.elapsed()
    );

    let step_start = std::time::Instant::now();
    let stop_route_map = build_stop_route_mapping(&mut tx).await?;
    info!(
        "[integrate] build_stop_route_mapping done in {:?} ({} entries)",
        step_start.elapsed(),
        stop_route_map.len()
    );

    let step_start = std::time::Instant::now();
    integrate_gtfs_stops_to_stations(&mut tx, &stop_route_map).await?;
    info!(
        "[integrate] stops_to_stations done in {:?}",
        step_start.elapsed()
    );

    let step_start = std::time::Instant::now();
    update_gtfs_crossreferences(&mut tx, &stop_route_map).await?;
    info!(
        "[integrate] crossreferences done in {:?}",
        step_start.elapsed()
    );

    let step_start = std::time::Instant::now();
    integrate_gtfs_trip_variations_to_types(&mut tx).await?;
    info!(
        "[integrate] trip_variations_to_types done in {:?}",
        step_start.elapsed()
    );

    info!("[integrate] committing transaction");
    let commit_start = std::time::Instant::now();
    tx.commit().await?;
    info!(
        "[integrate] transaction committed in {:?}",
        commit_start.elapsed()
    );

    info!(
        "GTFS integration completed successfully (transaction committed). total={:?}",
        total_start.elapsed()
    );
    Ok(())
}

/// Integrate gtfs_routes into lines table
async fn integrate_gtfs_routes_to_lines(
    conn: &mut PgConnection,
) -> Result<(), Box<dyn std::error::Error>> {
    let routes: Vec<GtfsRouteRow> = sqlx::query_as("SELECT * FROM gtfs_routes")
        .fetch_all(&mut *conn)
        .await?;

    for route in &routes {
        let company_cd = match company_cd_for_gtfs_route(&route.route_id) {
            Some(cd) => cd,
            None => continue, // Skip routes with unknown/unsupported prefix
        };
        let line_cd = generate_bus_line_cd(&route.route_id);
        let line_name = route
            .route_short_name
            .clone()
            .unwrap_or_else(|| route.route_long_name.clone().unwrap_or_default());
        let line_color = route
            .route_color
            .as_deref()
            .filter(|c| !c.is_empty())
            .map(|c| {
                if c.starts_with('#') {
                    c.to_string()
                } else {
                    format!("#{}", c)
                }
            })
            .unwrap_or_else(|| DEFAULT_GTFS_BUS_LINE_COLOR.to_string());

        let line_name_r = route.route_long_name.clone().unwrap_or_default();

        sqlx::query(
            r#"INSERT INTO lines (
                line_cd, company_cd, line_name, line_name_k, line_name_h,
                line_name_r, line_color_c, line_type, e_status, e_sort, transport_type
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, 0, $1, 1
            )
            ON CONFLICT (line_cd) DO NOTHING"#,
        )
        .bind(line_cd)
        .bind(company_cd)
        .bind(&line_name)
        .bind(&line_name) // line_name_k
        .bind(&line_name) // line_name_h
        .bind(&line_name_r) // line_name_r
        .bind(&line_color)
        .bind(route.route_type)
        .execute(&mut *conn)
        .await?;

        // Update gtfs_routes with generated line_cd
        sqlx::query("UPDATE gtfs_routes SET line_cd = $1 WHERE route_id = $2")
            .bind(line_cd)
            .bind(&route.route_id)
            .execute(&mut *conn)
            .await?;
    }

    info!("Integrated {} routes as lines.", routes.len());
    Ok(())
}

/// Build mapping of (parent_stop_id, route_id) -> stop_sequence from gtfs_stop_times
/// Groups child stops by their parent_station to represent physical bus stops
///
/// # Ordering Strategy
///
/// ## 1. Main trip selection
/// Pick one representative trip per route. The selection prefers:
///
/// 1. Trips whose `shape_id` matches the route's canonical shape, where the
///    canonical shape is the one covering the most unique stops (with longest
///    `MAX(shape_dist_traveled)` and `direction_id=0` as tiebreakers). This
///    avoids picking a short-turn variant when the full route only exists on
///    trips with `direction_id` NULL — as is the case for 池86, whose full
///    loop is recorded with `direction_id` empty while the only `direction_id=0`
///    trip is a 13-stop short turn (新宿伊勢丹前→池袋駅東口).
/// 2. `direction_id = 0` (then NULL, then 1)
/// 3. Most unique stops
/// 4. Longest `MAX(shape_dist_traveled)`
/// 5. `trip_id` for a deterministic tiebreak
///
/// The main trip's `stop_sequence` is strictly monotonic within that trip, so
/// stops on it inherit a reliable canonical order without having to compare
/// `shape_dist_traveled` across trips on different shapes.
///
/// ## 2. Variant stop estimation
/// For stops only on variant trips (not on the main trip):
/// - Use LAG/LEAD to find neighboring stops on the variant trip
/// - Look up neighbors' positions on the main trip
/// - Interpolate position based on neighbor positions
/// - Terminal stops (next_stop_id IS NULL) are placed at the end
/// - Start stops (prev_stop_id IS NULL) are placed at the beginning
async fn build_stop_route_mapping(
    conn: &mut PgConnection,
) -> Result<HashMap<String, Vec<(String, i32)>>, Box<dyn std::error::Error>> {
    // Strategy:
    // 1. Compute a canonical shape_id per route: the shape covering the most unique
    //    stops, with longest shape_dist_traveled and direction_id=0 as tiebreakers.
    //    Length comes first because some routes (e.g. 池86) record their full route
    //    only under direction_id NULL while a short-turn variant is the sole
    //    direction_id=0 trip — preferring direction_id=0 first would pin the main
    //    trip to that 13-stop short turn and push the full-route stops (渋谷駅東口,
    //    宮下公園, etc.) into the variant-interpolation path.
    // 2. Pick a main trip per route, preferring trips on the canonical shape, then
    //    direction_id=0 / most stops / longest distance. Its stop_sequence is the
    //    canonical order. This avoids the earlier attempt's bug where
    //    shape_dist_traveled was compared across trips with different shape_ids
    //    (short-turn / branching shapes restart dist at 0, so comparing them
    //    pulled mid-route stops to the front of multi-variant routes like 池86).
    // 3. Variant-only stops (on trips with a different shape) are interpolated
    //    against the main trip via LAG/LEAD neighbor lookup.
    let rows: Vec<(String, String, i32)> = sqlx::query_as(
        r#"WITH RECURSIVE canonical_shape AS (
               -- Pick one canonical shape_id per route. Used purely to bias main trip
               -- selection: a trip on the canonical shape is preferred over a trip
               -- with the same stop count on some short-turn / branching shape.
               -- Prefer the shape with the most unique stops, then longest physical
               -- distance, then direction_id=0 (then NULL, then 1), then shape_id for
               -- a deterministic tiebreak. Stop count comes before direction because
               -- e.g. 池86 records its full loop only under direction_id NULL while
               -- the sole direction_id=0 trip is a 13-stop short turn.
               SELECT DISTINCT ON (gt.route_id)
                   gt.route_id,
                   gt.shape_id
               FROM gtfs_trips gt
               JOIN gtfs_stop_times gst ON gt.trip_id = gst.trip_id
               WHERE gt.shape_id IS NOT NULL
               GROUP BY gt.route_id, gt.shape_id, gt.direction_id
               ORDER BY gt.route_id,
                        COUNT(DISTINCT gst.stop_id) DESC,
                        MAX(gst.shape_dist_traveled) DESC NULLS LAST,
                        CASE WHEN gt.direction_id = 0 THEN 0
                             WHEN gt.direction_id IS NULL THEN 1
                             ELSE 2 END,
                        gt.shape_id
           ),
           main_trips AS (
               -- One representative trip per route. Prefer a trip on the canonical
               -- shape, then direction_id=0, then most unique stops, then longest
               -- shape distance, then trip_id for determinism. The trip's
               -- stop_sequence is strictly monotonic, so it produces a reliable
               -- canonical order for every stop on it without cross-shape
               -- comparison hazards. Variant-only stops are interpolated below.
               SELECT DISTINCT ON (gt.route_id)
                   gt.route_id,
                   gt.trip_id,
                   gt.direction_id as main_direction_id,
                   COUNT(*) as stop_count
               FROM gtfs_trips gt
               JOIN gtfs_stop_times gst ON gt.trip_id = gst.trip_id
               LEFT JOIN canonical_shape cs
                   ON cs.route_id = gt.route_id AND cs.shape_id = gt.shape_id
               GROUP BY gt.route_id, gt.trip_id, gt.direction_id, cs.shape_id
               ORDER BY gt.route_id,
                        CASE WHEN cs.shape_id IS NOT NULL THEN 0 ELSE 1 END,
                        CASE WHEN gt.direction_id = 0 THEN 0 ELSE 1 END,
                        COUNT(DISTINCT gst.stop_id) DESC,
                        MAX(gst.shape_dist_traveled) DESC NULLS LAST,
                        COUNT(*) DESC,
                        gt.trip_id
           ),
           main_trip_stops AS (
               -- Get stops from main trips with their sequence
               SELECT DISTINCT ON (COALESCE(gs.parent_station, gs.stop_id), mt.route_id)
                   COALESCE(gs.parent_station, gs.stop_id) as parent_stop_id,
                   mt.route_id,
                   gst.stop_sequence
               FROM main_trips mt
               JOIN gtfs_stop_times gst ON mt.trip_id = gst.trip_id
               JOIN gtfs_stops gs ON gst.stop_id = gs.stop_id
               ORDER BY COALESCE(gs.parent_station, gs.stop_id), mt.route_id, gst.stop_sequence
           ),
           main_trip_max_seq AS (
               SELECT route_id, MAX(stop_sequence) as max_seq
               FROM main_trip_stops
               GROUP BY route_id
           ),
           -- Get variant trips (non-main trips) with their stops and neighbors using window functions
           variant_trip_stops_with_neighbors AS (
               SELECT
                   COALESCE(gs.parent_station, gs.stop_id) as parent_stop_id,
                   gt.route_id,
                   gt.trip_id,
                   gt.direction_id as variant_direction_id,
                   gst.stop_sequence,
                   LAG(COALESCE(gs.parent_station, gs.stop_id)) OVER (
                       PARTITION BY gt.trip_id ORDER BY gst.stop_sequence
                   ) as prev_stop_id,
                   LEAD(COALESCE(gs.parent_station, gs.stop_id)) OVER (
                       PARTITION BY gt.trip_id ORDER BY gst.stop_sequence
                   ) as next_stop_id
               FROM gtfs_trips gt
               JOIN gtfs_stop_times gst ON gt.trip_id = gst.trip_id
               JOIN gtfs_stops gs ON gst.stop_id = gs.stop_id
               WHERE NOT EXISTS (
                   SELECT 1 FROM main_trips mt WHERE mt.trip_id = gt.trip_id
               )
           ),
           -- Find variant-only stops (not on main trip) with their neighbor info
           -- Prioritize records where neighbors exist on main trip for better position estimation
           variant_only_with_neighbors AS (
               SELECT DISTINCT ON (vts.parent_stop_id, vts.route_id)
                   vts.parent_stop_id,
                   vts.route_id,
                   vts.variant_direction_id,
                   vts.prev_stop_id,
                   vts.next_stop_id
               FROM variant_trip_stops_with_neighbors vts
               LEFT JOIN main_trip_stops mts_prev
                   ON vts.prev_stop_id = mts_prev.parent_stop_id
                   AND vts.route_id = mts_prev.route_id
               LEFT JOIN main_trip_stops mts_next
                   ON vts.next_stop_id = mts_next.parent_stop_id
                   AND vts.route_id = mts_next.route_id
               WHERE NOT EXISTS (
                   SELECT 1 FROM main_trip_stops mts
                   WHERE mts.parent_stop_id = vts.parent_stop_id
                     AND mts.route_id = vts.route_id
               )
               ORDER BY vts.parent_stop_id, vts.route_id,
                        -- Prioritize records where neighbors exist on main trip
                        CASE
                            WHEN mts_prev.parent_stop_id IS NOT NULL AND mts_next.parent_stop_id IS NOT NULL THEN 0
                            WHEN mts_prev.parent_stop_id IS NOT NULL OR mts_next.parent_stop_id IS NOT NULL THEN 1
                            ELSE 2
                        END,
                        vts.stop_sequence
           ),
           -- Per-route upper bound for the prev/next recursion depth. The chain
           -- can revisit each variant-only stop at most once (enforced by the
           -- `visited` array), so the variant-only stop count plus a small
           -- safety margin is a safe and tight cap. This replaces the previous
           -- fixed limit of 10, which truncated chains on routes whose variant
           -- detour spans 11+ stops before rejoining the main trip.
           variant_chain_limit AS (
               SELECT
                   route_id,
                   COUNT(*)::INT + 1 AS max_depth
               FROM variant_only_with_neighbors
               GROUP BY route_id
           ),
           -- Recursive CTE to find the nearest main-trip stop by following prev chain
           prev_chain AS (
               -- Base case: start from each variant stop
               SELECT
                   von.parent_stop_id as origin_stop_id,
                   von.route_id,
                   von.prev_stop_id as current_stop_id,
                   1 as depth,
                   ARRAY[von.parent_stop_id::TEXT] as visited
               FROM variant_only_with_neighbors von
               WHERE von.prev_stop_id IS NOT NULL

               UNION ALL

               -- Recursive case: if current stop is also variant-only, follow its prev
               SELECT
                   pc.origin_stop_id,
                   pc.route_id,
                   von2.prev_stop_id as current_stop_id,
                   pc.depth + 1,
                   pc.visited || pc.current_stop_id::TEXT
               FROM prev_chain pc
               JOIN variant_only_with_neighbors von2
                   ON pc.current_stop_id = von2.parent_stop_id
                   AND pc.route_id = von2.route_id
               JOIN variant_chain_limit vcl
                   ON vcl.route_id = pc.route_id
               WHERE pc.depth < vcl.max_depth
                   AND von2.prev_stop_id IS NOT NULL
                   AND NOT pc.current_stop_id::TEXT = ANY(pc.visited)
                   -- Stop if we already found a main-trip stop
                   AND NOT EXISTS (
                       SELECT 1 FROM main_trip_stops mts
                       WHERE mts.parent_stop_id = pc.current_stop_id
                         AND mts.route_id = pc.route_id
                   )
           ),
           prev_resolved AS (
               -- For each origin stop, find the first stop in the chain that's on main trip
               SELECT DISTINCT ON (pc.origin_stop_id, pc.route_id)
                   pc.origin_stop_id,
                   pc.route_id,
                   mts.stop_sequence as prev_main_seq,
                   pc.depth as prev_depth
               FROM prev_chain pc
               JOIN main_trip_stops mts
                   ON pc.current_stop_id = mts.parent_stop_id
                   AND pc.route_id = mts.route_id
               ORDER BY pc.origin_stop_id, pc.route_id, pc.depth
           ),
           -- Similarly, recursive CTE for next chain
           next_chain AS (
               SELECT
                   von.parent_stop_id as origin_stop_id,
                   von.route_id,
                   von.next_stop_id as current_stop_id,
                   1 as depth,
                   ARRAY[von.parent_stop_id::TEXT] as visited
               FROM variant_only_with_neighbors von
               WHERE von.next_stop_id IS NOT NULL

               UNION ALL

               SELECT
                   nc.origin_stop_id,
                   nc.route_id,
                   von2.next_stop_id as current_stop_id,
                   nc.depth + 1,
                   nc.visited || nc.current_stop_id::TEXT
               FROM next_chain nc
               JOIN variant_only_with_neighbors von2
                   ON nc.current_stop_id = von2.parent_stop_id
                   AND nc.route_id = von2.route_id
               JOIN variant_chain_limit vcl
                   ON vcl.route_id = nc.route_id
               WHERE nc.depth < vcl.max_depth
                   AND von2.next_stop_id IS NOT NULL
                   AND NOT nc.current_stop_id::TEXT = ANY(nc.visited)
                   AND NOT EXISTS (
                       SELECT 1 FROM main_trip_stops mts
                       WHERE mts.parent_stop_id = nc.current_stop_id
                         AND mts.route_id = nc.route_id
                   )
           ),
           next_resolved AS (
               SELECT DISTINCT ON (nc.origin_stop_id, nc.route_id)
                   nc.origin_stop_id,
                   nc.route_id,
                   mts.stop_sequence as next_main_seq,
                   nc.depth as next_depth
               FROM next_chain nc
               JOIN main_trip_stops mts
                   ON nc.current_stop_id = mts.parent_stop_id
                   AND nc.route_id = mts.route_id
               ORDER BY nc.origin_stop_id, nc.route_id, nc.depth
           ),
           -- Look up main trip sequences for the neighbors (with recursive fallback)
           -- When variant trip has different direction_id than main trip, swap prev/next
           variant_estimated AS (
               SELECT
                   von.parent_stop_id,
                   von.route_id,
                   CASE
                       -- Direct neighbors on main trip (single-level lookup)
                       WHEN prev_mts.stop_sequence IS NOT NULL AND next_mts.stop_sequence IS NOT NULL
                           THEN ((prev_mts.stop_sequence + next_mts.stop_sequence) / 2.0)
                       WHEN prev_mts.stop_sequence IS NOT NULL
                           THEN CASE WHEN von.variant_direction_id IS NULL
                                          OR von.variant_direction_id = mt.main_direction_id
                                     THEN (prev_mts.stop_sequence + 0.5)
                                     ELSE (prev_mts.stop_sequence - 0.5)
                                END
                       WHEN next_mts.stop_sequence IS NOT NULL
                           THEN CASE WHEN von.variant_direction_id IS NULL
                                          OR von.variant_direction_id = mt.main_direction_id
                                     THEN (next_mts.stop_sequence - 0.5)
                                     ELSE (next_mts.stop_sequence + 0.5)
                                END
                       -- Recursive fallback: use resolved chains
                       WHEN pr.prev_main_seq IS NOT NULL AND nr.next_main_seq IS NOT NULL
                           THEN (pr.prev_main_seq + nr.next_main_seq) / 2.0
                               + (pr.prev_depth - nr.next_depth) * 0.01  -- Slight offset based on depth difference
                       WHEN pr.prev_main_seq IS NOT NULL
                           THEN CASE WHEN von.variant_direction_id IS NULL
                                          OR von.variant_direction_id = mt.main_direction_id
                                     THEN (pr.prev_main_seq + 0.1 * pr.prev_depth)
                                     ELSE (pr.prev_main_seq - 0.1 * pr.prev_depth)
                                END
                       WHEN nr.next_main_seq IS NOT NULL
                           THEN CASE WHEN von.variant_direction_id IS NULL
                                          OR von.variant_direction_id = mt.main_direction_id
                                     THEN (nr.next_main_seq - 0.1 * nr.next_depth)
                                     ELSE (nr.next_main_seq + 0.1 * nr.next_depth)
                                END
                       -- TERMINAL stop (next_stop_id IS NULL, no neighbors on main trip): put at END or START based on direction
                       WHEN von.next_stop_id IS NULL
                           THEN CASE WHEN von.variant_direction_id IS NULL
                                          OR von.variant_direction_id = mt.main_direction_id
                                     THEN (mtms.max_seq + 0.5)
                                     ELSE 0.5
                                END
                       -- START stop (prev_stop_id IS NULL, no neighbors on main trip): put at START or END based on direction
                       WHEN von.prev_stop_id IS NULL
                           THEN CASE WHEN von.variant_direction_id IS NULL
                                          OR von.variant_direction_id = mt.main_direction_id
                                     THEN 0.5
                                     ELSE (mtms.max_seq + 0.5)
                                END
                       -- Fallback: put at end
                       ELSE (mtms.max_seq + 9999)
                   END as estimated_seq
               FROM variant_only_with_neighbors von
               JOIN main_trips mt ON von.route_id = mt.route_id
               JOIN main_trip_max_seq mtms ON von.route_id = mtms.route_id
               LEFT JOIN main_trip_stops prev_mts
                   ON von.prev_stop_id = prev_mts.parent_stop_id
                   AND von.route_id = prev_mts.route_id
               LEFT JOIN main_trip_stops next_mts
                   ON von.next_stop_id = next_mts.parent_stop_id
                   AND von.route_id = next_mts.route_id
               LEFT JOIN prev_resolved pr
                   ON von.parent_stop_id = pr.origin_stop_id
                   AND von.route_id = pr.route_id
               LEFT JOIN next_resolved nr
                   ON von.parent_stop_id = nr.origin_stop_id
                   AND von.route_id = nr.route_id
           ),
           combined AS (
               SELECT parent_stop_id, route_id, stop_sequence::FLOAT as seq, 1 as priority
               FROM main_trip_stops
               UNION ALL
               SELECT parent_stop_id, route_id, estimated_seq as seq, 2 as priority
               FROM variant_estimated
           ),
           unique_stops AS (
               -- Deduplicate: prefer shape distance > main trip > variant
               SELECT DISTINCT ON (parent_stop_id, route_id)
                   parent_stop_id,
                   route_id,
                   seq
               FROM combined
               ORDER BY parent_stop_id, route_id, priority, seq
           ),
           numbered AS (
               -- Re-number sequences to be consecutive integers
               SELECT
                   parent_stop_id,
                   route_id,
                   ROW_NUMBER() OVER (PARTITION BY route_id ORDER BY seq, parent_stop_id)::INT as stop_sequence
               FROM unique_stops
           )
           SELECT parent_stop_id, route_id, stop_sequence FROM numbered
           ORDER BY route_id, stop_sequence"#,
    )
    .fetch_all(&mut *conn)
    .await?;

    let mut map: HashMap<String, Vec<(String, i32)>> = HashMap::new();
    for (parent_stop_id, route_id, stop_sequence) in rows {
        map.entry(parent_stop_id)
            .or_default()
            .push((route_id, stop_sequence));
    }

    info!("Built stop-route mapping for {} physical stops.", map.len());
    Ok(map)
}

/// Integrate gtfs_stops into stations table (one record per physical stop per route)
/// Only processes parent stops (stops without parent_station) to avoid duplicates
async fn integrate_gtfs_stops_to_stations(
    conn: &mut PgConnection,
    stop_route_map: &HashMap<String, Vec<(String, i32)>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Only fetch parent stops (stops that have no parent_station)
    // These represent physical bus stops, child stops are just different poles
    let stops: Vec<GtfsStopRow> = sqlx::query_as(
        "SELECT * FROM gtfs_stops WHERE parent_station IS NULL OR parent_station = ''",
    )
    .fetch_all(&mut *conn)
    .await?;

    // Collapse the separate direction poles of one physical stop (feeds without a
    // parent_station hierarchy expose each pole as its own top-level stop).
    let station_g_cd_by_stop = build_bus_station_g_cd_map(&stops);
    let group_count = station_g_cd_by_stop
        .values()
        .copied()
        .collect::<HashSet<_>>()
        .len();
    info!(
        "Grouped {} bus stops into {} station groups.",
        stops.len(),
        group_count
    );

    let mut inserted_count = 0;

    for stop in &stops {
        let station_g_cd = station_g_cd_by_stop
            .get(&stop.stop_id)
            .copied()
            .unwrap_or_else(|| generate_bus_station_g_cd(&stop.stop_id));

        // Get routes for this parent stop (with stop_sequence)
        // The mapping now uses parent_stop_id as key
        let routes = match stop_route_map.get(&stop.stop_id) {
            Some(r) => r.clone(),
            None => continue, // Skip stops not on any route
        };

        // station_name_rn is the plain-ASCII spelling of the romanized name
        // (Tokyo), whereas station_name_r keeps the macrons (Tōkyō). Derive it
        // from stop_name_r so both bus and rail rows follow the same convention.
        let station_name_rn = stop.stop_name_r.as_deref().map(strip_macrons);

        // Create a station record for each route this physical stop serves
        for (route_id, stop_sequence) in &routes {
            let station_cd = generate_bus_station_cd(&stop.stop_id, route_id);
            let line_cd = generate_bus_line_cd(route_id);

            sqlx::query(
                r#"INSERT INTO stations (
                    station_cd, station_g_cd, station_name, station_name_k,
                    station_name_r, station_name_rn, station_name_zh, station_name_ko,
                    line_cd, pref_cd, post, address, lon, lat,
                    open_ymd, close_ymd, e_status, e_sort, transport_type
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, 13, '', '', $10, $11,
                    '', '', 0, $12, 1
                )
                ON CONFLICT (station_cd) DO NOTHING"#,
            )
            .bind(station_cd)
            .bind(station_g_cd)
            .bind(&stop.stop_name)
            .bind(
                stop.stop_name_k
                    .as_ref()
                    .map(|k| hiragana_to_katakana(k))
                    .unwrap_or_else(|| stop.stop_name.clone()),
            )
            .bind(&stop.stop_name_r)
            .bind(&station_name_rn)
            .bind(&stop.stop_name_zh)
            .bind(&stop.stop_name_ko)
            .bind(line_cd)
            .bind(stop.stop_lon)
            .bind(stop.stop_lat)
            .bind(stop_sequence)
            .execute(&mut *conn)
            .await?;

            inserted_count += 1;
        }
    }

    info!(
        "Integrated {} station records from {} GTFS stops.",
        inserted_count,
        stops.len()
    );
    Ok(())
}

/// Update cross-references in GTFS tables (gtfs_stops.station_cd, gtfs_routes.line_cd)
async fn update_gtfs_crossreferences(
    conn: &mut PgConnection,
    stop_route_map: &HashMap<String, Vec<(String, i32)>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Update gtfs_stops with primary station_cd (using first route)
    // Updates both parent stops and their child stops with the same station_cd
    for (parent_stop_id, routes) in stop_route_map {
        if let Some((route_id, _)) = routes.first() {
            let station_cd = generate_bus_station_cd(parent_stop_id, route_id);
            // Update parent stop and all its children
            sqlx::query(
                "UPDATE gtfs_stops SET station_cd = $1 WHERE stop_id = $2 OR parent_station = $2",
            )
            .bind(station_cd)
            .bind(parent_stop_id)
            .execute(&mut *conn)
            .await?;
        }
    }

    info!("Updated GTFS cross-references.");
    Ok(())
}

/// Register each (route_id, shape_id) trip variation as a TrainType row
/// (`types.kind = BUS_ROUTE_KIND`) plus its stops in `station_station_types`.
///
/// One shape == one operational pattern of the bus line (e.g. for 池86: フルループ /
/// サンシャインシティ経由 / 新宿伊勢丹前止まりの短ターン). Clients can use the
/// resulting TrainTypes to switch between these patterns the same way a rail line
/// switches between のぞみ / ひかり / こだま.
///
/// `station_station_types` rows are inserted in `stop_sequence` order so that the
/// SERIAL `id` column preserves the trip ordering when read back via the
/// `ORDER BY sst.id` query path used by the rail TrainType code.
async fn integrate_gtfs_trip_variations_to_types(
    conn: &mut PgConnection,
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(sqlx::FromRow)]
    struct VariationRow {
        route_id: String,
        shape_id: String,
        representative_trip_id: String,
        direction_id: Option<i32>,
        trip_headsign: Option<String>,
        route_short_name: Option<String>,
        route_long_name: Option<String>,
        // English/romanized fallback for route_long_name (used when the trip has no
        // headsign and we fall back to the long route name for the JA type_name).
        route_long_name_r: Option<String>,
        route_color: Option<String>,
        stop_count: i64,
        first_stop_name: Option<String>,
        first_stop_name_r: Option<String>,
        // Last stop's name (JA + roman). The JA value lets us check whether the
        // headsign maps to the trip's terminal stop so we can borrow that stop's
        // romanized name as the headsign roman.
        last_stop_name: Option<String>,
        last_stop_name_r: Option<String>,
        is_loop: Option<bool>,
        // Sorted, '|'-joined list of distinct parent_stop_ids that this shape visits.
        // Shapes that share the same `stop_set_key` cover the exact same physical
        // stops — typically the up/down directions of the same physical route.
        // We collapse them into a single TrainType with `direction = Both`.
        stop_set_key: Option<String>,
    }

    // One representative trip per (route_id, shape_id). Pick by trip_id for determinism.
    // stop_count is the representative trip's stop_times count; ordering variations by it
    // (DESC) lets the longer / more comprehensive shapes get the un-suffixed name when we
    // disambiguate duplicate trip_headsigns.
    //
    // first_stop_name / is_loop are used by the type_name builder below to distinguish
    // multiple shapes that share the same trip_headsign — e.g. 池86 has 3 shapes whose
    // headsign is "池袋駅東口" (full loop, short-turn from 新宿伊勢丹前, サンシャインシティ
    // 発の延伸) — by prefixing with the starting stop or marking circular trips.
    //
    // stop_set_key is used to detect "same stops, different direction" shape pairs
    // (e.g. shape A: 池袋→新宿伊勢丹前 and shape B: 新宿伊勢丹前→池袋 visit the same
    // 13 stops) and fold them into one TrainType marked as bidirectional.
    let variations: Vec<VariationRow> = sqlx::query_as(
        r#"WITH per_variation AS (
               SELECT DISTINCT ON (gt.route_id, gt.shape_id)
                   gt.route_id,
                   gt.shape_id,
                   gt.trip_id AS representative_trip_id,
                   gt.direction_id,
                   gt.trip_headsign
               FROM gtfs_trips gt
               WHERE gt.shape_id IS NOT NULL
               ORDER BY gt.route_id, gt.shape_id, gt.trip_id
           ),
           endpoints AS (
               SELECT
                   pv.representative_trip_id,
                   (SELECT COALESCE(gs.parent_station, gs.stop_id)
                      FROM gtfs_stop_times gst
                      JOIN gtfs_stops gs ON gs.stop_id = gst.stop_id
                     WHERE gst.trip_id = pv.representative_trip_id
                     ORDER BY gst.stop_sequence ASC LIMIT 1) AS first_parent_id,
                   (SELECT gs.stop_name
                      FROM gtfs_stop_times gst
                      JOIN gtfs_stops gs ON gs.stop_id = gst.stop_id
                     WHERE gst.trip_id = pv.representative_trip_id
                     ORDER BY gst.stop_sequence ASC LIMIT 1) AS first_stop_name,
                   (SELECT gs.stop_name_r
                      FROM gtfs_stop_times gst
                      JOIN gtfs_stops gs ON gs.stop_id = gst.stop_id
                     WHERE gst.trip_id = pv.representative_trip_id
                     ORDER BY gst.stop_sequence ASC LIMIT 1) AS first_stop_name_r,
                   (SELECT COALESCE(gs.parent_station, gs.stop_id)
                      FROM gtfs_stop_times gst
                      JOIN gtfs_stops gs ON gs.stop_id = gst.stop_id
                     WHERE gst.trip_id = pv.representative_trip_id
                     ORDER BY gst.stop_sequence DESC LIMIT 1) AS last_parent_id,
                   (SELECT gs.stop_name
                      FROM gtfs_stop_times gst
                      JOIN gtfs_stops gs ON gs.stop_id = gst.stop_id
                     WHERE gst.trip_id = pv.representative_trip_id
                     ORDER BY gst.stop_sequence DESC LIMIT 1) AS last_stop_name,
                   (SELECT gs.stop_name_r
                      FROM gtfs_stop_times gst
                      JOIN gtfs_stops gs ON gs.stop_id = gst.stop_id
                     WHERE gst.trip_id = pv.representative_trip_id
                     ORDER BY gst.stop_sequence DESC LIMIT 1) AS last_stop_name_r,
                   (SELECT string_agg(parent_id, '|' ORDER BY parent_id)
                      FROM (
                          SELECT DISTINCT COALESCE(gs.parent_station, gs.stop_id) AS parent_id
                            FROM gtfs_stop_times gst
                            JOIN gtfs_stops gs ON gs.stop_id = gst.stop_id
                           WHERE gst.trip_id = pv.representative_trip_id
                      ) s) AS stop_set_key
               FROM per_variation pv
           )
           SELECT
               pv.route_id,
               pv.shape_id,
               pv.representative_trip_id,
               pv.direction_id,
               pv.trip_headsign,
               gr.route_short_name,
               gr.route_long_name,
               gr.route_long_name_r,
               gr.route_color,
               (SELECT COUNT(*) FROM gtfs_stop_times gst
                 WHERE gst.trip_id = pv.representative_trip_id)::bigint AS stop_count,
               ep.first_stop_name,
               ep.first_stop_name_r,
               ep.last_stop_name,
               ep.last_stop_name_r,
               (ep.first_parent_id IS NOT NULL
                 AND ep.first_parent_id = ep.last_parent_id) AS is_loop,
               ep.stop_set_key
           FROM per_variation pv
           JOIN gtfs_routes gr ON gr.route_id = pv.route_id
           LEFT JOIN endpoints ep ON ep.representative_trip_id = pv.representative_trip_id
           ORDER BY pv.route_id, stop_count DESC, pv.shape_id"#,
    )
    .fetch_all(&mut *conn)
    .await?;

    info!(
        "Found {} bus trip variations (pre-dedup).",
        variations.len()
    );

    // Fold variations that visit the exact same set of parent stops within the same
    // route. Such shapes typically correspond to the up/down directions of the same
    // physical route (e.g. 池86: shape 20003-1 池袋→新宿伊勢丹前 と shape 20003-2
    // 新宿伊勢丹前→池袋 は同じ 13 停留所を逆順で走る) — clients should see one
    // TrainType marked as bidirectional rather than two near-duplicates.
    //
    // The SQL already orders variations by `stop_count DESC`, so within each group the
    // first occurrence is the canonical representative; later ones contribute only
    // their direction_id and are otherwise discarded.
    let mut group_of: HashMap<(String, String), usize> = HashMap::new();
    let mut grouped_directions: Vec<Vec<Option<i32>>> = Vec::with_capacity(variations.len());
    let mut representatives: Vec<&VariationRow> = Vec::with_capacity(variations.len());
    for v in &variations {
        // Fall back to a key based on shape_id when stop_set_key is missing, so a row
        // without any stop_times still gets a unique slot.
        let key = (
            v.route_id.clone(),
            v.stop_set_key
                .clone()
                .unwrap_or_else(|| format!("__shape:{}", v.shape_id)),
        );
        match group_of.get(&key) {
            Some(&idx) => grouped_directions[idx].push(v.direction_id),
            None => {
                group_of.insert(key, representatives.len());
                grouped_directions.push(vec![v.direction_id]);
                representatives.push(v);
            }
        }
    }

    info!(
        "Folded into {} unique TrainTypes (same-stop / opposite-direction pairs merged).",
        representatives.len()
    );

    // For circular trips, compute a "経由地" (via stop) so the type_name can show
    // *what makes this loop distinct*, not just its start/end (which is the same
    // for a loop and matches the headsign). We prefer a stop that other same-headsign
    // loop variations in the route do NOT visit; if there is no sibling shape (or no
    // unique stop), we fall back to the stop at the trip's midpoint.
    let loop_trip_ids: Vec<String> = representatives
        .iter()
        .filter(|v| v.is_loop.unwrap_or(false))
        .map(|v| v.representative_trip_id.clone())
        .collect();

    // trip_id -> ordered (parent_id, stop_name, stop_name_r) for that trip, deduped by parent
    // (first stop_sequence wins, so a loop's terminal duplicate is dropped). stop_name_r is
    // carried through so the romanized type_name can borrow the via stop's English label.
    let stops_per_loop_trip: HashMap<String, Vec<(String, String, Option<String>)>> =
        if loop_trip_ids.is_empty() {
            HashMap::new()
        } else {
            let rows: Vec<(String, String, String, Option<String>, i32)> = sqlx::query_as(
                r#"SELECT DISTINCT ON (gst.trip_id, COALESCE(gs.parent_station, gs.stop_id))
                       gst.trip_id,
                       COALESCE(gs.parent_station, gs.stop_id) AS parent_stop_id,
                       gs.stop_name,
                       gs.stop_name_r,
                       gst.stop_sequence
                   FROM gtfs_stop_times gst
                   JOIN gtfs_stops gs ON gst.stop_id = gs.stop_id
                   WHERE gst.trip_id = ANY($1)
                   ORDER BY gst.trip_id, COALESCE(gs.parent_station, gs.stop_id), gst.stop_sequence"#,
            )
            .bind(&loop_trip_ids)
            .fetch_all(&mut *conn)
            .await?;

            // (parent_id, stop_name, stop_name_r, stop_sequence) tuples bucketed by trip_id.
            type StopRow = (String, String, Option<String>, i32);
            let mut buckets: HashMap<String, Vec<StopRow>> = HashMap::new();
            for (trip_id, parent_id, stop_name, stop_name_r, seq) in rows {
                buckets
                    .entry(trip_id)
                    .or_default()
                    .push((parent_id, stop_name, stop_name_r, seq));
            }
            buckets
                .into_iter()
                .map(|(trip_id, mut stops)| {
                    stops.sort_by_key(|(_, _, _, seq)| *seq);
                    (
                        trip_id,
                        stops
                            .into_iter()
                            .map(|(p, n, nr, _)| (p, n, nr))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect()
        };

    // Compute the headsign string the naming loop will use (same priority chain).
    let headsign_for = |v: &VariationRow| -> Option<String> {
        v.trip_headsign
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| v.route_long_name.clone().filter(|s| !s.is_empty()))
            .or_else(|| v.route_short_name.clone().filter(|s| !s.is_empty()))
            .or_else(|| v.first_stop_name.clone().filter(|s| !s.is_empty()))
    };

    // Group loop reps by (route_id, headsign) so we can find sibling shapes.
    let mut loop_groups: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (rep_idx, v) in representatives.iter().enumerate() {
        if !v.is_loop.unwrap_or(false) {
            continue;
        }
        if let Some(hs) = headsign_for(v) {
            loop_groups
                .entry((v.route_id.clone(), hs))
                .or_default()
                .push(rep_idx);
        }
    }

    // For each loop rep, pick a via-stop (prefer unique-to-shape stops near midpoint).
    // We carry both the JA name and the romanized name so the parallel type_name_r
    // can show the same stop as `via X`.
    let mut via_for_rep: HashMap<usize, String> = HashMap::new();
    let mut via_roman_for_rep: HashMap<usize, String> = HashMap::new();
    for ((_route_id, headsign_str), rep_idxs) in &loop_groups {
        for (pos, &idx) in rep_idxs.iter().enumerate() {
            let my_stops =
                match stops_per_loop_trip.get(&representatives[idx].representative_trip_id) {
                    Some(s) if s.len() >= 3 => s,
                    _ => continue,
                };

            // Stops in my shape not visited by any sibling shape with the same headsign.
            let unique_stops: Vec<&(String, String, Option<String>)> = my_stops
                .iter()
                .filter(|(parent, _, _)| {
                    rep_idxs.iter().enumerate().all(|(j, &other_idx)| {
                        if j == pos {
                            return true;
                        }
                        stops_per_loop_trip
                            .get(&representatives[other_idx].representative_trip_id)
                            .map(|other| !other.iter().any(|(p, _, _)| p == parent))
                            .unwrap_or(true)
                    })
                })
                .collect();

            let mid = my_stops.len() / 2;
            let picked: Option<(String, Option<String>)> = if !unique_stops.is_empty() {
                let mid_i = mid as i64;
                unique_stops
                    .iter()
                    .min_by_key(|(parent, _, _)| {
                        let pos_in_my = my_stops
                            .iter()
                            .position(|(p, _, _)| p == parent)
                            .unwrap_or(0) as i64;
                        (pos_in_my - mid_i).abs()
                    })
                    .map(|(_, name, name_r)| (name.clone(), name_r.clone()))
            } else {
                let (_, name, name_r) = &my_stops[mid];
                Some((name.clone(), name_r.clone()))
            };

            if let Some((name, name_r)) = picked {
                // Skip if the via name is the same as the headsign — adds no info.
                if name != *headsign_str {
                    via_for_rep.insert(idx, name);
                    if let Some(nr) = name_r.filter(|s| !s.is_empty()) {
                        via_roman_for_rep.insert(idx, nr);
                    }
                }
            }
        }
    }

    // Disambiguate duplicate type_names within the same route by appending the stop count.
    let mut name_counter: HashMap<(String, String), i32> = HashMap::new();
    let mut variation_count = 0;
    let mut sst_inserted = 0;

    for (rep_idx, v) in representatives.iter().enumerate() {
        let type_cd = generate_bus_type_cd(&v.route_id, &v.shape_id);
        let line_group_cd = generate_bus_line_group_cd(&v.route_id, &v.shape_id);

        let headsign = v
            .trip_headsign
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| v.route_long_name.clone().filter(|s| !s.is_empty()))
            .or_else(|| v.route_short_name.clone().filter(|s| !s.is_empty()));
        let first_stop = v.first_stop_name.clone().filter(|s| !s.is_empty());

        // Romanized counterparts. trip_headsign has no _r column, so for the headsign
        // roman we either borrow the terminal stop's roman (when the JA headsign equals
        // the last stop name — the common case for loops and many one-way routes) or
        // fall back to the route's long-name roman when the JA headsign came from there.
        // If neither matches we leave it None, which collapses type_name_r to "" later.
        let headsign_r: Option<String> = {
            let ja = v.trip_headsign.as_deref().filter(|s| !s.is_empty());
            let last_name = v.last_stop_name.as_deref();
            let last_r = v.last_stop_name_r.as_deref().filter(|s| !s.is_empty());
            let first_r = v.first_stop_name_r.as_deref().filter(|s| !s.is_empty());
            let route_r = v.route_long_name_r.as_deref().filter(|s| !s.is_empty());
            let route_long = v.route_long_name.as_deref().filter(|s| !s.is_empty());

            match (ja, last_name, last_r) {
                (Some(hs), Some(ln), Some(r)) if hs == ln => Some(r.to_string()),
                _ => {
                    // For loops, first stop == last stop, so the first stop's roman
                    // is a safe stand-in for the headsign roman even if the JA strings
                    // weren't byte-equal (whitespace/punctuation drift in translations).
                    if v.is_loop.unwrap_or(false) && ja.is_some() {
                        first_r.map(|s| s.to_string())
                    } else if ja.is_none() && route_long.is_some() {
                        // JA fell back to route_long_name — mirror with its roman.
                        route_r.map(|s| s.to_string())
                    } else {
                        None
                    }
                }
            }
        };
        let first_stop_r = v.first_stop_name_r.clone().filter(|s| !s.is_empty());

        // If multiple shapes share this stop set (= the route has explicit up and
        // down variants), `direction = Both` is the only honest answer. Otherwise
        // we keep whatever direction_id the representative trip carried — defaulting
        // to 0 (TrainDirection::Both) when GTFS leaves it NULL.
        let directions = &grouped_directions[rep_idx];
        let direction = if directions.len() > 1 {
            0 // Both — collapsed pair
        } else {
            v.direction_id.unwrap_or(0)
        };
        let is_bidirectional = directions.len() > 1;

        // Naming rule:
        // - 双方向ペア (同じ停留所集合, direction 0/1) → "<A> ⇔ <B>" (両端駅)
        // - 循環トリップ (始発 parent == 終点 parent) → "<headsign>（<経由地>経由・循環）"
        //   (経由地が取れなければ "<headsign> (循環)" にフォールバック)
        // - それ以外で始発名と headsign が異なる → "<first_stop> → <headsign>"
        // - 始発名と headsign が同じ / どちらか欠落 → headsign があれば "<headsign>"、
        //   無ければ "<first_stop>"
        // - すべて欠落 → "shape <shape_id>" でフォールバック
        let loop_name = |label: &str| -> String {
            match via_for_rep.get(&rep_idx) {
                Some(via) => format!("{}（{}経由・循環）", label, via),
                None => format!("{} (循環)", label),
            }
        };
        // Roman counterpart: "<label> via <via> (Loop)" / "<label> (Loop)". When the via
        // stop has no romanized name we drop the "via …" segment rather than fail the
        // whole roman name; an empty result here means the caller will return None for
        // the entire roman type_name (handled below).
        let loop_name_r = |label_r: &str| -> Option<String> {
            if label_r.is_empty() {
                return None;
            }
            match via_roman_for_rep.get(&rep_idx) {
                Some(via_r) => Some(format!("{} via {} (Loop)", label_r, via_r)),
                None if via_for_rep.contains_key(&rep_idx) => None,
                None => Some(format!("{} (Loop)", label_r)),
            }
        };

        let base_name = if is_bidirectional && !v.is_loop.unwrap_or(false) {
            match (first_stop.as_deref(), headsign.as_deref()) {
                (Some(first), Some(h)) if first != h => format!("{} ⇔ {}", first, h),
                (_, Some(h)) => h.to_string(),
                (Some(first), None) => first.to_string(),
                _ => format!("shape {}", v.shape_id),
            }
        } else {
            match (
                v.is_loop.unwrap_or(false),
                first_stop.as_deref(),
                headsign.as_deref(),
            ) {
                (true, _, Some(h)) => loop_name(h),
                (true, Some(first), None) => loop_name(first),
                (false, Some(first), Some(h)) if first != h => {
                    format!("{} → {}", first, h)
                }
                (_, _, Some(h)) => h.to_string(),
                (_, Some(first), None) => first.to_string(),
                _ => format!("shape {}", v.shape_id),
            }
        };

        // Build the romanized name following the same case structure. Any branch that
        // needs a missing roman piece returns None so the DB stores "" instead of a
        // partially-Japanese roman label.
        let base_name_r: Option<String> = if is_bidirectional && !v.is_loop.unwrap_or(false) {
            match (first_stop_r.as_deref(), headsign_r.as_deref()) {
                (Some(first_r), Some(h_r)) if first_r != h_r => {
                    Some(format!("{} ⇔ {}", first_r, h_r))
                }
                (_, Some(h_r)) => Some(h_r.to_string()),
                (Some(first_r), None) => Some(first_r.to_string()),
                _ => None,
            }
        } else {
            match (
                v.is_loop.unwrap_or(false),
                first_stop_r.as_deref(),
                headsign_r.as_deref(),
            ) {
                (true, _, Some(h_r)) => loop_name_r(h_r),
                (true, Some(first_r), None) => loop_name_r(first_r),
                (false, Some(first_r), Some(h_r)) if first_r != h_r => {
                    Some(format!("{} → {}", first_r, h_r))
                }
                (_, _, Some(h_r)) => Some(h_r.to_string()),
                (_, Some(first_r), None) => Some(first_r.to_string()),
                _ => None,
            }
        };

        // Disambiguate same-name variations within a route by appending stop count.
        // stop_count DESC ordering in the SQL means the longest variation wins the
        // un-suffixed name; shorter ones get "(N停)" appended.
        let counter = name_counter
            .entry((v.route_id.clone(), base_name.clone()))
            .or_insert(0);
        *counter += 1;
        let need_suffix = *counter > 1;
        let type_name = if need_suffix {
            format!("{} ({}停)", base_name, v.stop_count)
        } else {
            base_name.clone()
        };
        let type_name_r: String = match base_name_r {
            Some(roman) if need_suffix => format!("{} ({} stops)", roman, v.stop_count),
            Some(roman) => roman,
            None => String::new(),
        };

        let color = v
            .route_color
            .as_ref()
            .map(|c| {
                if c.starts_with('#') {
                    c.clone()
                } else {
                    format!("#{}", c)
                }
            })
            .unwrap_or_else(|| "#000000".to_string());

        sqlx::query(
            r#"INSERT INTO types (
                type_cd, type_name, type_name_k, type_name_r, type_name_zh, type_name_ko,
                color, direction, kind, priority
            ) VALUES ($1, $2, $2, $3, '', '', $4, $5, $6, 0)
            ON CONFLICT (type_cd) DO NOTHING"#,
        )
        .bind(type_cd)
        .bind(&type_name)
        .bind(&type_name_r)
        .bind(&color)
        .bind(direction)
        .bind(BUS_ROUTE_KIND)
        .execute(&mut *conn)
        .await?;
        variation_count += 1;

        // Fetch this trip's stops, deduplicated by parent_station, with the earliest
        // stop_sequence per parent. Then sort by sequence in Rust to insert in trip order.
        let stops: Vec<(String, i32)> = sqlx::query_as(
            r#"SELECT DISTINCT ON (COALESCE(gs.parent_station, gs.stop_id))
                   COALESCE(gs.parent_station, gs.stop_id) AS parent_stop_id,
                   gst.stop_sequence
               FROM gtfs_stop_times gst
               JOIN gtfs_stops gs ON gst.stop_id = gs.stop_id
               WHERE gst.trip_id = $1
               ORDER BY COALESCE(gs.parent_station, gs.stop_id), gst.stop_sequence"#,
        )
        .bind(&v.representative_trip_id)
        .fetch_all(&mut *conn)
        .await?;

        let mut sorted_stops = stops;
        sorted_stops.sort_by_key(|(_, seq)| *seq);

        // Single multi-row INSERT keeps stop_sequence → SERIAL id ordering intact
        // while avoiding the per-stop round-trip the previous loop incurred.
        // All three values are i32, so no SQL escaping is needed.
        if !sorted_stops.is_empty() {
            let mut sql = String::from(
                "INSERT INTO station_station_types (station_cd, type_cd, line_group_cd, pass) VALUES ",
            );
            let mut values: Vec<String> = Vec::with_capacity(sorted_stops.len());
            for (parent_stop_id, _) in &sorted_stops {
                let station_cd = generate_bus_station_cd(parent_stop_id, &v.route_id);
                values.push(format!(
                    "({}, {}, {}, 0)",
                    station_cd, type_cd, line_group_cd
                ));
            }
            sql.push_str(&values.join(","));
            sqlx::query(&sql).execute(&mut *conn).await?;
            sst_inserted += sorted_stops.len();
        }
    }

    info!(
        "Integrated {} bus trip variations as TrainTypes ({} station_station_types rows).",
        variation_count, sst_inserted
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_gtfs_time_valid() {
        assert_eq!(parse_gtfs_time("08:30:00"), Some("08:30:00".to_string()));
        assert_eq!(parse_gtfs_time("23:59:59"), Some("23:59:59".to_string()));
        // GTFS allows times > 24:00:00 for trips past midnight
        assert_eq!(parse_gtfs_time("25:30:00"), Some("25:30:00".to_string()));
        assert_eq!(parse_gtfs_time("00:00:00"), Some("00:00:00".to_string()));
    }

    #[test]
    fn test_parse_gtfs_time_invalid() {
        assert_eq!(parse_gtfs_time(""), None);
        assert_eq!(parse_gtfs_time("invalid"), None);
        assert_eq!(parse_gtfs_time("08:30"), None);
        assert_eq!(parse_gtfs_time("08:30:00:00"), None);
        assert_eq!(parse_gtfs_time("aa:bb:cc"), None);
    }

    #[test]
    fn test_load_gtfs_translations_field_value_keyed() {
        // Keio and the Tokyu community feeds key stop_name translations by
        // field_value (the Japanese stop_name) with an empty record_id, ship
        // ja-Hrkt as half-width katakana, and use the 7-column layout that
        // includes record_sub_id.
        let dir = std::env::temp_dir().join("stationapi_test_translations_fv");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("translations.txt"),
            "table_name,field_name,language,translation,record_id,record_sub_id,field_value\n\
             stops,stop_name,ja,西八王子駅南口,,,西八王子駅南口\n\
             stops,stop_name,ja-Hrkt,ﾆｼﾊﾁｵｳｼﾞｴｷﾐﾅﾐｸﾞﾁ,,,西八王子駅南口\n\
             stops,stop_name,en,Nishi-hachioji Sta. South,,,西八王子駅南口\n",
        )
        .unwrap();

        let translations = load_gtfs_translations(&dir).unwrap();
        let t = translations
            .get(&("stops".to_string(), "西八王子駅南口".to_string()))
            .expect("field_value-keyed translation should be found by stop_name");
        assert_eq!(t.ja_hrkt.as_deref(), Some("ﾆｼﾊﾁｵｳｼﾞｴｷﾐﾅﾐｸﾞﾁ"));
        assert_eq!(t.en.as_deref(), Some("Nishi-hachioji Sta. South"));
        // The half-width reading normalizes to full-width katakana for storage.
        assert_eq!(
            to_fullwidth_katakana(t.ja_hrkt.as_deref().unwrap()),
            "ニシハチオウジエキミナミグチ"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_gtfs_translations_record_id_keyed() {
        // Seibu keys translations by record_id (the stop_id, with a "-01" pole
        // suffix) in a 6-column layout without record_sub_id, and ships ja-Hrkt
        // as full-width katakana. The parent stop_id resolves too.
        let dir = std::env::temp_dir().join("stationapi_test_translations_rid");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("translations.txt"),
            "table_name,field_name,language,translation,record_id,field_value\n\
             stops,stop_name,ja,西武百貨店前,20001-01,\n\
             stops,stop_name,ja-Hrkt,セイブヒャッカテンマエ,20001-01,\n\
             stops,stop_name,en,Seibu hyakkaten-mae,20001-01,\n",
        )
        .unwrap();

        let translations = load_gtfs_translations(&dir).unwrap();
        let t = translations
            .get(&("stops".to_string(), "20001-01".to_string()))
            .expect("record_id-keyed translation should be found by stop_id");
        assert_eq!(t.en.as_deref(), Some("Seibu hyakkaten-mae"));
        // Parent stop_id (pole suffix dropped) also resolves to the reading.
        assert!(translations.contains_key(&("stops".to_string(), "20001".to_string())));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_odpt_time_adds_gtfs_seconds() {
        assert_eq!(parse_odpt_time("08:30"), Some("08:30:00".to_string()));
        assert_eq!(parse_odpt_time("8:03"), Some("08:03:00".to_string()));
        assert_eq!(parse_odpt_time("25:30:00"), Some("25:30:00".to_string()));
        assert_eq!(parse_odpt_time("08:60"), None);
        assert_eq!(parse_odpt_time("invalid"), None);
    }

    #[test]
    fn test_hiragana_to_katakana() {
        assert_eq!(hiragana_to_katakana("あいうえお"), "アイウエオ");
        assert_eq!(hiragana_to_katakana("かきくけこ"), "カキクケコ");
        assert_eq!(hiragana_to_katakana("しんじゅく"), "シンジュク");
        // Mixed content
        assert_eq!(hiragana_to_katakana("東京えき"), "東京エキ");
        // Already katakana - should remain unchanged
        assert_eq!(hiragana_to_katakana("アイウエオ"), "アイウエオ");
        // ASCII - should remain unchanged
        assert_eq!(hiragana_to_katakana("abc123"), "abc123");
    }

    #[test]
    fn test_fnv1a_hash_deterministic() {
        // Same input should always produce same output
        let hash1 = fnv1a_hash(b"test");
        let hash2 = fnv1a_hash(b"test");
        assert_eq!(hash1, hash2);

        // Different inputs should produce different outputs
        let hash3 = fnv1a_hash(b"test2");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_fnv1a_hash_known_values() {
        // Empty string
        assert_eq!(fnv1a_hash(b""), 0xcbf29ce484222325);
        // Known FNV-1a test vectors
        assert_eq!(fnv1a_hash(b"a"), 0xaf63dc4c8601ec8c);
    }

    #[test]
    fn test_generate_bus_line_cd() {
        let line_cd = generate_bus_line_cd("route_001");
        // Should be in range 100,000,000 to 109,999,999
        assert!(line_cd >= 100_000_000);
        assert!(line_cd < 110_000_000);

        // Should be deterministic
        let line_cd2 = generate_bus_line_cd("route_001");
        assert_eq!(line_cd, line_cd2);

        // Different route_id should produce different line_cd
        let line_cd3 = generate_bus_line_cd("route_002");
        assert_ne!(line_cd, line_cd3);
    }

    #[test]
    fn test_company_cd_for_gtfs_route() {
        assert_eq!(company_cd_for_gtfs_route("toei:route_001"), Some(119));
        assert_eq!(company_cd_for_gtfs_route("seibu:route_001"), Some(253));
        assert_eq!(company_cd_for_gtfs_route("keio:route_001"), Some(254));
        assert_eq!(company_cd_for_gtfs_route("tokyu_ota:route_001"), Some(255));
        assert_eq!(
            company_cd_for_gtfs_route("tokyu_shinagawa:route_001"),
            Some(255)
        );
        assert_eq!(
            company_cd_for_gtfs_route("tokyu_meguro:route_001"),
            Some(255)
        );
        assert_eq!(
            company_cd_for_gtfs_route("tokyu_json:TokyuBus.Route"),
            Some(255)
        );
        assert_eq!(company_cd_for_gtfs_route("unknown:route_001"), None);
    }

    #[test]
    fn test_generate_bus_station_cd() {
        let station_cd = generate_bus_station_cd("stop_001", "route_001");
        // Should be in range 200,000,000 to 299,999,999
        assert!(station_cd >= 200_000_000);
        assert!(station_cd < 300_000_000);

        // Should be deterministic
        let station_cd2 = generate_bus_station_cd("stop_001", "route_001");
        assert_eq!(station_cd, station_cd2);

        // Different stop_id or route_id should produce different station_cd
        let station_cd3 = generate_bus_station_cd("stop_002", "route_001");
        assert_ne!(station_cd, station_cd3);

        let station_cd4 = generate_bus_station_cd("stop_001", "route_002");
        assert_ne!(station_cd, station_cd4);
    }

    #[test]
    fn test_generate_bus_station_g_cd() {
        let station_g_cd = generate_bus_station_g_cd("stop_001");
        // Should be in range 200,000,000 to 299,999,999
        assert!(station_g_cd >= 200_000_000);
        assert!(station_g_cd < 300_000_000);

        // Should be deterministic
        let station_g_cd2 = generate_bus_station_g_cd("stop_001");
        assert_eq!(station_g_cd, station_g_cd2);

        // Same stop_id on different routes should have same station_g_cd
        // (station_g_cd is only based on stop_id, not route_id)
        let station_cd_route1 = generate_bus_station_cd("stop_001", "route_001");
        let station_cd_route2 = generate_bus_station_cd("stop_001", "route_002");
        assert_ne!(station_cd_route1, station_cd_route2); // station_cd differs
                                                          // but station_g_cd is the same for both
        assert_eq!(
            generate_bus_station_g_cd("stop_001"),
            generate_bus_station_g_cd("stop_001")
        );
    }

    fn stop_row(stop_id: &str, name: &str, lat: f64, lon: f64) -> GtfsStopRow {
        GtfsStopRow {
            stop_id: stop_id.to_string(),
            stop_code: None,
            stop_name: name.to_string(),
            stop_name_k: None,
            stop_name_r: None,
            stop_name_zh: None,
            stop_name_ko: None,
            stop_desc: None,
            stop_lat: lat,
            stop_lon: lon,
            zone_id: None,
            stop_url: None,
            location_type: None,
            parent_station: None,
            stop_timezone: None,
            wheelchair_boarding: None,
            platform_code: None,
        }
    }

    #[test]
    fn test_build_bus_station_g_cd_map_merges_nearby_poles() {
        let stops = vec![
            // Two direction poles of the same physical stop (~17 m apart).
            stop_row("100001-01", "南大通り【朝霞警察署】", 35.787006, 139.592117),
            stop_row("100001-02", "南大通り【朝霞警察署】", 35.786852, 139.592150),
            // A distinct stop that merely shares a common name, far away (~40 km).
            stop_row("A", "新田", 35.79, 139.59),
            stop_row("B", "新田", 35.65, 139.80),
        ];

        let map = build_bus_station_g_cd_map(&stops);

        // The two nearby poles collapse into one group.
        assert_eq!(map["100001-01"], map["100001-02"]);
        // The far-apart namesakes stay separate.
        assert_ne!(map["A"], map["B"]);
        // The merged group is stable and equals the representative (min stop_id).
        assert_eq!(map["100001-01"], generate_bus_station_g_cd("100001-01"));
        // A lone stop keeps the exact station_g_cd it had before grouping.
        assert_eq!(map["A"], generate_bus_station_g_cd("A"));
    }

    #[test]
    fn test_generate_bus_type_cd() {
        let type_cd = generate_bus_type_cd("route_001", "shape_A");
        // Should be in range 100,000,000 to 199,999,999
        assert!(type_cd >= 100_000_000);
        assert!(type_cd < 200_000_000);

        // Deterministic
        assert_eq!(type_cd, generate_bus_type_cd("route_001", "shape_A"));

        // Different (route_id, shape_id) should produce different type_cd
        assert_ne!(type_cd, generate_bus_type_cd("route_001", "shape_B"));
        assert_ne!(type_cd, generate_bus_type_cd("route_002", "shape_A"));
    }

    #[test]
    fn test_generate_bus_line_group_cd_distinct_from_type_cd() {
        // line_group_cd and type_cd both live in 100M+ space; make sure the same
        // (route_id, shape_id) maps them to different values so the FK + grouping
        // semantics are independent.
        let route = "152";
        let shape = "20007-3";
        assert_ne!(
            generate_bus_type_cd(route, shape),
            generate_bus_line_group_cd(route, shape)
        );
        // Deterministic
        assert_eq!(
            generate_bus_line_group_cd(route, shape),
            generate_bus_line_group_cd(route, shape)
        );
    }

    #[test]
    fn test_is_bus_feature_disabled() {
        // This test depends on environment variable, so we just verify it doesn't panic
        let _ = is_bus_feature_disabled();
    }

    #[test]
    fn test_gtfs_feeds() {
        assert_eq!(
            GTFS_FEEDS.iter().map(|feed| feed.id).collect::<Vec<_>>(),
            vec![
                "toei",
                "seibu",
                "tokyu_ota",
                "tokyu_shinagawa",
                "tokyu_meguro",
                "keio"
            ]
        );

        let keio = GTFS_FEEDS.iter().find(|feed| feed.id == "keio").unwrap();
        assert_eq!(keio.name, "Keio Bus");
        assert_eq!(keio.path, "data/KeioBus-GTFS");
        // ODPT's versioned files endpoint 404s without a `date` selector; `current`
        // keeps the URL pinned to the timetable in effect today.
        assert_eq!(
            keio.url,
            "https://api.odpt.org/api/v4/files/odpt/KeioBus/AllLines.zip?date=current"
        );
        assert!(keio.requires_consumer_key);

        for feed in GTFS_FEEDS
            .iter()
            .filter(|feed| feed.id.starts_with("tokyu_"))
        {
            assert!(feed.url.contains("date=current"));
            assert!(feed.requires_consumer_key);
        }
    }

    #[test]
    fn test_append_consumer_key() {
        // URL without an existing query string uses `?`.
        assert_eq!(
            append_consumer_key(
                "https://api.odpt.org/api/v4/files/SeibuBus/data/SeibuBus-GTFS.zip",
                "TOKEN"
            ),
            "https://api.odpt.org/api/v4/files/SeibuBus/data/SeibuBus-GTFS.zip?acl:consumerKey=TOKEN"
        );
        // URL that already carries a query string (e.g. `?date=current`) uses `&`
        // so the consumer key does not clobber the existing parameter.
        assert_eq!(
            append_consumer_key(
                "https://api.odpt.org/api/v4/files/odpt/KeioBus/AllLines.zip?date=current",
                "TOKEN"
            ),
            "https://api.odpt.org/api/v4/files/odpt/KeioBus/AllLines.zip?date=current&acl:consumerKey=TOKEN"
        );
    }

    #[test]
    fn test_gtfs_download_url_preserves_existing_query() {
        let feed = GTFS_FEEDS
            .iter()
            .find(|feed| feed.id == "tokyu_ota")
            .unwrap();
        assert_eq!(
            append_consumer_key(feed.url, "test-token"),
            "https://api.odpt.org/api/v4/files/odpt/TokyuBus/tokyubus_community_OtaCity.zip?date=current&acl:consumerKey=test-token"
        );

        let feed = GTFS_FEEDS.iter().find(|feed| feed.id == "seibu").unwrap();
        assert_eq!(
            append_consumer_key(feed.url, "test-token"),
            "https://api.odpt.org/api/v4/files/SeibuBus/data/SeibuBus-GTFS.zip?acl:consumerKey=test-token"
        );
    }

    #[test]
    fn test_tokyu_odpt_json_deserialization_and_id_conversion() {
        let pattern: OdptBusroutePattern = serde_json::from_str(
            r#"{
                "owl:sameAs":"odpt.BusroutePattern:TokyuBus.Sh11.1",
                "dc:title":"渋11 田園調布駅行き",
                "odpt:operator":"odpt.Operator:TokyuBus",
                "odpt:busroute":"odpt.Busroute:TokyuBus.Sh11",
                "odpt:direction":"2"
            }"#,
        )
        .unwrap();
        assert_eq!(
            scoped_tokyu_odpt_id(&pattern.same_as),
            "tokyu_json:TokyuBus.Sh11.1"
        );
        assert_eq!(tokyu_route_name(&pattern.title), "渋11");
        assert_eq!(odpt_direction_id(pattern.direction.as_deref()), Some(1));

        let stop: OdptBusstopPole = serde_json::from_str(
            r#"{
                "owl:sameAs":"odpt.BusstopPole:TokyuBus.123.a",
                "dc:title":"渋谷駅",
                "odpt:kana":"しぶやえき",
                "odpt:busstopPoleNumber":"01",
                "odpt:operator":["odpt.Operator:TokyuBus"]
            }"#,
        )
        .unwrap();
        assert_eq!(stop.lat, 0.0);
        assert_eq!(stop.lon, 0.0);
        assert_eq!(stop.operators, vec![TOKYU_ODPT_OPERATOR]);
    }

    #[test]
    fn test_tokyu_odpt_streaming_filter_discards_other_operators() {
        let json = r#"[
            {
                "owl:sameAs":"odpt.BusroutePattern:TokyuBus.Sh11.1",
                "dc:title":"渋11",
                "odpt:operator":"odpt.Operator:TokyuBus",
                "odpt:busroute":"odpt.Busroute:TokyuBus.Sh11"
            },
            {
                "owl:sameAs":"odpt.BusroutePattern:Other.1",
                "dc:title":"Other",
                "odpt:operator":"odpt.Operator:Other",
                "odpt:busroute":"odpt.Busroute:Other.1"
            }
        ]"#;
        let selected: Vec<OdptBusroutePattern> =
            read_tokyu_odpt_items_from_reader(json.as_bytes()).unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].operator, TOKYU_ODPT_OPERATOR);
    }

    #[test]
    #[ignore = "requires ODPT_ACCESS_TOKEN and downloads the large ODPT bus dumps"]
    fn test_live_tokyu_odpt_download_and_filter() {
        let _ = dotenv::from_filename(".env.local");
        let data = download_tokyu_odpt_data().unwrap();
        assert!(!data.patterns.is_empty());
        assert!(!data.stops.is_empty());
        assert!(!data.timetables.is_empty());
        assert!(data
            .patterns
            .iter()
            .all(|pattern| pattern.operator == TOKYU_ODPT_OPERATOR));
        assert!(data.stops.iter().all(|stop| stop
            .operators
            .iter()
            .any(|operator| operator == TOKYU_ODPT_OPERATOR)));
        assert!(data
            .timetables
            .iter()
            .all(|timetable| timetable.operator == TOKYU_ODPT_OPERATOR));
    }

    #[test]
    fn test_tokyu_community_routes_are_excluded_from_json_conversion() {
        assert!(is_tokyu_community_route("odpt.Busroute:TokyuBus.Tamachan"));
        assert!(is_tokyu_community_route("odpt.Busroute:TokyuBus.Shinabasu"));
        assert!(is_tokyu_community_route("odpt.Busroute:TokyuBus.Sanma"));
        assert!(!is_tokyu_community_route("odpt.Busroute:TokyuBus.Sh11"));
    }

    #[test]
    fn test_escape_sql_string_single_quotes() {
        // Single quotes should be doubled
        assert_eq!(escape_sql_string("O'Brien"), "O''Brien");
        assert_eq!(escape_sql_string("It's"), "It''s");
        assert_eq!(escape_sql_string("''"), "''''");
        assert_eq!(escape_sql_string("a'b'c"), "a''b''c");
    }

    #[test]
    fn test_escape_sql_string_backslashes() {
        // Backslashes should be doubled
        assert_eq!(escape_sql_string(r"a\b"), r"a\\b");
        assert_eq!(escape_sql_string(r"\\"), r"\\\\");
        assert_eq!(escape_sql_string(r"path\to\file"), r"path\\to\\file");
    }

    #[test]
    fn test_escape_sql_string_combined() {
        // Both single quotes and backslashes
        assert_eq!(escape_sql_string(r"O'Brien\path"), r"O''Brien\\path");
        assert_eq!(escape_sql_string(r"\'"), r"\\''");
        // Order matters: backslash first, then single quote
        // Input: \' -> after backslash escape: \\' -> after quote escape: \\''
        assert_eq!(escape_sql_string(r"test\'value"), r"test\\''value");
    }

    #[test]
    fn test_escape_sql_string_no_escaping_needed() {
        // Strings without special characters should remain unchanged
        assert_eq!(escape_sql_string("hello"), "hello");
        assert_eq!(escape_sql_string("東京駅"), "東京駅");
        assert_eq!(escape_sql_string("abc123"), "abc123");
        assert_eq!(escape_sql_string(""), "");
    }

    #[test]
    fn test_escape_sql_string_unicode() {
        // Unicode characters should pass through unchanged
        assert_eq!(escape_sql_string("新宿駅"), "新宿駅");
        assert_eq!(escape_sql_string("カタカナ"), "カタカナ");
        // But special chars in unicode strings should still be escaped
        assert_eq!(escape_sql_string("新宿'駅"), "新宿''駅");
    }

    #[test]
    fn test_translation_struct_default() {
        // Test Translation struct initialization
        let translation = Translation {
            ja: Some("日本語".to_string()),
            ja_hrkt: Some("にほんご".to_string()),
            en: Some("Japanese".to_string()),
            zh: None,
            ko: None,
        };
        assert_eq!(translation.ja, Some("日本語".to_string()));
        assert_eq!(translation.ja_hrkt, Some("にほんご".to_string()));
        assert_eq!(translation.en, Some("Japanese".to_string()));
        assert!(translation.zh.is_none());
        assert!(translation.ko.is_none());
    }

    #[test]
    fn test_translation_all_none() {
        let translation = Translation {
            ja: None,
            ja_hrkt: None,
            en: None,
            zh: None,
            ko: None,
        };
        assert!(translation.ja.is_none());
        assert!(translation.ja_hrkt.is_none());
        assert!(translation.en.is_none());
        assert!(translation.zh.is_none());
        assert!(translation.ko.is_none());
    }

    #[test]
    fn test_date_parse_valid() {
        // Test GTFS date format (YYYYMMDD)
        let date = chrono::NaiveDate::parse_from_str("20240101", "%Y%m%d");
        assert!(date.is_ok());
        let date = date.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 1);

        // End of year
        let date = chrono::NaiveDate::parse_from_str("20231231", "%Y%m%d").unwrap();
        assert_eq!(date.year(), 2023);
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 31);
    }

    #[test]
    fn test_date_parse_invalid() {
        // Invalid formats
        assert!(chrono::NaiveDate::parse_from_str("2024-01-01", "%Y%m%d").is_err());
        assert!(chrono::NaiveDate::parse_from_str("01/01/2024", "%Y%m%d").is_err());
        assert!(chrono::NaiveDate::parse_from_str("invalid", "%Y%m%d").is_err());
        assert!(chrono::NaiveDate::parse_from_str("", "%Y%m%d").is_err());
        // Invalid date values
        assert!(chrono::NaiveDate::parse_from_str("20241301", "%Y%m%d").is_err()); // month 13
        assert!(chrono::NaiveDate::parse_from_str("20240132", "%Y%m%d").is_err());
        // day 32
    }

    #[test]
    fn test_generate_bus_line_cd_no_collision() {
        // Test that different route IDs produce different line_cds
        let mut line_cds = std::collections::HashSet::new();
        let route_ids = vec![
            "route_001",
            "route_002",
            "route_003",
            "route_100",
            "Toei_Bus_01",
            "Toei_Bus_02",
            "AB01",
            "AB02",
        ];
        for route_id in route_ids {
            let line_cd = generate_bus_line_cd(route_id);
            assert!(
                line_cds.insert(line_cd),
                "Collision detected for {}",
                route_id
            );
        }
    }

    #[test]
    fn test_generate_bus_station_cd_no_collision() {
        // Test that different stop_id/route_id combinations produce different station_cds
        let mut station_cds = std::collections::HashSet::new();
        let combinations = vec![
            ("stop_001", "route_001"),
            ("stop_001", "route_002"),
            ("stop_002", "route_001"),
            ("stop_002", "route_002"),
            ("Toei_Stop_A", "Toei_Bus_01"),
            ("Toei_Stop_B", "Toei_Bus_01"),
        ];
        for (stop_id, route_id) in combinations {
            let station_cd = generate_bus_station_cd(stop_id, route_id);
            assert!(
                station_cds.insert(station_cd),
                "Collision detected for ({}, {})",
                stop_id,
                route_id
            );
        }
    }

    #[test]
    fn test_hiragana_to_katakana_edge_cases() {
        // Empty string
        assert_eq!(hiragana_to_katakana(""), "");
        // Only punctuation
        assert_eq!(hiragana_to_katakana("。、"), "。、");
        // Mixed hiragana, katakana, kanji, ascii
        assert_eq!(
            hiragana_to_katakana("あいうアイウ漢字abc"),
            "アイウアイウ漢字abc"
        );
        // Small hiragana characters
        assert_eq!(hiragana_to_katakana("ぁぃぅぇぉ"), "ァィゥェォ");
        // Voiced/semi-voiced marks
        assert_eq!(hiragana_to_katakana("がぎぐげご"), "ガギグゲゴ");
        assert_eq!(hiragana_to_katakana("ぱぴぷぺぽ"), "パピプペポ");
    }

    #[test]
    fn test_fnv1a_hash_different_lengths() {
        // Different length inputs should produce different hashes
        let hash1 = fnv1a_hash(b"a");
        let hash2 = fnv1a_hash(b"aa");
        let hash3 = fnv1a_hash(b"aaa");
        assert_ne!(hash1, hash2);
        assert_ne!(hash2, hash3);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_fnv1a_hash_unicode() {
        // Unicode strings should hash correctly
        let hash1 = fnv1a_hash("新宿".as_bytes());
        let hash2 = fnv1a_hash("渋谷".as_bytes());
        assert_ne!(hash1, hash2);
        // Same string should produce same hash
        assert_eq!(fnv1a_hash("新宿".as_bytes()), fnv1a_hash("新宿".as_bytes()));
    }

    #[test]
    fn test_escape_sql_string_special_sequences() {
        // Test various special sequences that might cause issues
        assert_eq!(escape_sql_string("\\n"), "\\\\n");
        assert_eq!(escape_sql_string("\\t"), "\\\\t");
        assert_eq!(escape_sql_string("\\r"), "\\\\r");
        // Multiple consecutive special chars
        assert_eq!(escape_sql_string("'''"), "''''''");
        assert_eq!(escape_sql_string("\\\\\\"), "\\\\\\\\\\\\");
    }

    #[test]
    fn test_parse_gtfs_time_boundary() {
        // Test boundary values for GTFS time
        assert_eq!(parse_gtfs_time("00:00:00"), Some("00:00:00".to_string()));
        assert_eq!(parse_gtfs_time("23:59:59"), Some("23:59:59".to_string()));
        // GTFS allows times past midnight for overnight trips
        assert_eq!(parse_gtfs_time("24:00:00"), Some("24:00:00".to_string()));
        assert_eq!(parse_gtfs_time("25:30:00"), Some("25:30:00".to_string()));
        assert_eq!(parse_gtfs_time("48:00:00"), Some("48:00:00".to_string()));
    }

    #[test]
    fn test_parse_gtfs_time_with_leading_zeros() {
        assert_eq!(parse_gtfs_time("01:02:03"), Some("01:02:03".to_string()));
        assert_eq!(parse_gtfs_time("00:00:01"), Some("00:00:01".to_string()));
    }

    // ============================================================================
    // build_stop_route_mapping regression tests
    //
    // 実 PostgreSQL を前提とした回帰テスト。`build_stop_route_mapping()` が
    // 読み取る 3 テーブル (gtfs_trips / gtfs_stop_times / gtfs_stops) を
    // 隔離スキーマに最小構成で再現し、出力マッピングを assert する。
    //
    // 既定では `#[ignore]` で除外され、`integration-tests` feature を付けたとき
    // のみ実行される。ローカルでの実行例:
    //
    //     export TEST_DATABASE_URL=postgres://test:test@localhost/stationapi_test
    //     cargo test -p stationapi --features integration-tests \
    //         build_stop_route_mapping
    //
    // テストは並列実行可能。各テストが per-process カウンタとナノ秒タイムスタンプ
    // から生成したユニークなスキーマ名を使うので、複数スレッドで衝突しない。
    // パニックでテストが中断するとスキーマがクリーンアップされず残るが、
    // スキーマ名がユニークなので後続実行には影響しない (テスト DB を作り直す
    // 際にまとめて消えるので運用上問題ない)。
    // ============================================================================

    mod stop_route_mapping_fixtures {
        use sqlx::{Connection, Executor, PgConnection};
        use std::env;
        use std::sync::atomic::{AtomicU64, Ordering};

        static SCHEMA_COUNTER: AtomicU64 = AtomicU64::new(0);

        pub fn unique_schema_name() -> String {
            let id = SCHEMA_COUNTER.fetch_add(1, Ordering::SeqCst);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            format!("brsm_test_{nanos}_{id}")
        }

        pub async fn open_conn() -> PgConnection {
            let database_url = env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://test:test@localhost/stationapi_test".to_string());
            PgConnection::connect(&database_url)
                .await
                .expect("Failed to connect to test database. Set TEST_DATABASE_URL.")
        }

        /// 隔離スキーマと最小 GTFS テーブルを作成し、search_path を切り替える。
        /// 戻り値のスキーマ名は呼び出し側が `drop_schema` で破棄すること。
        pub async fn setup_schema(conn: &mut PgConnection) -> String {
            let schema = unique_schema_name();
            conn.execute(format!("CREATE SCHEMA \"{schema}\"").as_str())
                .await
                .expect("create schema");
            conn.execute(format!("SET search_path TO \"{schema}\"").as_str())
                .await
                .expect("set search_path");
            conn.execute(
                r#"
                CREATE TABLE gtfs_stops (
                    stop_id VARCHAR(255) PRIMARY KEY,
                    stop_name TEXT NOT NULL DEFAULT '',
                    stop_lat DOUBLE PRECISION NOT NULL DEFAULT 0,
                    stop_lon DOUBLE PRECISION NOT NULL DEFAULT 0,
                    parent_station VARCHAR(255)
                );
                CREATE TABLE gtfs_trips (
                    trip_id VARCHAR(255) PRIMARY KEY,
                    route_id VARCHAR(255) NOT NULL,
                    service_id VARCHAR(255) NOT NULL DEFAULT 'svc',
                    direction_id INTEGER,
                    shape_id VARCHAR(255)
                );
                CREATE TABLE gtfs_stop_times (
                    id SERIAL PRIMARY KEY,
                    trip_id VARCHAR(255) NOT NULL REFERENCES gtfs_trips(trip_id),
                    stop_id VARCHAR(255) NOT NULL REFERENCES gtfs_stops(stop_id),
                    stop_sequence INTEGER NOT NULL,
                    shape_dist_traveled DOUBLE PRECISION
                );
                "#,
            )
            .await
            .expect("create gtfs tables");
            schema
        }

        pub async fn drop_schema(conn: &mut PgConnection, schema: &str) {
            let _ = conn.execute("RESET search_path").await;
            let _ = conn
                .execute(format!("DROP SCHEMA \"{schema}\" CASCADE").as_str())
                .await;
        }

        pub async fn insert_stop(conn: &mut PgConnection, stop_id: &str) {
            sqlx::query("INSERT INTO gtfs_stops (stop_id, parent_station) VALUES ($1, NULL)")
                .bind(stop_id)
                .execute(&mut *conn)
                .await
                .expect("insert stop");
        }

        pub async fn insert_pole(conn: &mut PgConnection, stop_id: &str, parent_station: &str) {
            sqlx::query("INSERT INTO gtfs_stops (stop_id, parent_station) VALUES ($1, $2)")
                .bind(stop_id)
                .bind(parent_station)
                .execute(&mut *conn)
                .await
                .expect("insert pole");
        }

        /// trip と対応する stop_times を一括投入する。`stops` の要素は
        /// `(stop_id, shape_dist_traveled)` で、`stop_sequence` は要素順に
        /// 1 から自動採番される。
        pub async fn insert_trip_with_stops(
            conn: &mut PgConnection,
            trip_id: &str,
            route_id: &str,
            direction_id: Option<i32>,
            shape_id: Option<&str>,
            stops: &[(&str, Option<f64>)],
        ) {
            sqlx::query(
                "INSERT INTO gtfs_trips (trip_id, route_id, direction_id, shape_id) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(trip_id)
            .bind(route_id)
            .bind(direction_id)
            .bind(shape_id)
            .execute(&mut *conn)
            .await
            .expect("insert trip");
            for (idx, (stop_id, dist)) in stops.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO gtfs_stop_times \
                       (trip_id, stop_id, stop_sequence, shape_dist_traveled) \
                     VALUES ($1, $2, $3, $4)",
                )
                .bind(trip_id)
                .bind(*stop_id)
                .bind((idx as i32) + 1)
                .bind(*dist)
                .execute(&mut *conn)
                .await
                .expect("insert stop_time");
            }
        }
    }

    /// マッピングから特定の route_id について `(parent_stop_id, seq)` を
    /// `seq` 昇順で抜き出すヘルパ。
    fn collect_route(
        mapping: &HashMap<String, Vec<(String, i32)>>,
        route_id: &str,
    ) -> Vec<(String, i32)> {
        let mut out: Vec<(String, i32)> = mapping
            .iter()
            .filter_map(|(stop, entries)| {
                entries
                    .iter()
                    .find(|(r, _)| r == route_id)
                    .map(|(_, seq)| (stop.clone(), *seq))
            })
            .collect();
        out.sort_by_key(|(_, seq)| *seq);
        out
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_build_stop_route_mapping_ike86_canonical_shape() {
        // 池86 風シナリオ: 同一 route_id に複数 shape_id があり、最多停留所の
        // shape は direction_id=NULL、唯一の direction_id=0 trip は短縮便。
        // canonical_shape は stops 数を最優先するので shape_full が選ばれ、
        // main_trip も trip_full になるべき。
        // (PR #1515 で導入された挙動の lock-in テスト。)
        let mut conn = stop_route_mapping_fixtures::open_conn().await;
        let schema = stop_route_mapping_fixtures::setup_schema(&mut conn).await;
        for s in ["S01", "S02", "S03", "S04", "S05"] {
            stop_route_mapping_fixtures::insert_stop(&mut conn, s).await;
        }
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_full",
            "IKE86",
            None,
            Some("shape_full"),
            &[
                ("S01", Some(0.0)),
                ("S02", Some(100.0)),
                ("S03", Some(200.0)),
                ("S04", Some(300.0)),
                ("S05", Some(400.0)),
            ],
        )
        .await;
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_short",
            "IKE86",
            Some(0),
            Some("shape_short"),
            &[("S01", Some(0.0)), ("S02", Some(50.0))],
        )
        .await;
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_inbound",
            "IKE86",
            Some(1),
            Some("shape_in"),
            &[
                ("S05", Some(0.0)),
                ("S04", Some(100.0)),
                ("S03", Some(200.0)),
                ("S02", Some(300.0)),
                ("S01", Some(400.0)),
            ],
        )
        .await;

        let mapping = build_stop_route_mapping(&mut conn).await.unwrap();
        let actual = collect_route(&mapping, "IKE86");
        stop_route_mapping_fixtures::drop_schema(&mut conn, &schema).await;

        assert_eq!(
            actual,
            vec![
                ("S01".to_string(), 1),
                ("S02".to_string(), 2),
                ("S03".to_string(), 3),
                ("S04".to_string(), 4),
                ("S05".to_string(), 5),
            ],
            "canonical_shape は stop 数最多の shape_full を選び、\
             main_trip もそこから取られるべき"
        );
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_build_stop_route_mapping_canonical_shape_direction_tiebreak() {
        // stops 数と shape_dist_traveled が並ぶ場合、canonical_shape の
        // タイブレークで direction_id=0 が NULL/1 に勝つことを確認する。
        // shape_a は逆順、shape_b は順方向。canonical=shape_b になれば
        // 結果は順方向になる。
        let mut conn = stop_route_mapping_fixtures::open_conn().await;
        let schema = stop_route_mapping_fixtures::setup_schema(&mut conn).await;
        for s in ["S01", "S02", "S03"] {
            stop_route_mapping_fixtures::insert_stop(&mut conn, s).await;
        }
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_a",
            "R1",
            None,
            Some("shape_a"),
            &[
                ("S03", Some(0.0)),
                ("S02", Some(100.0)),
                ("S01", Some(200.0)),
            ],
        )
        .await;
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_b",
            "R1",
            Some(0),
            Some("shape_b"),
            &[
                ("S01", Some(0.0)),
                ("S02", Some(100.0)),
                ("S03", Some(200.0)),
            ],
        )
        .await;

        let mapping = build_stop_route_mapping(&mut conn).await.unwrap();
        let actual = collect_route(&mapping, "R1");
        stop_route_mapping_fixtures::drop_schema(&mut conn, &schema).await;

        assert_eq!(
            actual,
            vec![
                ("S01".to_string(), 1),
                ("S02".to_string(), 2),
                ("S03".to_string(), 3),
            ],
            "direction_id=0 の shape_b が canonical に選ばれ、順方向順序になるべき"
        );
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_build_stop_route_mapping_main_trip_prefers_direction_zero() {
        // canonical_shape を共有する 2 つの trip があるとき、main_trip は
        // direction_id=0 のものが選ばれる。逆方向の trip_dir1 と順方向の
        // trip_dir0 を同じ shape_main に乗せ、結果が順方向になることを確認。
        let mut conn = stop_route_mapping_fixtures::open_conn().await;
        let schema = stop_route_mapping_fixtures::setup_schema(&mut conn).await;
        for s in ["S01", "S02", "S03", "S04"] {
            stop_route_mapping_fixtures::insert_stop(&mut conn, s).await;
        }
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_dir1",
            "R2",
            Some(1),
            Some("shape_main"),
            &[
                ("S04", Some(0.0)),
                ("S03", Some(100.0)),
                ("S02", Some(200.0)),
                ("S01", Some(300.0)),
            ],
        )
        .await;
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_dir0",
            "R2",
            Some(0),
            Some("shape_main"),
            &[
                ("S01", Some(0.0)),
                ("S02", Some(100.0)),
                ("S03", Some(200.0)),
                ("S04", Some(300.0)),
            ],
        )
        .await;

        let mapping = build_stop_route_mapping(&mut conn).await.unwrap();
        let actual = collect_route(&mapping, "R2");
        stop_route_mapping_fixtures::drop_schema(&mut conn, &schema).await;

        assert_eq!(
            actual,
            vec![
                ("S01".to_string(), 1),
                ("S02".to_string(), 2),
                ("S03".to_string(), 3),
                ("S04".to_string(), 4),
            ],
            "main_trip は direction_id=0 の trip_dir0 を選び、結果は順方向になるべき"
        );
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_build_stop_route_mapping_variant_chain_recursive_interpolation() {
        // メイン系統に乗らない停留所が連鎖する場合、再帰 CTE で両端を解決して
        // 中央付近の位置に補間できることを確認する。
        //
        // main_trip: M1, M2, M3, M4, M5            (seq 1..5)
        // variant : M1, V1, V2, V3, M5
        //   - V1: prev=M1(main), next=V2(variant)  → direct prev: 1 + 0.5 = 1.5
        //   - V2: prev=V1(variant), next=V3(variant) → 再帰両端: (1+5)/2 = 3.0
        //   - V3: prev=V2(variant), next=M5(main)  → direct next: 5 - 0.5 = 4.5
        //
        // 番号付け後 (ORDER BY seq, parent_stop_id):
        //   M1=1, V1=2, M2=3, M3=4 ("M3" < "V2" の辞書順タイブレーク),
        //   V2=5, M4=6, V3=7, M5=8
        let mut conn = stop_route_mapping_fixtures::open_conn().await;
        let schema = stop_route_mapping_fixtures::setup_schema(&mut conn).await;
        for s in ["M1", "M2", "M3", "M4", "M5", "V1", "V2", "V3"] {
            stop_route_mapping_fixtures::insert_stop(&mut conn, s).await;
        }
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_main",
            "R3",
            Some(0),
            Some("shape_main"),
            &[
                ("M1", Some(0.0)),
                ("M2", Some(100.0)),
                ("M3", Some(200.0)),
                ("M4", Some(300.0)),
                ("M5", Some(400.0)),
            ],
        )
        .await;
        // shape_variant の max_dist は shape_main (400) より低くしておく。
        // 両 shape は同じ 5 停留所 だが MAX(shape_dist_traveled) が高い方が
        // canonical_shape のタイブレークで勝つため、ここで意図的に
        // shape_main を canonical にする。
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_variant",
            "R3",
            Some(0),
            Some("shape_variant"),
            &[
                ("M1", Some(0.0)),
                ("V1", Some(50.0)),
                ("V2", Some(150.0)),
                ("V3", Some(250.0)),
                ("M5", Some(350.0)),
            ],
        )
        .await;

        let mapping = build_stop_route_mapping(&mut conn).await.unwrap();
        let actual = collect_route(&mapping, "R3");
        stop_route_mapping_fixtures::drop_schema(&mut conn, &schema).await;

        assert_eq!(
            actual,
            vec![
                ("M1".to_string(), 1),
                ("V1".to_string(), 2),
                ("M2".to_string(), 3),
                ("M3".to_string(), 4),
                ("V2".to_string(), 5),
                ("M4".to_string(), 6),
                ("V3".to_string(), 7),
                ("M5".to_string(), 8),
            ],
            "V2 は再帰 CTE で両端 (M1, M5) を解決し、中央付近に挿入されるべき"
        );
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_build_stop_route_mapping_long_variant_chain_dynamic_limit() {
        // 12 連続の variant-only 停留所をはさむ系統で、再帰 CTE の深さ上限が
        // 動的に拡張されることを確認する (issue #1513)。
        //
        // 旧実装は `depth < 10` で打ち切るため、V11/V12 のうち両端が variant の
        // ものは prev_chain で M1 まで到達できず、`mtms.max_seq + 9999` の末尾
        // フォールバックや「next のみ解決」ブランチに落ち、メイン系統の末尾
        // 付近に押し出されてしまっていた。
        //
        // main_trip: M01..M15           (seq 1..15, shape_main, 15 unique stops)
        // variant : M01, V01..V12, M15  (shape_variant, 14 unique stops)
        //
        // canonical_shape は stop 数で shape_main が勝ち、M01..M15 はそのまま
        // main_trip_stops に並ぶ。V01 は prev=M01 で直接補間 (1.5)、V12 は
        // next=M15 で直接補間 (14.5)。中央の V02..V11 は両側 variant なので
        // prev_chain と next_chain の双方を踏破して M01/M15 を解決し、
        // (1+15)/2 + (prev_depth - next_depth) * 0.01 で 7.91..8.09 に並ぶ。
        let mut conn = stop_route_mapping_fixtures::open_conn().await;
        let schema = stop_route_mapping_fixtures::setup_schema(&mut conn).await;

        let main_stops: Vec<String> = (1..=15).map(|i| format!("M{:02}", i)).collect();
        let variant_stops: Vec<String> = (1..=12).map(|i| format!("V{:02}", i)).collect();
        for s in main_stops.iter().chain(variant_stops.iter()) {
            stop_route_mapping_fixtures::insert_stop(&mut conn, s).await;
        }

        let main_trip_stops: Vec<(&str, Option<f64>)> = main_stops
            .iter()
            .enumerate()
            .map(|(i, s)| (s.as_str(), Some((i as f64) * 100.0)))
            .collect();
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_main",
            "R6",
            Some(0),
            Some("shape_main"),
            &main_trip_stops,
        )
        .await;

        // variant: M01 → V01..V12 → M15。distance は main 側より短くしておく
        // (canonical の決定は stop 数優先なので shape_main が勝つ)。
        let mut variant_trip: Vec<(&str, Option<f64>)> = Vec::with_capacity(14);
        variant_trip.push(("M01", Some(0.0)));
        for (i, v) in variant_stops.iter().enumerate() {
            variant_trip.push((v.as_str(), Some(((i as f64) + 1.0) * 50.0)));
        }
        variant_trip.push(("M15", Some(((variant_stops.len() as f64) + 1.0) * 50.0)));
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_variant",
            "R6",
            Some(0),
            Some("shape_variant"),
            &variant_trip,
        )
        .await;

        let mapping = build_stop_route_mapping(&mut conn).await.unwrap();
        let actual = collect_route(&mapping, "R6");
        stop_route_mapping_fixtures::drop_schema(&mut conn, &schema).await;

        let expected: Vec<(String, i32)> = vec![
            ("M01", 1),
            ("V01", 2),
            ("M02", 3),
            ("M03", 4),
            ("M04", 5),
            ("M05", 6),
            ("M06", 7),
            ("M07", 8),
            ("V02", 9),
            ("V03", 10),
            ("V04", 11),
            ("V05", 12),
            ("V06", 13),
            ("M08", 14),
            ("V07", 15),
            ("V08", 16),
            ("V09", 17),
            ("V10", 18),
            ("V11", 19),
            ("M09", 20),
            ("M10", 21),
            ("M11", 22),
            ("M12", 23),
            ("M13", 24),
            ("M14", 25),
            ("V12", 26),
            ("M15", 27),
        ]
        .into_iter()
        .map(|(s, n)| (s.to_string(), n))
        .collect();

        assert_eq!(
            actual, expected,
            "12 段の variant チェーンでも V02..V11 が両端 (M01, M15) を再帰解決し、\
             メイン系統の中央 (M07..M09 付近) に補間されるべき。旧 `depth < 10` では \
             V11 が next のみ解決されて M14 付近 (~14.8) に流される回帰が発生する"
        );
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_build_stop_route_mapping_null_shape_dist_traveled() {
        // shape_dist_traveled が全行 NULL でも canonical/main の選択ロジックが
        // フォールバックして動作することを確認する。stops 数で shape_long が
        // canonical に選ばれ、その停留所順がそのまま結果になる。
        let mut conn = stop_route_mapping_fixtures::open_conn().await;
        let schema = stop_route_mapping_fixtures::setup_schema(&mut conn).await;
        for s in ["S01", "S02", "S03", "S04"] {
            stop_route_mapping_fixtures::insert_stop(&mut conn, s).await;
        }
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_short",
            "R4",
            Some(0),
            Some("shape_short"),
            &[("S01", None), ("S02", None), ("S03", None)],
        )
        .await;
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_long",
            "R4",
            Some(0),
            Some("shape_long"),
            &[("S01", None), ("S02", None), ("S03", None), ("S04", None)],
        )
        .await;

        let mapping = build_stop_route_mapping(&mut conn).await.unwrap();
        let actual = collect_route(&mapping, "R4");
        stop_route_mapping_fixtures::drop_schema(&mut conn, &schema).await;

        assert_eq!(
            actual,
            vec![
                ("S01".to_string(), 1),
                ("S02".to_string(), 2),
                ("S03".to_string(), 3),
                ("S04".to_string(), 4),
            ],
            "shape_dist_traveled が NULL でも stops 数で shape_long が \
             canonical に選ばれ、その順序が返るべき"
        );
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_build_stop_route_mapping_parent_station_grouping() {
        // 同一物理停留所の複数ポール (parent_station 経由) は
        // `COALESCE(parent_station, stop_id)` で集約され、結果のキーは
        // 親停留所 ID になることを確認する。
        // P1, P2 が物理停留所。各 trip は別ポールを使うが、結果上は P1, P2
        // として 1 レコードずつになる。
        let mut conn = stop_route_mapping_fixtures::open_conn().await;
        let schema = stop_route_mapping_fixtures::setup_schema(&mut conn).await;
        stop_route_mapping_fixtures::insert_stop(&mut conn, "P1").await;
        stop_route_mapping_fixtures::insert_stop(&mut conn, "P2").await;
        stop_route_mapping_fixtures::insert_pole(&mut conn, "P1_a", "P1").await;
        stop_route_mapping_fixtures::insert_pole(&mut conn, "P1_b", "P1").await;
        stop_route_mapping_fixtures::insert_pole(&mut conn, "P2_a", "P2").await;
        stop_route_mapping_fixtures::insert_pole(&mut conn, "P2_b", "P2").await;
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_ab",
            "R5",
            Some(0),
            Some("shape_ab"),
            &[("P1_a", Some(0.0)), ("P2_a", Some(100.0))],
        )
        .await;
        stop_route_mapping_fixtures::insert_trip_with_stops(
            &mut conn,
            "trip_ba",
            "R5",
            Some(1),
            Some("shape_ba"),
            &[("P2_b", Some(0.0)), ("P1_b", Some(100.0))],
        )
        .await;

        let mapping = build_stop_route_mapping(&mut conn).await.unwrap();
        let actual = collect_route(&mapping, "R5");
        stop_route_mapping_fixtures::drop_schema(&mut conn, &schema).await;

        assert_eq!(
            actual,
            vec![("P1".to_string(), 1), ("P2".to_string(), 2)],
            "結果は親停留所 ID (P1, P2) で集約され、ポール ID は混入しないべき"
        );
    }
}
