mod import;

use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, PgConnection};
use stationapi::config::fetch_database_url;
use stationapi::infrastructure::company_repository::MyCompanyRepository;
use stationapi::infrastructure::line_repository::MyLineRepository;
use stationapi::infrastructure::station_repository::MyStationRepository;
use stationapi::infrastructure::train_type_repository::MyTrainTypeRepository;
use stationapi::presentation::controller::grpc::MyApi;
use stationapi::proto;
use stationapi::proto::station_api_server::StationApiServer;
use stationapi::use_case::interactor::query::QueryInteractor;
use tonic::transport::Server;
use tonic_health::server::HealthReporter;

use std::sync::Arc;
use std::{
    env::{self, VarError},
    net::{AddrParseError, SocketAddr},
};
use tracing::{info, warn};

#[derive(sqlx::FromRow)]
struct AliveRow {
    pub alive: Option<bool>,
}

async fn station_api_service_status(mut reporter: HealthReporter) {
    let db_url = fetch_database_url();
    let mut conn = match PgConnection::connect(&db_url).await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Failed to connect to database: {}", e);
            panic!("Failed to connect to database: {e}");
        }
    };
    // NOTE: 今までの障害でDBのデータが一部だけ消えたという現象はなかったので駅数だけ見ればいい
    let row = sqlx::query_as!(
        AliveRow,
        "SELECT COUNT(stations.station_cd) <> 0 AS alive FROM stations"
    )
    .fetch_one(&mut conn)
    .await
    .expect("Failed to fetch station count");

    if row.alive.unwrap_or(false) {
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

    if let Err(e) = import::import_csv().await {
        return Err(anyhow::anyhow!("Failed to import CSV: {}", e));
    }

    // Import GTFS data (ToeiBus)
    if let Err(e) = import::import_gtfs().await {
        warn!(
            "Failed to import GTFS data: {}. Continuing without GTFS data.",
            e
        );
    }

    // Integrate GTFS data into stations/lines tables
    // This is wrapped in a transaction - if any step fails, all changes are rolled back
    if let Err(e) = import::integrate_gtfs_to_stations().await {
        return Err(anyhow::anyhow!(
            "Failed to integrate GTFS to stations (transaction rolled back): {}",
            e
        ));
    }

    let db_url = &fetch_database_url();
    let pool = Arc::new(
        match PgPoolOptions::new()
            .max_connections(5)
            .connect(db_url)
            .await
        {
            Ok(conn) => conn,
            Err(e) => {
                tracing::error!("Failed to connect to database: {}", e);
                panic!("Failed to connect to database: {e}");
            }
        },
    );

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
            .add_service(health_service)
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
        Ok(s) => format!("{s}:{port}").parse(),
        Err(env::VarError::NotPresent) => {
            let fallback_host = format!("[::1]:{port}");
            warn!("$HOST is not set. Falling back to {}.", fallback_host);
            fallback_host.parse()
        }
        Err(VarError::NotUnicode(_)) => panic!("$HOST should be written in Unicode."),
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
