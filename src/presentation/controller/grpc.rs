use std::sync::Arc;

use sqlx::{MySql, Pool};
use tonic::Response;

use crate::{
    infrastructure::{line_repository::MyLineRepository, station_repository::MyStationRepository},
    pb::{
        station_api_server::StationApi, GetStationByCoordinatesRequest, GetStationByGroupIdRequest,
        GetStationByIdRequest, GetStationByLineIdRequest, GetStationByNameRequest,
        MultipleStationResponse, SingleStationResponse,
    },
    presentation::error::PresentationalError,
    use_case::{interactor::query::QueryInteractor, traits::query::QueryUseCase},
};

pub struct GrpcRouter {
    query_use_case: QueryInteractor<MyStationRepository, MyLineRepository>,
}

impl GrpcRouter {
    pub fn new(pool: Pool<MySql>) -> Self {
        let station_repository = MyStationRepository::new(pool.clone());
        let line_repository = MyLineRepository::new(pool);
        let query_use_case = QueryInteractor {
            station_repository,
            line_repository,
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
                stations: stations
                    .into_iter()
                    .map(|station| SingleStationResponse {
                        station: Some(station.into()),
                    })
                    .collect(),
            })),
            Err(err) => Err(PresentationalError::from(err).into()),
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
            stations: stations
                .into_iter()
                .map(|station| {
                    let res = station.into();
                    SingleStationResponse { station: Some(res) }
                })
                .collect(),
        }))
    }
    async fn get_stations_by_line_id(
        &self,
        request: tonic::Request<GetStationByLineIdRequest>,
    ) -> Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let line_id = request.get_ref().line_id;

        match self.query_use_case.get_stations_by_line_id(line_id).await {
            Ok(stations) => Ok(Response::new(MultipleStationResponse {
                stations: stations
                    .into_iter()
                    .map(|station| SingleStationResponse {
                        station: Some(station.into()),
                    })
                    .collect(),
            })),
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }
    async fn get_stations_by_name(
        &self,
        request: tonic::Request<GetStationByNameRequest>,
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
                stations: stations
                    .into_iter()
                    .map(|station| SingleStationResponse {
                        station: Some(station.into()),
                    })
                    .collect(),
            })),
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }
}
