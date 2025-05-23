use csv::{ReaderBuilder, StringRecord};
use sqlx::{Connection, SqliteConnection};
use stationapi::{
    infrastructure::{
        company_repository::MyCompanyRepository, line_repository::MyLineRepository,
        station_repository::MyStationRepository, train_type_repository::MyTrainTypeRepository,
    },
    presentation::controller::grpc::MyApi,
    proto::{self, station_api_server::StationApiServer},
    use_case::interactor::query::QueryInteractor,
};
use std::{
    env::{self, VarError},
    fs,
    net::{AddrParseError, SocketAddr},
};
use std::{path::Path, sync::Arc};
use tokio::sync::Mutex;
use tonic::transport::Server;
use tonic_health::server::HealthReporter;
use tracing::{info, warn};

async fn import_csv(conn: Arc<Mutex<SqliteConnection>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = conn.lock().await;
    let data_path = Path::new("data");

    let create_sql_path = data_path.join("create_table.sql");
    let create_sql_content = fs::read(&create_sql_path).map_err(|e| {
        tracing::error!("Failed to read create_table.sql: {}", e);
        Box::new(e) as Box<dyn std::error::Error>
    })?;
    let create_sql: String = String::from_utf8_lossy(&create_sql_content).parse()?;
    sqlx::query(&create_sql)
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create tables: {}", e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;
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
        sql_lines_inner.push(format!("INSERT INTO `{}` VALUES ", table_name));

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
            sql_lines_inner.push(format!("({}{}", values_part, separator));
        }

        sqlx::query(&sql_lines_inner.concat())
            .execute(&mut *conn)
            .await?;
    }

    // NOTE: SQLiteパフォーマンスチューニング
    sqlx::query(
        r#"PRAGMA journal_mode = MEMORY;
    PRAGMA synchronous = OFF;
    PRAGMA temp_store = MEMORY;
    PRAGMA locking_mode = EXCLUSIVE;
    PRAGMA cache_size = -262144;
    PRAGMA query_only = ON;"#,
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

async fn station_api_service_status(mut reporter: HealthReporter) {
    let db_url = fetch_database_url();
    let mut conn = match SqliteConnection::connect(&db_url).await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Failed to connect to database: {}", e);
            panic!("Failed to connect to database: {}", e); // または適切な回復戦略
        }
    };
    // NOTE: 今までの障害でDBのデータが一部だけ消えたという現象はなかったので駅数だけ見ればいい
    let row = sqlx::query!("SELECT COUNT(`stations`.station_cd) <> 0 AS alive FROM `stations`")
        .fetch_one(&mut conn)
        .await
        .expect("Failed to fetch station count");

    if row.alive == 1 {
        reporter.set_serving::<StationApiServer<MyApi>>().await;
    } else {
        reporter.set_not_serving::<StationApiServer<MyApi>>().await;
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::result::Result<(), anyhow::Error> {
    run().await
}

async fn run() -> std::result::Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    if dotenv::from_filename(".env.local").is_err() {
        warn!("Could not load .env.local");
    };

    let db_url = &fetch_database_url();
    let conn = Arc::new(Mutex::new(match SqliteConnection::connect(db_url).await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Failed to connect to database: {}", e);
            panic!("Failed to connect to database: {}", e); // または適切な回復戦略
        }
    }));

    if let Err(e) = import_csv(Arc::clone(&conn)).await {
        tracing::error!("Failed to import CSV: {}", e);
        return Err(anyhow::anyhow!("Failed to import CSV: {}", e));
    }

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<StationApiServer<MyApi>>()
        .await;

    tokio::spawn(station_api_service_status(health_reporter.clone()));

    let disable_grpc_web = fetch_disable_grpc_web_flag();
    let addr = fetch_addr()?;

    let station_repository = MyStationRepository::new(Arc::clone(&conn));
    let line_repository = MyLineRepository::new(Arc::clone(&conn));
    let train_type_repository = MyTrainTypeRepository::new(Arc::clone(&conn));
    let company_repository = MyCompanyRepository::new(Arc::clone(&conn));

    let query_use_case = QueryInteractor {
        station_repository,
        line_repository,
        train_type_repository,
        company_repository,
    };

    let my_api = MyApi { query_use_case };

    let svc = StationApiServer::new(my_api);

    let reflection_svc = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .expect("Failed to build reflection service");

    info!("StationAPI Server listening on {}", addr);

    if disable_grpc_web {
        Server::builder()
            .add_service(health_service)
            .add_service(svc)
            .add_service(reflection_svc)
            .serve(addr)
            .await?;
    } else {
        Server::builder()
            .accept_http1(true)
            .add_service(tonic_web::enable(health_service))
            .add_service(tonic_web::enable(svc))
            .add_service(tonic_web::enable(reflection_svc))
            .serve(addr)
            .await?;
    }

    Ok(())
}

fn fetch_port() -> u16 {
    match env::var("PORT") {
        Ok(s) => s.parse().expect("Failed to parse $PORT"),
        Err(env::VarError::NotPresent) => {
            warn!("$PORT is not set. Falling back to 50051.");
            50051
        }
        Err(VarError::NotUnicode(_)) => panic!("$PORT should be written in Unicode."),
    }
}

fn fetch_addr() -> Result<SocketAddr, AddrParseError> {
    let port = fetch_port();
    match env::var("HOST") {
        Ok(s) => format!("{}:{}", s, port).parse(),
        Err(env::VarError::NotPresent) => {
            let fallback_host = format!("[::1]:{}", port);
            warn!("$HOST is not set. Falling back to {}.", fallback_host);
            fallback_host.parse()
        }
        Err(VarError::NotUnicode(_)) => panic!("$HOST should be written in Unicode."),
    }
}

fn fetch_database_url() -> String {
    match env::var("DATABASE_URL") {
        Ok(s) => s,
        Err(env::VarError::NotPresent) => panic!("$DATABASE_URL is not set."),
        Err(VarError::NotUnicode(_)) => panic!("$DATABASE_URL should be written in Unicode."),
    }
}

fn fetch_disable_grpc_web_flag() -> bool {
    match env::var("DISABLE_GRPC_WEB") {
        Ok(s) => s.parse().expect("Failed to parse $DISABLE_GRPC_WEB"),
        Err(env::VarError::NotPresent) => {
            warn!("$DISABLE_GRPC_WEB is not set. Falling back to false.");
            false
        }
        Err(VarError::NotUnicode(_)) => panic!("$DISABLE_GRPC_WEB should be written in Unicode."),
    }
}
