use std::env;

use service::{
    station_api_server::{StationApi, StationApiServer},
    SingleStationReply, SingleStationRequest,
};
use sqlx::{MySql, MySqlPool, Pool};
use stationapi::{
    repositories::station::StationRepositoryImplOnMySQL, usecases::station::find_one_station,
};
use tonic::{transport::Server, Request, Response, Status};
use tonic_web::GrpcWebLayer;
use tower_http::cors::CorsLayer;

pub mod service {
    tonic::include_proto!("app.traincd.grpc");
}

#[derive(Debug)]
pub struct MyApi {
    pool: Pool<MySql>,
}

#[tonic::async_trait]
impl StationApi for MyApi {
    async fn single_station(
        &self,
        request: Request<SingleStationRequest>,
    ) -> Result<Response<SingleStationReply>, Status> {
        if let Some(station) = find_one_station(
            StationRepositoryImplOnMySQL { pool: &self.pool },
            request.into_inner().id,
        )
        .await
        {
            let reply = SingleStationReply {
                name: station.station_name,
            };
            return Ok(Response::new(reply));
        }
        Err(Status::not_found("The station is not found"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    dotenv::from_filename(".env").ok();

    let addr = "[::1]:50051".parse().unwrap();

    let pool = MySqlPool::connect(&env::var("DATABASE_URL")?).await?;

    let api_server = MyApi { pool };
    let api_server = StationApiServer::new(api_server);

    let allow_cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any);

    println!("StationAPI Server listening on {}", addr);

    Server::builder()
        .accept_http1(true)
        .layer(allow_cors)
        .layer(GrpcWebLayer::new())
        .add_service(tonic_web::enable(api_server))
        .serve(addr)
        .await?;

    Ok(())
}
