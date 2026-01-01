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
                        Some(format!(
                            "'{}'",
                            col.replace('\'', "''").replace('\\', "\\\\")
                        ))
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

    let db_url = fetch_database_url();
    let mut conn = PgConnection::connect(&db_url).await?;

    info!("Starting GTFS import from {:?}...", gtfs_path);

    // First, clear existing GTFS data (in reverse order of dependencies)
    sqlx::query("DELETE FROM gtfs_stop_times")
        .execute(&mut conn)
        .await?;
    sqlx::query("DELETE FROM gtfs_trips")
        .execute(&mut conn)
        .await?;
    sqlx::query("DELETE FROM gtfs_shapes")
        .execute(&mut conn)
        .await?;
    sqlx::query("DELETE FROM gtfs_calendar_dates")
        .execute(&mut conn)
        .await?;
    sqlx::query("DELETE FROM gtfs_calendar")
        .execute(&mut conn)
        .await?;
    sqlx::query("DELETE FROM gtfs_stops")
        .execute(&mut conn)
        .await?;
    sqlx::query("DELETE FROM gtfs_routes")
        .execute(&mut conn)
        .await?;
    sqlx::query("DELETE FROM gtfs_agencies")
        .execute(&mut conn)
        .await?;
    sqlx::query("DELETE FROM gtfs_feed_info")
        .execute(&mut conn)
        .await?;

    // Load translations for multi-language support
    let translations = load_gtfs_translations(gtfs_path)?;

    // Import agencies
    import_gtfs_agencies(&mut conn, gtfs_path).await?;

    // Import routes
    import_gtfs_routes(&mut conn, gtfs_path).await?;

    // Import stops with translations
    import_gtfs_stops(&mut conn, gtfs_path, &translations).await?;

    // Import calendar
    import_gtfs_calendar(&mut conn, gtfs_path).await?;

    // Import calendar_dates
    import_gtfs_calendar_dates(&mut conn, gtfs_path).await?;

    // Import shapes
    import_gtfs_shapes(&mut conn, gtfs_path).await?;

    // Import trips
    import_gtfs_trips(&mut conn, gtfs_path).await?;

    // Import stop_times (largest file, needs batch processing)
    import_gtfs_stop_times(&mut conn, gtfs_path).await?;

    // Import feed_info
    import_gtfs_feed_info(&mut conn, gtfs_path).await?;

    sqlx::query("ANALYZE;").execute(&mut conn).await?;

    info!("GTFS import completed successfully.");

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
    let mut count = 0;

    for result in rdr.records() {
        let record = result?;
        // stop_id,stop_code,stop_name,stop_desc,stop_lat,stop_lon,zone_id,stop_url,location_type,parent_station,stop_timezone,wheelchair_boarding,platform_code,stop_access
        let stop_id = record.get(0).unwrap_or("");
        let stop_code = record.get(1).filter(|s| !s.is_empty());
        let stop_name = record.get(2).unwrap_or("");
        let stop_desc = record.get(3).filter(|s| !s.is_empty());
        let stop_lat: f64 = record.get(4).unwrap_or("0").parse().unwrap_or(0.0);
        let stop_lon: f64 = record.get(5).unwrap_or("0").parse().unwrap_or(0.0);
        let zone_id = record.get(6).filter(|s| !s.is_empty());
        let stop_url = record.get(7).filter(|s| !s.is_empty());
        let location_type: i32 = record.get(8).unwrap_or("0").parse().unwrap_or(0);
        let parent_station = record.get(9).filter(|s| !s.is_empty());
        let stop_timezone = record.get(10).filter(|s| !s.is_empty());
        let wheelchair_boarding: Option<i32> = record
            .get(11)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok());
        let platform_code = record.get(12).filter(|s| !s.is_empty());

        // Get translations
        let key = ("stops".to_string(), stop_id.to_string());
        let translation = translations.get(&key);

        let stop_name_k = translation.and_then(|t| t.ja_hrkt.clone());
        let stop_name_r = translation.and_then(|t| t.en.clone());
        let stop_name_zh = translation.and_then(|t| t.zh.clone());
        let stop_name_ko = translation.and_then(|t| t.ko.clone());

        sqlx::query(
            r#"INSERT INTO gtfs_stops
               (stop_id, stop_code, stop_name, stop_name_k, stop_name_r, stop_name_zh, stop_name_ko,
                stop_desc, stop_lat, stop_lon, zone_id, stop_url, location_type, parent_station,
                stop_timezone, wheelchair_boarding, platform_code)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
               ON CONFLICT (stop_id) DO NOTHING"#,
        )
        .bind(stop_id)
        .bind(stop_code)
        .bind(stop_name)
        .bind(stop_name_k)
        .bind(stop_name_r)
        .bind(stop_name_zh)
        .bind(stop_name_ko)
        .bind(stop_desc)
        .bind(stop_lat)
        .bind(stop_lon)
        .bind(zone_id)
        .bind(stop_url)
        .bind(location_type)
        .bind(parent_station)
        .bind(stop_timezone)
        .bind(wheelchair_boarding)
        .bind(platform_code)
        .execute(&mut *conn)
        .await?;

        count += 1;
    }

    info!("Imported {} stops.", count);
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
    let mut count = 0;

    for result in rdr.records() {
        let record = result?;
        // service_id,date,exception_type
        let service_id = record.get(0).unwrap_or("");
        let date = record.get(1).unwrap_or("");
        let exception_type: i32 = record.get(2).unwrap_or("1").parse().unwrap_or(1);

        let date = chrono::NaiveDate::parse_from_str(date, "%Y%m%d")?;

        sqlx::query(
            r#"INSERT INTO gtfs_calendar_dates (service_id, date, exception_type)
               VALUES ($1, $2, $3)"#,
        )
        .bind(service_id)
        .bind(date)
        .bind(exception_type)
        .execute(&mut *conn)
        .await?;

        count += 1;
    }

    info!("Imported {} calendar_dates.", count);
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
    let batch_size = 5000;

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
            shape_id.replace('\'', "''"),
            lat,
            lon,
            seq,
            dist_str
        ));

        if (i + 1) % 1000 == 0 || i == batch.len() - 1 {
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
    let batch_size = 5000;

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

    let mut sql = String::from(
        "INSERT INTO gtfs_trips (trip_id, route_id, service_id, trip_headsign, trip_short_name, direction_id, block_id, shape_id, wheelchair_accessible, bikes_allowed) VALUES ",
    );
    let mut values: Vec<String> = Vec::with_capacity(batch.len());

    for (
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
    ) in batch
    {
        let headsign_str = trip_headsign
            .as_ref()
            .map(|s| format!("'{}'", s.replace('\'', "''")))
            .unwrap_or_else(|| "NULL".to_string());
        let short_name_str = trip_short_name
            .as_ref()
            .map(|s| format!("'{}'", s.replace('\'', "''")))
            .unwrap_or_else(|| "NULL".to_string());
        let direction_str = direction_id
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".to_string());
        let block_str = block_id
            .as_ref()
            .map(|s| format!("'{}'", s.replace('\'', "''")))
            .unwrap_or_else(|| "NULL".to_string());
        let shape_str = shape_id
            .as_ref()
            .map(|s| format!("'{}'", s.replace('\'', "''")))
            .unwrap_or_else(|| "NULL".to_string());
        let wheelchair_str = wheelchair_accessible
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".to_string());
        let bikes_str = bikes_allowed
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".to_string());

        values.push(format!(
            "('{}', '{}', '{}', {}, {}, {}, {}, {}, {}, {})",
            trip_id.replace('\'', "''"),
            route_id.replace('\'', "''"),
            service_id.replace('\'', "''"),
            headsign_str,
            short_name_str,
            direction_str,
            block_str,
            shape_str,
            wheelchair_str,
            bikes_str
        ));
    }

    sql.push_str(&values.join(","));
    sql.push_str(" ON CONFLICT (trip_id) DO NOTHING");

    sqlx::query(&sql).execute(&mut *conn).await?;

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
    let batch_size = 5000;

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
    let mut sql = String::from(
        "INSERT INTO gtfs_stop_times (trip_id, arrival_time, departure_time, stop_id, stop_sequence, stop_headsign, pickup_type, drop_off_type, shape_dist_traveled, timepoint) VALUES ",
    );
    let mut values: Vec<String> = Vec::with_capacity(batch.len());

    for (
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
    ) in batch
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
            .map(|s| format!("'{}'", s.replace('\'', "''")))
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
            trip_id.replace('\'', "''"),
            arrival_str,
            departure_str,
            stop_id.replace('\'', "''"),
            stop_sequence,
            headsign_str,
            pickup_str,
            dropoff_str,
            dist_str,
            timepoint_str
        ));
    }

    sql.push_str(&values.join(","));
    sql.push_str(" ON CONFLICT DO NOTHING");

    sqlx::query(&sql).execute(&mut *conn).await?;

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

    // Step 1: Clear existing bus data from stations/lines
    sqlx::query("DELETE FROM stations WHERE transport_type = 1")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM lines WHERE transport_type = 1")
        .execute(&mut *tx)
        .await?;
    info!("Cleared existing bus data from stations/lines tables.");

    // Step 2: Insert bus routes as lines
    integrate_gtfs_routes_to_lines(&mut tx).await?;

    // Step 3: Build stop-route mapping from stop_times
    let stop_route_map = build_stop_route_mapping(&mut tx).await?;

    // Step 4: Insert bus stops as stations
    integrate_gtfs_stops_to_stations(&mut tx, &stop_route_map).await?;

    // Step 5: Update cross-references in GTFS tables
    update_gtfs_crossreferences(&mut tx, &stop_route_map).await?;

    sqlx::query("ANALYZE;").execute(&mut *tx).await?;

    // Commit the transaction - all changes are now permanent
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

/// Build mapping of (stop_id, route_id) -> stop_sequence from gtfs_stop_times
async fn build_stop_route_mapping(
    conn: &mut PgConnection,
) -> Result<HashMap<String, Vec<(String, i32)>>, Box<dyn std::error::Error>> {
    // For each route, find the representative trip (the one with the most stops)
    // This ensures consistent stop ordering within a route
    // Use direction_id = 0 (outbound) preferentially
    let rows: Vec<(String, String, i32)> = sqlx::query_as(
        r#"WITH representative_trips AS (
               -- Find the trip with the most stops for each route (prefer direction_id = 0)
               SELECT DISTINCT ON (gt.route_id)
                   gt.route_id,
                   gt.trip_id,
                   COUNT(*) as stop_count
               FROM gtfs_trips gt
               JOIN gtfs_stop_times gst ON gt.trip_id = gst.trip_id
               GROUP BY gt.route_id, gt.trip_id, gt.direction_id
               ORDER BY gt.route_id,
                        CASE WHEN gt.direction_id = 0 THEN 0 ELSE 1 END,
                        COUNT(*) DESC
           )
           SELECT gst.stop_id, rt.route_id, gst.stop_sequence
           FROM representative_trips rt
           JOIN gtfs_stop_times gst ON rt.trip_id = gst.trip_id
           ORDER BY rt.route_id, gst.stop_sequence"#,
    )
    .fetch_all(&mut *conn)
    .await?;

    let mut map: HashMap<String, Vec<(String, i32)>> = HashMap::new();
    for (stop_id, route_id, stop_sequence) in rows {
        map.entry(stop_id)
            .or_default()
            .push((route_id, stop_sequence));
    }

    info!("Built stop-route mapping for {} stops.", map.len());
    Ok(map)
}

/// Integrate gtfs_stops into stations table (one record per route served)
async fn integrate_gtfs_stops_to_stations(
    conn: &mut PgConnection,
    stop_route_map: &HashMap<String, Vec<(String, i32)>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let stops: Vec<GtfsStopRow> = sqlx::query_as("SELECT * FROM gtfs_stops")
        .fetch_all(&mut *conn)
        .await?;

    let mut inserted_count = 0;

    for stop in &stops {
        // Use parent_station for station_g_cd if available, otherwise use stop_id
        let parent_stop_id = stop.parent_station.as_ref().unwrap_or(&stop.stop_id);
        let station_g_cd = generate_bus_station_g_cd(parent_stop_id);

        // Get routes for this stop (with stop_sequence)
        let routes = match stop_route_map.get(&stop.stop_id) {
            Some(r) => r.clone(),
            None => continue, // Skip stops not on any route
        };

        // Create a station record for each route this stop serves
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
            .bind(stop.stop_name_k.as_ref().unwrap_or(&stop.stop_name))
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
    for (stop_id, routes) in stop_route_map {
        if let Some((route_id, _)) = routes.first() {
            let station_cd = generate_bus_station_cd(stop_id, route_id);
            sqlx::query("UPDATE gtfs_stops SET station_cd = $1 WHERE stop_id = $2")
                .bind(station_cd)
                .bind(stop_id)
                .execute(&mut *conn)
                .await?;
        }
    }

    info!("Updated GTFS cross-references.");
    Ok(())
}
