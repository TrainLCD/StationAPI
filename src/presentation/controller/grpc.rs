use bigdecimal::ToPrimitive;
use moka::future::Cache;
use sqlx::{MySql, Pool};
use std::sync::Arc;
use tonic::Response;

use crate::{
    domain::entity::{station::Station as StationEntity, train_type::TrainType as TrainTypeEntity},
    infrastructure::{
        company_repository::MyCompanyRepository, line_repository::MyLineRepository,
        station_repository::MyStationRepository, train_type_repository::MyTrainTypeRepository,
    },
    pb::{
        station_api_server::StationApi, GetStationByCoordinatesRequest, GetStationByGroupIdRequest,
        GetStationByIdRequest, GetStationByLineIdRequest, GetStationsByLineGroupIdRequest,
        GetStationsByNameRequest, GetTrainTypesByStationIdRequest, MultipleStationResponse,
        MultipleTrainTypeResponse, SingleStationResponse, Station as PbStation,
        TrainType as PbTrainType,
    },
    presentation::error::PresentationalError,
    use_case::{interactor::query::QueryInteractor, traits::query::QueryUseCase},
};

const CACHE_SIZE: usize = 10_000;

pub struct GrpcRouter {
    station_list_cache: Cache<String, Arc<Vec<StationEntity>>>,
    train_types_cache: Cache<String, Arc<Vec<TrainTypeEntity>>>,
    query_use_case: QueryInteractor<
        MyStationRepository,
        MyLineRepository,
        MyTrainTypeRepository,
        MyCompanyRepository,
    >,
}

impl GrpcRouter {
    pub fn new(pool: Pool<MySql>) -> Self {
        let station_repository = MyStationRepository::new(pool.clone());
        let line_repository = MyLineRepository::new(pool.clone());
        let train_type_repository = MyTrainTypeRepository::new(pool.clone());
        let company_repository = MyCompanyRepository::new(pool);

        let query_use_case = QueryInteractor {
            station_repository,
            line_repository,
            train_type_repository,
            company_repository,
        };

        Self {
            query_use_case,
            station_list_cache: Cache::new(CACHE_SIZE.to_u64().unwrap()),
            train_types_cache: Cache::new(CACHE_SIZE.to_u64().unwrap()),
        }
    }
}

#[tonic::async_trait]
impl StationApi for GrpcRouter {
    async fn get_station_by_id(
        &self,
        request: tonic::Request<GetStationByIdRequest>,
    ) -> Result<tonic::Response<SingleStationResponse>, tonic::Status> {
        let station_id = request.get_ref().id;

        let cache = self.station_list_cache.clone();
        let cache_key = format!("station:id:{}", station_id);
        if let Some(cache_data) = cache.get(&cache_key) {
            return Ok(Response::new(SingleStationResponse {
                station: Some(
                    cache_data
                        .first()
                        .map(|station| station.clone().into())
                        .unwrap(),
                ),
            }));
        };

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

        cache
            .insert(cache_key, Arc::new(vec![station.clone()]))
            .await;

        Ok(Response::new(SingleStationResponse {
            station: Some(station.into()),
        }))
    }
    async fn get_stations_by_group_id(
        &self,
        request: tonic::Request<GetStationByGroupIdRequest>,
    ) -> Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let group_id = request.get_ref().group_id;

        let cache = self.station_list_cache.clone();
        let cache_key = format!("stations:group_id:{}", group_id);
        if let Some(stations) = cache.get(&cache_key) {
            let stations = stations.to_vec();
            return Ok(Response::new(MultipleStationResponse {
                stations: stations.into_iter().map(|station| station.into()).collect(),
            }));
        };

        match self.query_use_case.get_stations_by_group_id(group_id).await {
            Ok(stations) => {
                cache.insert(cache_key, Arc::new(stations.clone())).await;

                return Ok(Response::new(MultipleStationResponse {
                    stations: stations.into_iter().map(|station| station.into()).collect(),
                }));
            }
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
        let via_station_id = &request.get_ref().via_station_id;

        let cache = self.station_list_cache.clone();
        let cache_key = format!(
            "stations:line_id:{}:via_station_id:{}",
            line_id,
            via_station_id.unwrap_or(0)
        );
        if let Some(stations) = cache.get(&cache_key) {
            let stations = stations.to_vec();
            return Ok(Response::new(MultipleStationResponse {
                stations: stations.into_iter().map(|station| station.into()).collect(),
            }));
        };

        match self
            .query_use_case
            .get_stations_by_line_id(line_id, via_station_id)
            .await
        {
            Ok(stations) => {
                cache.insert(cache_key, Arc::new(stations.clone())).await;

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
        let query_limit = request_ref.limit;

        let cache: Cache<String, Arc<Vec<StationEntity>>> = self.station_list_cache.clone();
        let cache_key = format!(
            "stations:station_name:{}:limit:{:?}",
            query_station_name,
            query_limit.clone()
        );
        if let Some(cache_data) = cache.get(&cache_key) {
            let stations = cache_data.to_vec();
            let stations: Vec<PbStation> =
                stations.into_iter().map(|station| station.into()).collect();
            return Ok(Response::new(MultipleStationResponse { stations }));
        };

        match self
            .query_use_case
            .get_stations_by_name(query_station_name, query_limit)
            .await
        {
            Ok(stations) => {
                cache.insert(cache_key, Arc::new(stations.clone())).await;

                return Ok(Response::new(MultipleStationResponse {
                    stations: stations.into_iter().map(|station| station.into()).collect(),
                }));
            }
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }

    async fn get_stations_by_line_group_id(
        &self,
        request: tonic::Request<GetStationsByLineGroupIdRequest>,
    ) -> Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let request_ref = request.get_ref();
        let query_line_group_id = request_ref.line_group_id;

        let cache = self.station_list_cache.clone();
        let cache_key = format!("stations:line_group_id:{}", query_line_group_id);
        if let Some(cache_data) = cache.get(&cache_key) {
            let stations = cache_data.to_vec();
            let stations: Vec<PbStation> =
                stations.into_iter().map(|station| station.into()).collect();
            return Ok(Response::new(MultipleStationResponse { stations }));
        };

        match self
            .query_use_case
            .get_stations_by_line_group_id(query_line_group_id)
            .await
        {
            Ok(stations) => {
                cache.insert(cache_key, Arc::new(stations.clone())).await;

                return Ok(Response::new(MultipleStationResponse {
                    stations: stations.into_iter().map(|station| station.into()).collect(),
                }));
            }
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }

    async fn get_train_types_by_station_id(
        &self,
        request: tonic::Request<GetTrainTypesByStationIdRequest>,
    ) -> Result<tonic::Response<MultipleTrainTypeResponse>, tonic::Status> {
        let request_ref: &GetTrainTypesByStationIdRequest = request.get_ref();
        let query_station_id = request_ref.station_id;

        let cache = self.train_types_cache.clone();
        let cache_key = format!("train_types:station_id:{:?}", query_station_id);
        if let Some(cache_data) = cache.get(&cache_key) {
            let train_types = cache_data.to_vec();
            let train_types: Vec<PbTrainType> = train_types
                .into_iter()
                .map(|station| station.into())
                .collect();
            return Ok(Response::new(MultipleTrainTypeResponse { train_types }));
        };

        match self
            .query_use_case
            .get_train_types_by_station_id(query_station_id)
            .await
        {
            Ok(train_types) => {
                cache.insert(cache_key, Arc::new(train_types.clone())).await;

                Ok(Response::new(MultipleTrainTypeResponse {
                    train_types: train_types.into_iter().map(|tt| tt.into()).collect(),
                }))
            }
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }
}
