use std::env;

use sqlx::{MySql, MySqlPool, Pool};
use stationapi::{
    repositories::station::StationRepositoryImplOnMySQL,
    service::{
        station_api_server::{StationApi, StationApiServer},
        GetStationByCoordinatesRequest, GetStationByGroupIdRequest, GetStationByIdRequest,
        GetStationByLineIdRequest, MultipleStationResponse, SingleStationResponse,
    },
    usecases::station::{
        find_station_by_id, get_stations_by_coordinates, get_stations_by_group_id,
        get_stations_by_line_id,
    },
};
use tonic::{transport::Server, Request, Response, Status};
use tonic_web::GrpcWebLayer;
use tower_http::cors::CorsLayer;
use tracing_log::LogTracer;

#[derive(Debug)]
pub struct MyApi {
    pool: Pool<MySql>,
}

#[tonic::async_trait]
impl StationApi for MyApi {
    async fn get_station_by_id(
        &self,
        request: Request<GetStationByIdRequest>,
    ) -> Result<Response<SingleStationResponse>, Status> {
        if let Some(resp) = find_station_by_id(
            StationRepositoryImplOnMySQL { pool: &self.pool },
            request.into_inner().id,
        )
        .await
        {
            return Ok(Response::new(resp));
        }
        Err(Status::not_found("The station is not found"))
    }

    async fn get_station_by_coordinates(
        &self,
        request: Request<GetStationByCoordinatesRequest>,
    ) -> Result<Response<MultipleStationResponse>, Status> {
        let req_inner = request.into_inner();
        let resp = get_stations_by_coordinates(
            StationRepositoryImplOnMySQL { pool: &self.pool },
            req_inner.latitude,
            req_inner.longitude,
            req_inner.limit,
        )
        .await;
        Ok(Response::new(resp))
    }

    async fn get_station_by_group_id(
        &self,
        request: Request<GetStationByGroupIdRequest>,
    ) -> Result<Response<MultipleStationResponse>, Status> {
        let req_inner = request.into_inner();
        let resp = get_stations_by_group_id(
            StationRepositoryImplOnMySQL { pool: &self.pool },
            req_inner.group_id,
        )
        .await;
        Ok(Response::new(resp))
    }

    async fn get_stations_by_line_id(
        &self,
        request: Request<GetStationByLineIdRequest>,
    ) -> Result<Response<MultipleStationResponse>, Status> {
        let req_inner = request.into_inner();
        let resp = get_stations_by_line_id(
            StationRepositoryImplOnMySQL { pool: &self.pool },
            req_inner.line_id,
        )
        .await;
        Ok(Response::new(resp))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    LogTracer::init()?;

    dotenv::from_filename(".env.local").ok();

    let port = env::var("PORT").unwrap_or("50051".to_string());
    let addr = format!("[::]:{}", port).parse().unwrap();

    let db_url = env::var("DATABASE_URL").unwrap();
    let pool = MySqlPool::connect(db_url.as_str()).await?;

    let api_server = MyApi { pool };
    let api_server = StationApiServer::new(api_server);

    let allow_cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any);

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<StationApiServer<MyApi>>()
        .await;

    println!("StationAPI Server listening on {}", addr);

    Server::builder()
        .accept_http1(true)
        .layer(allow_cors)
        .layer(GrpcWebLayer::new())
        .add_service(health_service)
        .add_service(tonic_web::enable(api_server))
        .serve(addr)
        .await?;

    Ok(())
}
