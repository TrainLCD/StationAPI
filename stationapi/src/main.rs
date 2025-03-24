use csv::{ReaderBuilder, StringRecord};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    ConnectOptions, Sqlite,
};
use stationapi::{
    infrastructure::{
        company_repository::MyCompanyRepository, connection_repository::MyConnectionRepository,
        line_repository::MyLineRepository, station_repository::MyStationRepository,
        train_type_repository::MyTrainTypeRepository,
    },
    presentation::controller::grpc::MyApi,
    proto::{self, station_api_server::StationApiServer},
    use_case::interactor::query::QueryInteractor,
};
use std::{
    env::{self, VarError},
    fs,
    net::{AddrParseError, SocketAddr},
    str::FromStr,
};
use std::{path::Path, sync::Arc};
use tonic::codec::CompressionEncoding;
use tonic::transport::Server;
use tonic_health::server::HealthReporter;
use tracing::{info, warn};

async fn import_csv(db_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let data_path = Path::new("data");

    let mut conn = SqliteConnectOptions::from_str(db_url)?.connect().await?;

    let create_sql: String =
        String::from_utf8_lossy(&fs::read(data_path.join("create_table.sql"))?).parse()?;
    sqlx::query(&create_sql).execute(&mut conn).await.unwrap();

    let entries = fs::read_dir(data_path).expect("The `data` directory could not be found.");
    let mut file_list: Vec<_> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() && path.extension()? == "csv" {
                Some(path.file_name()?.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect();
    file_list.sort();

    for file_name in &file_list {
        if file_name
            .split('!')
            .nth(1)
            .unwrap_or_default()
            .split('.')
            .nth(1)
            .unwrap_or_default()
            != "csv"
        {
            continue;
        }

        let mut rdr = ReaderBuilder::new().from_path(data_path.join(file_name))?;
        let headers_record = rdr.headers()?;
        let headers: Vec<String> = headers_record
            .into_iter()
            .map(|row| row.to_string())
            .collect();

        let mut csv_data: Vec<StringRecord> = Vec::new();
        let mut rdr = ReaderBuilder::new().from_path(data_path.join(file_name))?;
        let records: Vec<StringRecord> = rdr.records().filter_map(|row| row.ok()).collect();
        csv_data.extend(records);

        let table_name = file_name
            .split('!')
            .nth(1)
            .unwrap_or_default()
            .split('.')
            .next()
            .unwrap_or_default();

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
                        Some(format!("'{}'", col.replace('\'', "''")))
                    }
                })
                .collect();

            sql_lines_inner.push(if idx == csv_data.len() - 1 {
                format!("({});", cols.join(","))
            } else {
                format!("({}),", cols.join(","))
            });
        }

        sqlx::query(&sql_lines_inner.concat())
            .execute(&mut conn)
            .await?;
    }

    Ok(())
}

async fn station_api_service_status(mut reporter: HealthReporter) {
    let db_url = fetch_database_url();
    let pool: sqlx::Pool<Sqlite> = SqlitePoolOptions::new().connect(&db_url).await.unwrap();
    // NOTE: 今までの障害でDBのデータが一部だけ消えたという現象はなかったので駅数だけ見ればいい
    let row = sqlx::query!("SELECT COUNT(`stations`.station_cd) <> 0 AS alive FROM `stations`")
        .fetch_one(&pool)
        .await
        .unwrap();

    if row.alive == 1 {
        reporter.set_serving::<StationApiServer<MyApi>>().await;
    } else {
        reporter.set_not_serving::<StationApiServer<MyApi>>().await;
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), anyhow::Error> {
    run().await
}

async fn run() -> std::result::Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    if dotenv::from_filename(".env.local").is_err() {
        warn!("Could not load .env.local");
    };

    let db_url = &fetch_database_url();
    let pool = Arc::new(SqlitePoolOptions::new().connect(db_url).await.unwrap());

    import_csv(db_url).await.expect("Failed to import CSV");

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<StationApiServer<MyApi>>()
        .await;

    tokio::spawn(station_api_service_status(health_reporter.clone()));

    let disable_grpc_web = fetch_disable_grpc_web_flag();
    let addr = fetch_addr()?;

    let station_repository = MyStationRepository::new(Arc::clone(&pool));
    let line_repository = MyLineRepository::new(Arc::clone(&pool));
    let train_type_repository = MyTrainTypeRepository::new(Arc::clone(&pool));
    let company_repository = MyCompanyRepository::new(Arc::clone(&pool));
    let connection_repository = MyConnectionRepository::new(Arc::clone(&pool));

    let query_use_case = QueryInteractor {
        station_repository,
        line_repository,
        train_type_repository,
        company_repository,
        connection_repository,
    };

    let my_api = MyApi { query_use_case };

    let svc = StationApiServer::new(my_api)
        .accept_compressed(CompressionEncoding::Zstd)
        .send_compressed(CompressionEncoding::Zstd);

    let reflection_svc = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

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
