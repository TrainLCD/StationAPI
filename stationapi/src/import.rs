//! Data import module for CSV and GTFS data

use csv::{ReaderBuilder, StringRecord};
use sqlx::{Connection, PgConnection};
use stationapi::config::fetch_database_url;
use std::collections::HashMap;
use std::io::{Cursor, Read as _};
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

/// Import CSV data from the data directory
pub async fn import_csv() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = fetch_database_url();
    let mut conn = PgConnection::connect(&db_url).await?;
    let data_path = Path::new("data");

    // Ensure required extensions exist before running schema import
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

/// GTFS download URL for Toei Bus
const TOEI_BUS_GTFS_URL: &str =
    "https://api-public.odpt.org/api/v4/files/Toei/data/ToeiBus-GTFS.zip";

/// Download and extract GTFS data from ODPT API
fn download_gtfs() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let gtfs_path = Path::new("data/ToeiBus-GTFS");

    // Skip if directory already exists
    if gtfs_path.exists() {
        info!("GTFS directory already exists, skipping download.");
        return Ok(());
    }

    info!("Downloading GTFS data from ODPT API...");

    // Download the ZIP file
    let response = reqwest::blocking::get(TOEI_BUS_GTFS_URL)?;

    if !response.status().is_success() {
        return Err(format!("Failed to download GTFS: HTTP {}", response.status()).into());
    }

    let bytes = response.bytes()?;
    info!("Downloaded {} bytes, extracting...", bytes.len());

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

        info!("Extracted: {}", output_name);
    }

    info!("GTFS extraction completed.");
    Ok(())
}

/// Import GTFS data from ToeiBus-GTFS directory
/// All imports are wrapped in a transaction - if any step fails, all changes are rolled back
pub async fn import_gtfs() -> Result<(), Box<dyn std::error::Error>> {
    // Check if bus feature is disabled
    if is_bus_feature_disabled() {
        info!("Bus feature is disabled, skipping GTFS import.");
        return Ok(());
    }

    // Download GTFS data if not present (use spawn_blocking to avoid blocking async runtime)
    tokio::task::spawn_blocking(download_gtfs)
        .await
        .map_err(|e| format!("Failed to spawn blocking task: {}", e))?
        .map_err(|e| -> Box<dyn std::error::Error> { e })?;

    let gtfs_path = Path::new("data/ToeiBus-GTFS");

    if !gtfs_path.exists() {
        info!("GTFS directory not found, skipping GTFS import.");
        return Ok(());
    }

    // Load translations for multi-language support (before transaction to avoid holding lock)
    let translations = load_gtfs_translations(gtfs_path)?;

    let db_url = fetch_database_url();
    let mut conn = PgConnection::connect(&db_url).await?;

    info!(
        "Starting GTFS import from {:?} (using transaction)...",
        gtfs_path
    );

    // Begin transaction - all changes will be rolled back if any step fails
    let mut tx = conn.begin().await?;

    // First, clear existing GTFS data (in reverse order of dependencies)
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

    // Import agencies
    import_gtfs_agencies(&mut tx, gtfs_path).await?;

    // Import routes
    import_gtfs_routes(&mut tx, gtfs_path).await?;

    // Import stops with translations
    import_gtfs_stops(&mut tx, gtfs_path, &translations).await?;

    // Import calendar
    import_gtfs_calendar(&mut tx, gtfs_path).await?;

    // Import calendar_dates
    import_gtfs_calendar_dates(&mut tx, gtfs_path).await?;

    // Import shapes
    import_gtfs_shapes(&mut tx, gtfs_path).await?;

    // Import trips
    import_gtfs_trips(&mut tx, gtfs_path).await?;

    // Import stop_times (largest file, needs batch processing)
    import_gtfs_stop_times(&mut tx, gtfs_path).await?;

    // Import feed_info
    import_gtfs_feed_info(&mut tx, gtfs_path).await?;

    // Commit transaction - all changes are now permanent
    tx.commit().await?;

    info!("GTFS import completed successfully (transaction committed).");

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

    for result in rdr.records() {
        let record = result?;
        // table_name,field_name,language,translation,record_id,record_sub_id,field_value
        let table_name = record.get(0).unwrap_or("");
        let field_name = record.get(1).unwrap_or("");
        let language = record.get(2).unwrap_or("");
        let translation_text = record.get(3).unwrap_or("");
        let record_id = record.get(4).unwrap_or("");

        // Only process stop_name translations for now
        if table_name == "stops" && field_name == "stop_name" {
            // Store translation for the exact record_id (e.g., "0001-01")
            let key = ("stops".to_string(), record_id.to_string());
            let entry = translations.entry(key).or_default();

            match language {
                "ja" => entry.ja = Some(translation_text.to_string()),
                "ja-Hrkt" => entry.ja_hrkt = Some(translation_text.to_string()),
                "en" => entry.en = Some(translation_text.to_string()),
                "zh-Hans" | "zh-Hant" | "zh" => entry.zh = Some(translation_text.to_string()),
                "ko" => entry.ko = Some(translation_text.to_string()),
                _ => {}
            }

            // Also store translation for parent stop_id (without suffix like "-01", "-02")
            // This allows parent stops (location_type=1) to find translations
            if let Some(parent_id) = record_id.rfind('-').map(|pos| &record_id[..pos]) {
                let parent_key = ("stops".to_string(), parent_id.to_string());
                // Only insert if not already present (first child's translation wins)
                translations
                    .entry(parent_key)
                    .or_insert_with(|| Translation {
                        ja: None,
                        ja_hrkt: None,
                        en: None,
                        zh: None,
                        ko: None,
                    });
                let parent_entry = translations
                    .get_mut(&("stops".to_string(), parent_id.to_string()))
                    .unwrap();
                match language {
                    "ja" if parent_entry.ja.is_none() => {
                        parent_entry.ja = Some(translation_text.to_string())
                    }
                    "ja-Hrkt" if parent_entry.ja_hrkt.is_none() => {
                        parent_entry.ja_hrkt = Some(translation_text.to_string())
                    }
                    "en" if parent_entry.en.is_none() => {
                        parent_entry.en = Some(translation_text.to_string())
                    }
                    "zh-Hans" | "zh-Hant" | "zh" if parent_entry.zh.is_none() => {
                        parent_entry.zh = Some(translation_text.to_string())
                    }
                    "ko" if parent_entry.ko.is_none() => {
                        parent_entry.ko = Some(translation_text.to_string())
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(translations)
}

/// Import agencies from agency.txt
async fn import_gtfs_agencies(
    conn: &mut PgConnection,
    gtfs_path: &Path,
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
        let agency_id = record.get(0).unwrap_or("");
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
        .bind(agency_id)
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
        let route_id = record.get(0).unwrap_or("");
        let agency_id = record.get(1).filter(|s| !s.is_empty());
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
        .bind(route_id)
        .bind(agency_id)
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
        let stop_id = record.get(0).unwrap_or("").to_string();
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
            .map(|s| s.to_string());
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

        // Get translations
        let key = ("stops".to_string(), stop_id.clone());
        let translation = translations.get(&key);

        let stop_name_k = translation.and_then(|t| t.ja_hrkt.clone());
        let stop_name_r = translation.and_then(|t| t.en.clone());
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
        let service_id = record.get(0).unwrap_or("");
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
        .bind(service_id)
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
        let service_id = record.get(0).unwrap_or("").to_string();
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
        let shape_id = record.get(0).unwrap_or("").to_string();
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
        let route_id = record.get(0).unwrap_or("").to_string();
        let service_id = record.get(1).unwrap_or("").to_string();
        let trip_id = record.get(2).unwrap_or("").to_string();
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
            .map(|s| s.to_string());
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
        let trip_id = record.get(0).unwrap_or("").to_string();
        let arrival_time = parse_gtfs_time(record.get(1).unwrap_or(""));
        let departure_time = parse_gtfs_time(record.get(2).unwrap_or(""));
        let stop_id = record.get(3).unwrap_or("").to_string();
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
        let feed_publisher_name = record.get(0).unwrap_or("");
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
    match env::var("DISABLE_BUS_FEATURE") {
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
    if is_bus_feature_disabled() {
        info!("Bus feature is disabled, skipping GTFS integration.");
        return Ok(());
    }

    let db_url = fetch_database_url();
    let mut conn = PgConnection::connect(&db_url).await?;

    // Check if GTFS data exists (outside transaction for quick exit)
    let gtfs_route_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM gtfs_routes")
        .fetch_one(&mut conn)
        .await?;

    if gtfs_route_count.0 == 0 {
        info!("No GTFS routes found, skipping integration.");
        return Ok(());
    }

    info!("Starting GTFS to stations/lines integration (using transaction)...");

    // Begin transaction - all changes will be rolled back if any step fails
    let mut tx = conn.begin().await?;

    // Step 1: Clear existing bus data from stations/lines/types/sst.
    // station_station_types references both types (FK) and stations (FK), so delete
    // bus sst rows before bus types and before stations.
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
    info!("Cleared existing bus data from stations/lines/types/station_station_types tables.");

    // Step 2: Insert bus routes as lines
    integrate_gtfs_routes_to_lines(&mut tx).await?;

    // Step 3: Build stop-route mapping from stop_times
    let stop_route_map = build_stop_route_mapping(&mut tx).await?;

    // Step 4: Insert bus stops as stations
    integrate_gtfs_stops_to_stations(&mut tx, &stop_route_map).await?;

    // Step 5: Update cross-references in GTFS tables
    update_gtfs_crossreferences(&mut tx, &stop_route_map).await?;

    // Step 6: Register each (route_id, shape_id) trip variation as a TrainType
    // (kind=BusRoute) so clients can switch between bus operation patterns
    // (e.g. 池86 のフルループ / サンシャインシティ経由 / 短ターン).
    integrate_gtfs_trip_variations_to_types(&mut tx).await?;

    // Commit the transaction - all changes are now permanent
    // ANALYZE is run separately in main.rs after all GTFS imports complete
    tx.commit().await?;

    info!("GTFS integration completed successfully (transaction committed).");
    Ok(())
}

/// Integrate gtfs_routes into lines table
async fn integrate_gtfs_routes_to_lines(
    conn: &mut PgConnection,
) -> Result<(), Box<dyn std::error::Error>> {
    let routes: Vec<GtfsRouteRow> = sqlx::query_as("SELECT * FROM gtfs_routes")
        .fetch_all(&mut *conn)
        .await?;

    let company_cd = 119; // Tokyo Metropolitan Bureau of Transportation (東京都交通局)

    for route in &routes {
        let line_cd = generate_bus_line_cd(&route.route_id);
        let line_name = route
            .route_short_name
            .clone()
            .unwrap_or_else(|| route.route_long_name.clone().unwrap_or_default());
        let line_color = route.route_color.as_ref().map(|c| {
            if c.starts_with('#') {
                c.clone()
            } else {
                format!("#{}", c)
            }
        });

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
               WHERE pc.depth < 10
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
               WHERE nc.depth < 10
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

    let mut inserted_count = 0;

    for stop in &stops {
        let station_g_cd = generate_bus_station_g_cd(&stop.stop_id);

        // Get routes for this parent stop (with stop_sequence)
        // The mapping now uses parent_stop_id as key
        let routes = match stop_route_map.get(&stop.stop_id) {
            Some(r) => r.clone(),
            None => continue, // Skip stops not on any route
        };

        // Create a station record for each route this physical stop serves
        for (route_id, stop_sequence) in &routes {
            let station_cd = generate_bus_station_cd(&stop.stop_id, route_id);
            let line_cd = generate_bus_line_cd(route_id);

            sqlx::query(
                r#"INSERT INTO stations (
                    station_cd, station_g_cd, station_name, station_name_k,
                    station_name_r, station_name_zh, station_name_ko,
                    line_cd, pref_cd, post, address, lon, lat,
                    open_ymd, close_ymd, e_status, e_sort, transport_type
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, 13, '', '', $9, $10,
                    '', '', 0, $11, 1
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
        // - 双方向ペア (同じ停留所集合, direction 0/1) → "<A> ⇔ <B>" (両端駅) ※「行き」は付けない
        // - 循環トリップ (始発 parent == 終点 parent) → "<headsign>行き（<経由地>経由・循環）"
        //   (経由地が取れなければ "<headsign>行き (循環)" にフォールバック)
        // - それ以外で始発名と headsign が異なる → "<first_stop> → <headsign>行き"
        // - 始発名と headsign が同じ / どちらか欠落 → headsign があれば "<headsign>行き"、
        //   無ければ "<first_stop>" (始発名は行き先ではないので「行き」は付けない)
        // - すべて欠落 → "shape <shape_id>" でフォールバック
        let loop_name = |label: &str| -> String {
            match via_for_rep.get(&rep_idx) {
                Some(via) => format!("{}行き（{}経由・循環）", label, via),
                None => format!("{}行き (循環)", label),
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
                    format!("{} → {}行き", first, h)
                }
                (_, _, Some(h)) => format!("{}行き", h),
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
}
