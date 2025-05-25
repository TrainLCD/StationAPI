use crate::{
    infrastructure::{
        company_repository::MyCompanyRepository, line_repository::MyLineRepository,
        station_repository::MyStationRepository, train_type_repository::MyTrainTypeRepository,
    },
    presentation::error::PresentationalError,
    proto::{
        station_api_server::StationApi, GetConnectedStationsRequest, GetLineByIdRequest,
        GetLinesByNameRequest, GetRouteRequest, GetStationByCoordinatesRequest,
        GetStationByGroupIdRequest, GetStationByIdListRequest, GetStationByIdRequest,
        GetStationByLineIdRequest, GetStationsByLineGroupIdRequest, GetStationsByNameRequest,
        GetTrainTypesByStationIdRequest, MultipleLineResponse, MultipleStationResponse,
        MultipleTrainTypeResponse, Route, RouteResponse, RouteTypeResponse, SingleLineResponse,
        SingleStationResponse,
    },
    use_case::{interactor::query::QueryInteractor, traits::query::QueryUseCase},
};
use tonic::Response;

pub struct MyApi {
    pub query_use_case: QueryInteractor<
        MyStationRepository,
        MyLineRepository,
        MyTrainTypeRepository,
        MyCompanyRepository,
    >,
}

#[tonic::async_trait]
impl StationApi for MyApi {
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
                return Err(PresentationalError::OtherError(anyhow::anyhow!(err).into()).into())
            }
        };

        Ok(Response::new(SingleStationResponse {
            station: Some(station.into()),
        }))
    }

    async fn get_station_by_id_list(
        &self,
        request: tonic::Request<GetStationByIdListRequest>,
    ) -> Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let station_ids = &request.get_ref().ids;

        let stations = match self
            .query_use_case
            .get_stations_by_id_vec(station_ids)
            .await
        {
            Ok(stations) => stations,
            Err(err) => {
                return Err(PresentationalError::OtherError(anyhow::anyhow!(err).into()).into())
            }
        };

        Ok(Response::new(MultipleStationResponse {
            stations: stations.into_iter().map(|station| station.into()).collect(),
        }))
    }

    async fn get_stations_by_group_id(
        &self,
        request: tonic::Request<GetStationByGroupIdRequest>,
    ) -> Result<tonic::Response<MultipleStationResponse>, tonic::Status> {
        let group_id = request.get_ref().group_id;

        match self.query_use_case.get_stations_by_group_id(group_id).await {
            Ok(stations) => {
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

        match self
            .query_use_case
            .get_stations_by_line_id(line_id, station_id)
            .await
        {
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
        let query_limit = request_ref.limit;
        let from_station_group_id = request_ref.from_station_group_id;
        match self
            .query_use_case
            .get_stations_by_name(query_station_name, query_limit, from_station_group_id)
            .await
        {
            Ok(stations) => {
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

        match self
            .query_use_case
            .get_stations_by_line_group_id(query_line_group_id)
            .await
        {
            Ok(stations) => {
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

    async fn get_routes(
        &self,
        request: tonic::Request<GetRouteRequest>,
    ) -> Result<tonic::Response<RouteResponse>, tonic::Status> {
        let from_id = request.get_ref().from_station_group_id;
        let to_id = request.get_ref().to_station_group_id;

        match self.query_use_case.get_routes(from_id, to_id).await {
            Ok(routes) => {
                return Ok(Response::new(RouteResponse {
                    routes,
                    next_page_token: "".to_string(),
                }));
            }
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }

    async fn get_route_types(
        &self,
        request: tonic::Request<GetRouteRequest>,
    ) -> Result<tonic::Response<RouteTypeResponse>, tonic::Status> {
        let from_id = request.get_ref().from_station_group_id;
        let to_id = request.get_ref().to_station_group_id;

        match self.query_use_case.get_train_types(from_id, to_id).await {
            Ok(train_types) => {
                return Ok(Response::new(RouteTypeResponse {
                    train_types: train_types.into_iter().map(|t| t.into()).collect(),
                    next_page_token: "".to_string(),
                }));
            }
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }

    async fn get_line_by_id(
        &self,
        request: tonic::Request<GetLineByIdRequest>,
    ) -> Result<tonic::Response<SingleLineResponse>, tonic::Status> {
        let line_id = request.get_ref().line_id;

        let line = match self.query_use_case.find_line_by_id(line_id).await {
            Ok(Some(line)) => line,
            Ok(None) => {
                return Err(PresentationalError::NotFound(format!(
                    "Line with id {} not found",
                    line_id
                ))
                .into())
            }
            Err(err) => {
                return Err(PresentationalError::OtherError(anyhow::anyhow!(err).into()).into())
            }
        };

        Ok(Response::new(SingleLineResponse {
            line: Some(line.into()),
        }))
    }

    async fn get_lines_by_name(
        &self,
        request: tonic::Request<GetLinesByNameRequest>,
    ) -> Result<tonic::Response<MultipleLineResponse>, tonic::Status> {
        let line_name = request.get_ref().line_name.clone();
        let limit = request.get_ref().limit;

        match self
            .query_use_case
            .get_lines_by_name(line_name, limit)
            .await
        {
            Ok(lines) => {
                return Ok(Response::new(MultipleLineResponse {
                    lines: lines.into_iter().map(|line| line.into()).collect(),
                }));
            }
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }

    async fn get_connected_routes(
        &self,
        request: tonic::Request<GetConnectedStationsRequest>,
    ) -> Result<tonic::Response<RouteResponse>, tonic::Status> {
        let from_station_group_id = request.get_ref().from_station_group_id;
        let to_station_group_id = request.get_ref().to_station_group_id;

        match self
            .query_use_case
            .get_connected_stations(from_station_group_id, to_station_group_id)
            .await
        {
            Ok(stations) => Ok(Response::new(RouteResponse {
                routes: vec![Route {
                    id: 0,
                    stops: stations.into_iter().map(|station| station.into()).collect(),
                }],
                next_page_token: "".to_string(),
            })),
            Err(err) => {
                return Err(PresentationalError::OtherError(anyhow::anyhow!(err).into()).into())
            }
        }
    }
}
