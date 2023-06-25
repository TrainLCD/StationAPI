use std::{
    env::{self, VarError},
    net::{AddrParseError, SocketAddr},
};

use sqlx::MySqlPool;
use stationapi::{
    pb::station_api_server::StationApiServer, presentation::controller::grpc::GrpcRouter,
};
use tonic::transport::Server;
// use tower_http::cors::CorsLayer;
use tracing_log::LogTracer;

#[tokio::main]
async fn main() -> std::result::Result<(), anyhow::Error> {
    run().await
}

async fn run() -> std::result::Result<(), anyhow::Error> {
    LogTracer::init().expect("Failed to initialize LogTracer.");

    dotenv::from_filename(".env.local").ok();

    let addr = fetch_addr().unwrap();

    let db_url = fetch_database_url();
    let pool = MySqlPool::connect(db_url.as_str()).await?;
    let api_server = GrpcRouter::new(pool);
    let api_server = StationApiServer::new(api_server);

    // これを使うとアプリから接続できなくなる
    let allow_cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any);

    println!("StationAPI Server listening on {}", addr);

    Server::builder()
        .accept_http1(true)
        // これを使うとアプリから接続できなくなる
        .layer(allow_cors)
        .add_service(tonic_web::enable(api_server))
        .serve(addr)
        .await?;

    Ok(())
}

fn fetch_port() -> u16 {
    match env::var("PORT") {
        Ok(s) => s.parse().expect("Failed to parse $PORT"),
        Err(env::VarError::NotPresent) => {
            log::warn!("$PORT is not set. Falling back to 50051.");
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
            log::warn!("$HOST is not set. Falling back to {}.", fallback_host);
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
