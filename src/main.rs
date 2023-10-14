use std::{
    env::{self, VarError},
    net::{AddrParseError, SocketAddr},
};

use sqlx::MySqlPool;
use stationapi::{
    pb::station_api_server::StationApiServer, presentation::controller::grpc::GrpcRouter,
};
use tonic::transport::Server;
use tower::make::Shared;
use tracing::info;

#[tokio::main]
async fn main() -> std::result::Result<(), anyhow::Error> {
    run().await
}

async fn run() -> std::result::Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    dotenv::from_filename(".env.local").ok();

    let addr = fetch_addr().unwrap();

    let db_url = fetch_database_url();
    let pool = MySqlPool::connect(db_url.as_str()).await?;
    let api_server = GrpcRouter::new(pool);

    info!("StationAPI Server listening on {}", addr);

    if cfg!(debug_assertions) {
        Server::builder()
            .accept_http1(true)
            .add_service(StationApiServer::new(api_server))
            .serve(addr)
            .await?;
    } else {
        let svc = Server::builder()
            .add_service(StationApiServer::new(api_server))
            .into_router();

        let h2c = h2c::H2c { s: svc };

        let server = hyper::Server::bind(&addr).serve(Shared::new(h2c));
        server.await.unwrap();
    }

    Ok(())
}

mod h2c {
    use std::pin::Pin;

    use hyper::{Body, Request, Response};
    use tower::Service;

    #[derive(Clone)]
    pub struct H2c<S> {
        pub s: S,
    }

    type BoxError = Box<dyn std::error::Error + Send + Sync>;

    impl<S> Service<Request<Body>> for H2c<S>
    where
        S: Service<Request<Body>, Response = Response<tonic::transport::AxumBoxBody>>
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
        S::Error: Into<BoxError> + Sync + Send + 'static,
        S::Response: Send + 'static,
    {
        type Response = hyper::Response<Body>;
        type Error = hyper::Error;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(
            &mut self,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn call(&mut self, mut req: hyper::Request<Body>) -> Self::Future {
            let svc = self.s.clone();
            Box::pin(async move {
                tokio::spawn(async move {
                    let upgraded_io = hyper::upgrade::on(&mut req).await.unwrap();

                    hyper::server::conn::Http::new()
                        .http2_only(true)
                        .serve_connection(upgraded_io, svc)
                        .await
                        .unwrap();
                });

                let mut res = hyper::Response::new(hyper::Body::empty());
                *res.status_mut() = http::StatusCode::SWITCHING_PROTOCOLS;
                res.headers_mut().insert(
                    http::header::UPGRADE,
                    http::header::HeaderValue::from_static("h2c"),
                );

                Ok(res)
            })
        }
    }
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
