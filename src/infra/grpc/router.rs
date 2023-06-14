use std::env;

use anyhow::Result;
use sqlx::{MySql, MySqlPool, Pool};
use tonic::{transport::Server, Response};
use tower_http::cors::CorsLayer;
use tracing_log::LogTracer;

use crate::{
    domain::models::{
        company::company_repository::CompanyRepository, line::line_repository::LineRepository,
        station::station_repository::StationRepository,
    },
    infra::{
        company::company_repository_impl::CompanyRepositoryImpl,
        line::line_repository_impl::LineRepositoryImpl,
    },
    pb::{
        station_api_server::{StationApi, StationApiServer},
        GetStationByCoordinatesRequest, GetStationByGroupIdRequest, GetStationByIdRequest,
        GetStationByLineIdRequest, GetStationByNameRequest, MultipleStationResponse,
        SingleStationResponse,
    },
};

use super::handlers;

pub struct ApiContext {
    pool: Pool<MySql>,
}

impl ApiContext {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
    pub fn station_repository(&self) -> impl StationRepository {
        use crate::infra::station::station_repository_impl::StationRepositoryImpl;
        StationRepositoryImpl {
            pool: Box::new(self.pool.to_owned()),
        }
    }
    pub fn line_repository(&self) -> impl LineRepository {
        LineRepositoryImpl {
            pool: Box::new(self.pool.to_owned()),
        }
    }
    pub fn company_repository(&self) -> impl CompanyRepository {
        CompanyRepositoryImpl {
            pool: Box::new(self.pool.to_owned()),
        }
    }
}

#[tonic::async_trait]
impl StationApi for ApiContext {
    async fn get_station_by_id(
        &self,
        request: tonic::Request<GetStationByIdRequest>,
    ) -> std::result::Result<tonic::Response<SingleStationResponse>, tonic::Status> {
        let res = handlers::get_station_by_id(self, request).await.unwrap();
        Ok(Response::new(res))
    }
    async fn get_station_by_group_id(
        &self,
        request: tonic::Request<GetStationByGroupIdRequest>,
    ) -> std::result::Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let res = handlers::get_station_by_group_id(self, request)
            .await
            .unwrap();
        Ok(Response::new(res))
    }
    async fn get_station_by_coordinates(
        &self,
        request: tonic::Request<GetStationByCoordinatesRequest>,
    ) -> std::result::Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let res: MultipleStationResponse = handlers::get_station_by_coordinates(self, request)
            .await
            .unwrap();
        Ok(Response::new(res))
    }
    async fn get_stations_by_line_id(
        &self,
        request: tonic::Request<GetStationByLineIdRequest>,
    ) -> std::result::Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let res = handlers::get_stations_by_line_id(self, request)
            .await
            .unwrap();
        Ok(Response::new(res))
    }
    async fn get_stations_by_name(
        &self,
        request: tonic::Request<GetStationByNameRequest>,
    ) -> std::result::Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let res = handlers::get_stations_by_station_name(self, request)
            .await
            .unwrap();
        Ok(Response::new(res))
    }
}

pub async fn run() -> Result<()> {
    LogTracer::init()?;
    dotenv::from_filename(".env.local").ok();

    let port = env::var("PORT").unwrap_or("50051".to_string());
    let addr = format!("[::]:{}", port).parse().unwrap();

    let db_url = env::var("DATABASE_URL").unwrap();
    let pool = MySqlPool::connect(db_url.as_str()).await?;
    let api_server = ApiContext::new(pool);
    let api_server = StationApiServer::new(api_server);

    let allow_cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any);

    println!("StationAPI Server listening on {}", addr);

    Server::builder()
        .accept_http1(true)
        .layer(allow_cors)
        .add_service(tonic_web::enable(api_server))
        .serve(addr)
        .await?;

    Ok(())
}
