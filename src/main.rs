use sqlx::MySqlPool;
use stationapi::{
    pb::station_api_server::StationApiServer, presentation::controller::grpc::GrpcRouter,
};
use std::{
    env::{self, VarError},
    net::{AddrParseError, SocketAddr},
};
use tonic::transport::Server;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> std::result::Result<(), anyhow::Error> {
    run().await
}

async fn run() -> std::result::Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    dotenv::from_filename(".env.local").ok();

    let db_url = fetch_database_url();
    let pool = MySqlPool::connect(db_url.as_str()).await?;

    let cache_client = connect_to_memcached()?;

    let api_server = GrpcRouter::new(pool, cache_client);
    let accept_http1 = fetch_http1_flag();

    let addr = fetch_addr()?;

    info!("StationAPI Server listening on {}", addr);

    Server::builder()
        .accept_http1(accept_http1)
        .add_service(tonic_web::enable(StationApiServer::new(api_server)))
        .serve(addr)
        .await?;

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
fn fetch_http1_flag() -> bool {
    match env::var("ACCEPT_HTTP1") {
        Ok(s) => s.parse().expect("Failed to parse $ACCEPT_HTTP1"),
        Err(env::VarError::NotPresent) => {
            warn!("$ACCEPT_HTTP1 is not set. Falling back to false.");
            false
        }
        Err(VarError::NotUnicode(_)) => panic!("$ACCEPT_HTTP1 should be written in Unicode."),
    }
}

fn fetch_memcached_url() -> String {
    match env::var("MEMCACHED_URL") {
        Ok(s) => s.parse().expect("Failed to parse $MEMCACHED_URL"),
        Err(VarError::NotPresent) => panic!("$MEMCACHED_URL is not set."),
        Err(VarError::NotUnicode(_)) => panic!("$MEMCACHED_URL should be written in Unicode."),
    }
}

fn fetch_disable_memcache_flag() -> bool {
    match env::var("DISABLE_MEMCACHE") {
        Ok(s) => s.parse().expect("Failed to parse $DISABLE_MEMCACHE"),
        Err(env::VarError::NotPresent) => {
            warn!("$DISABLE_MEMCACHE is not set. Falling back to false.");
            false
        }
        Err(VarError::NotUnicode(_)) => panic!("$DISABLE_MEMCACHE should be written in Unicode."),
    }
}

fn connect_to_memcached() -> Result<Option<memcache::Client>, anyhow::Error> {
    let memcached_url = fetch_memcached_url();
    let disable_memcache = fetch_disable_memcache_flag();
    if disable_memcache {
        warn!("In-memory cache is disabled by an environment variable.");
        return Ok(None);
    }

    match memcache::connect(memcached_url) {
        Ok(client) => Ok(Some(client)),
        Err(_) => {
            warn!("Could not communicate with memcached. In-memory cache has been disabled.");
            Ok(None)
        }
    }
}
