use std::sync::Arc;

use bigdecimal::ToPrimitive;
use moka::sync::Cache;
use sqlx::{MySql, Pool};
use tonic::Response;

use crate::{
    domain::entity::{line::Line, station::Station, train_type::TrainType},
    infrastructure::{
        line_repository::MyLineRepository, station_repository::MyStationRepository,
        train_type_repository::MyTrainTypeRepository,
    },
    pb::{
        station_api_server::StationApi, GetStationByCoordinatesRequest, GetStationByGroupIdRequest,
        GetStationByIdRequest, GetStationByLineIdRequest, GetStationsByLineGroupIdRequest,
        GetStationsByNameRequest, GetTrainTypesByStationIdRequest, MultipleStationResponse,
        MultipleTrainTypeResponse, SingleStationResponse,
    },
    presentation::error::PresentationalError,
    use_case::{interactor::query::QueryInteractor, traits::query::QueryUseCase},
};

const CACHE_SIZE: usize = 10_000;

pub struct GrpcRouter {
    query_use_case: QueryInteractor<MyStationRepository, MyLineRepository, MyTrainTypeRepository>,
}

impl GrpcRouter {
    pub fn new(pool: Pool<MySql>) -> Self {
        let station_repository_cache =
            Cache::<String, Vec<Station>>::new(CACHE_SIZE.to_u64().unwrap());
        let station_repository = MyStationRepository::new(pool.clone(), station_repository_cache);
        let line_repository_cache = Cache::<String, Vec<Line>>::new(CACHE_SIZE.to_u64().unwrap());
        let line_repository = MyLineRepository::new(pool.clone(), line_repository_cache);
        let train_type_repository_cache =
            Cache::<String, Vec<TrainType>>::new(CACHE_SIZE.to_u64().unwrap());
        let train_type_repository = MyTrainTypeRepository::new(pool, train_type_repository_cache);
        let query_use_case = QueryInteractor {
            station_repository,
            line_repository,
            train_type_repository,
        };
        Self { query_use_case }
    }
}

#[tonic::async_trait]
impl StationApi for GrpcRouter {
    async fn get_station_by_id(
        &self,
        request: tonic::Request<GetStationByIdRequest>,
    ) -> Result<tonic::Response<SingleStationResponse>, tonic::Status> {
        let station_id = request.get_ref().id;

        let station = match self.query_use_case.find_station_by_id(station_id).await {
            Ok(Some(station)) => station,
            Ok(None) => {
                return Err(PresentationalError::NotFound(format!(
                    "Station with id {} not found",
                    station_id
                ))
                .into())
            }
            Err(err) => {
                return Err(PresentationalError::OtherError(Arc::new(anyhow::anyhow!(err))).into())
            }
        };

        Ok(Response::new(SingleStationResponse {
            station: Some(station.into()),
        }))
    }
    async fn get_stations_by_group_id(
        &self,
        request: tonic::Request<GetStationByGroupIdRequest>,
    ) -> Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        match self
            .query_use_case
            .get_stations_by_group_id(request.get_ref().group_id)
            .await
        {
            Ok(stations) => Ok(Response::new(MultipleStationResponse {
                stations: stations.into_iter().map(|station| station.into()).collect(),
            })),
            Err(err) => return Err(PresentationalError::from(err).into()),
        }
    }
    async fn get_stations_by_coordinates(
        &self,
        request: tonic::Request<GetStationByCoordinatesRequest>,
    ) -> Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let request_ref: &GetStationByCoordinatesRequest = request.get_ref();
        let latitude = request_ref.latitude;
        let longitude = request_ref.longitude;
        let limit = request_ref.limit;
        let stations = match self
            .query_use_case
            .get_stations_by_coordinates(latitude, longitude, limit)
            .await
        {
            Ok(stations) => stations,
            Err(err) => return Err(PresentationalError::from(err).into()),
        };

        Ok(tonic::Response::new(MultipleStationResponse {
            stations: stations.into_iter().map(|station| station.into()).collect(),
        }))
    }
    async fn get_stations_by_line_id(
        &self,
        request: tonic::Request<GetStationByLineIdRequest>,
    ) -> Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let line_id = request.get_ref().line_id;

        match self.query_use_case.get_stations_by_line_id(line_id).await {
            Ok(stations) => {
                return Ok(Response::new(MultipleStationResponse {
                    stations: stations.into_iter().map(|station| station.into()).collect(),
                }));
            }
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }
    async fn get_stations_by_name(
        &self,
        request: tonic::Request<GetStationsByNameRequest>,
    ) -> Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let request_ref = request.get_ref();
        let query_station_name = request_ref.station_name.clone();
        let limit = request_ref.limit;

        match self
            .query_use_case
            .get_stations_by_name(query_station_name, limit)
            .await
        {
            Ok(stations) => Ok(Response::new(MultipleStationResponse {
                stations: stations.into_iter().map(|station| station.into()).collect(),
            })),
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }

    async fn get_stations_by_line_group_id(
        &self,
        request: tonic::Request<GetStationsByLineGroupIdRequest>,
    ) -> Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let request_ref = request.get_ref();
        let query_line_group_id = request_ref.line_group_id.clone();

        match self
            .query_use_case
            .get_stations_by_line_group_id(query_line_group_id)
            .await
        {
            Ok(stations) => Ok(Response::new(MultipleStationResponse {
                stations: stations.into_iter().map(|station| station.into()).collect(),
            })),
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }

    async fn get_train_types_by_station_id(
        &self,
        request: tonic::Request<GetTrainTypesByStationIdRequest>,
    ) -> Result<tonic::Response<MultipleTrainTypeResponse>, tonic::Status> {
        let request_ref: &GetTrainTypesByStationIdRequest = request.get_ref();
        let query_station_id = request_ref.station_id.clone();

        match self
            .query_use_case
            .get_train_types_by_station_id(query_station_id)
            .await
        {
            Ok(train_types) => Ok(Response::new(MultipleTrainTypeResponse {
                train_types: train_types.into_iter().map(|tt| tt.into()).collect(),
            })),
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }
}
