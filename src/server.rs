use std::env;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use sqlx::{MySql, MySqlPool, Pool};
use stationapi::{
    repositories::station::StationRepositoryImplOnMySQL,
    service::{
        station_api_server::{StationApi, StationApiServer},
        GetStationByCoordinatesRequest, GetStationByGroupIdRequest, GetStationByIdRequest,
        MultipleStationResponse, SingleStationResponse,
    },
    usecases::station::{
        find_station_by_id, get_stations_by_coordinates, get_stations_by_group_id,
    },
};
use tonic::{transport::Server, Request, Response, Status};
use tonic_web::GrpcWebLayer;
use tower_http::cors::CorsLayer;
use url::Url;

pub mod service {
    tonic::include_proto!("app.traincd.grpc");
}

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    dotenv::from_filename(".env.local").ok();

    let port = env::var("PORT").unwrap_or("50051".to_string());
    let addr = format!("[::]:{}", port).parse().unwrap();

    let host = env::var("MYSQL_HOST").unwrap();
    let database = env::var("MYSQL_DATABASE").unwrap();
    let user = env::var("MYSQL_USER").unwrap();
    let pass = env::var("MYSQL_PASSWORD").unwrap();
    let db_uri = format!("mysql://{}:3306/{}", host, database);
    let mut uri = Url::parse(&db_uri).unwrap();
    uri.set_username(user.as_str()).unwrap();
    uri.set_password(Some(
        utf8_percent_encode(pass.as_str(), NON_ALPHANUMERIC)
            .to_string()
            .as_str(),
    ))
    .unwrap();
    let uri = uri.as_str();
    let pool = MySqlPool::connect(uri).await?;

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
