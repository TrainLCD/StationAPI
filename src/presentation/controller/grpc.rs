use sqlx::{MySql, Pool};
use tonic::Response;

use crate::{
    domain::entity::{station::Station, train_type::TrainType},
    infrastructure::{
        company_repository::MyCompanyRepository, line_repository::MyLineRepository,
        station_repository::MyStationRepository, train_type_repository::MyTrainTypeRepository,
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

pub struct GrpcRouter {
    cache_client: Option<memcache::Client>,
    query_use_case: QueryInteractor<
        MyStationRepository,
        MyLineRepository,
        MyTrainTypeRepository,
        MyCompanyRepository,
    >,
}

impl GrpcRouter {
    pub fn new(pool: Pool<MySql>, cache_client: Option<memcache::Client>) -> Self {
        let station_repository = MyStationRepository::new(pool.clone());
        let line_repository = MyLineRepository::new(pool.clone());
        let train_type_repository = MyTrainTypeRepository::new(pool.clone());
        let company_repository = MyCompanyRepository::new(pool);

        let query_use_case = QueryInteractor {
            station_repository,
            line_repository,
            train_type_repository,
            company_repository,
            cache_client: cache_client.clone(),
        };

        Self {
            cache_client,
            query_use_case,
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

        let cache_key = format!("station:id:{}", station_id);
        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                let station =
                    serde_json::from_str::<Station>(&cache_value).expect("Failed to parse JSON");

                return Ok(Response::new(SingleStationResponse {
                    station: Some(station.into()),
                }));
            };
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
                return Err(PresentationalError::OtherError(anyhow::anyhow!(err).into()).into())
            }
        };

        if let Some(cache_client) = &self.cache_client {
            if let Ok(station_str) = serde_json::to_string(&station) {
                cache_client.set(&cache_key, station_str, 0).unwrap();
            };
        };

        Ok(Response::new(SingleStationResponse {
            station: Some(station.into()),
        }))
    }
    async fn get_stations_by_group_id(
        &self,
        request: tonic::Request<GetStationByGroupIdRequest>,
    ) -> Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let group_id = request.get_ref().group_id;

        let cache_key = format!("stations:group_id:{}", group_id);

        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                let stations = serde_json::from_str::<Vec<Station>>(&cache_value)
                    .expect("Failed to parse JSON");
                return Ok(Response::new(MultipleStationResponse {
                    stations: stations.into_iter().map(|station| station.into()).collect(),
                }));
            };
        };

        match self.query_use_case.get_stations_by_group_id(group_id).await {
            Ok(stations) => {
                if let Some(cache_client) = &self.cache_client {
                    if let Ok(station_str) = serde_json::to_string(&stations) {
                        cache_client.set(&cache_key, station_str, 0).unwrap();
                    };
                }

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
        let station_id = request.get_ref().station_id;

        let cache_key = format!(
            "stations:line_id:{}:station_id:{}",
            line_id,
            station_id.unwrap_or(0)
        );
        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                let stations = serde_json::from_str::<Vec<Station>>(&cache_value)
                    .expect("Failed to parse JSON");
                return Ok(Response::new(MultipleStationResponse {
                    stations: stations.into_iter().map(|station| station.into()).collect(),
                }));
            };
        }

        match self
            .query_use_case
            .get_stations_by_line_id(line_id, station_id)
            .await
        {
            Ok(stations) => {
                if let Some(cache_client) = &self.cache_client {
                    if let Ok(station_str) = serde_json::to_string(&stations) {
                        cache_client.set(&cache_key, station_str, 0).unwrap();
                    };
                };

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

        let cache_key = format!(
            "stations:station_name:{}:limit:{:?}",
            query_station_name,
            query_limit.clone()
        );
        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                let stations = serde_json::from_str::<Vec<Station>>(&cache_value)
                    .expect("Failed to parse JSON");
                return Ok(Response::new(MultipleStationResponse {
                    stations: stations.into_iter().map(|station| station.into()).collect(),
                }));
            };
        }

        match self
            .query_use_case
            .get_stations_by_name(query_station_name, query_limit)
            .await
        {
            Ok(stations) => {
                if let Some(cache_client) = &self.cache_client {
                    if let Ok(station_str) = serde_json::to_string(&stations) {
                        cache_client.set(&cache_key, station_str, 0).unwrap();
                    };
                };

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

        let cache_key = format!("stations:line_group_id:{}", query_line_group_id);
        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                let stations = serde_json::from_str::<Vec<Station>>(&cache_value)
                    .expect("Failed to parse JSON");
                return Ok(Response::new(MultipleStationResponse {
                    stations: stations.into_iter().map(|station| station.into()).collect(),
                }));
            };
        };

        match self
            .query_use_case
            .get_stations_by_line_group_id(query_line_group_id)
            .await
        {
            Ok(stations) => {
                if let Some(cache_client) = &self.cache_client {
                    if let Ok(station_str) = serde_json::to_string(&stations) {
                        cache_client.set(&cache_key, station_str, 0).unwrap();
                    };
                }
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
        let cache_key = format!("train_types:station_id:{:?}", query_station_id);
        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                let train_types = serde_json::from_str::<Vec<TrainType>>(&cache_value)
                    .expect("Failed to parse JSON");
                return Ok(Response::new(MultipleTrainTypeResponse {
                    train_types: train_types
                        .into_iter()
                        .map(|station| station.into())
                        .collect(),
                }));
            };
        };

        match self
            .query_use_case
            .get_train_types_by_station_id(query_station_id)
            .await
        {
            Ok(train_types) => {
                if let Some(cache_client) = &self.cache_client {
                    if let Ok(train_types_str) = serde_json::to_string(&train_types) {
                        cache_client.set(&cache_key, train_types_str, 0).unwrap();
                    };
                }
                Ok(Response::new(MultipleTrainTypeResponse {
                    train_types: train_types.into_iter().map(|tt| tt.into()).collect(),
                }))
            }
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }
}
