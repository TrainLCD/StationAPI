use sqlx::MySqlPool;
use stationapi::{
    infrastructure::{
        company_repository::MyCompanyRepository, line_repository::MyLineRepository,
        station_repository::MyStationRepository, train_type_repository::MyTrainTypeRepository,
    },
    presentation::controller::grpc::MyApi,
    station_api::station_api_server::StationApiServer,
    use_case::interactor::query::QueryInteractor,
};
use std::sync::Arc;
use std::{
    env::{self, VarError},
    net::{AddrParseError, SocketAddr},
};
use tonic::codec::CompressionEncoding;
use tonic::transport::Server;
use tonic_health::server::HealthReporter;
use tracing::{info, warn};

async fn station_api_service_status(mut reporter: HealthReporter) {
    let db_url = fetch_database_url();
    let pool: sqlx::Pool<sqlx::MySql> = MySqlPool::connect(db_url.as_str()).await.unwrap();
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

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<StationApiServer<MyApi>>()
        .await;

    tokio::spawn(station_api_service_status(health_reporter.clone()));

    let disable_grpc_web = fetch_disable_grpc_web_flag();
    let addr = fetch_addr()?;

    let db_url = fetch_database_url();
    let pool = Arc::new(MySqlPool::connect(db_url.as_str()).await?);

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

    let svc = StationApiServer::new(my_api)
        .accept_compressed(CompressionEncoding::Zstd)
        .send_compressed(CompressionEncoding::Zstd);

    info!("StationAPI Server listening on {}", addr);

    if disable_grpc_web {
        Server::builder()
            .add_service(health_service)
            .add_service(svc)
            .serve(addr)
            .await?;
    } else {
        Server::builder()
            .accept_http1(true)
            .add_service(tonic_web::enable(health_service))
            .add_service(tonic_web::enable(svc))
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
