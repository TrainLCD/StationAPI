use crate::{
    domain::entity::gtfs::TransportType,
    infrastructure::{
        company_repository::MyCompanyRepository, line_repository::MyLineRepository,
        station_repository::MyStationRepository, train_type_repository::MyTrainTypeRepository,
    },
    presentation::error::PresentationalError,
    proto::{
        station_api_server::StationApi, GetConnectedStationsRequest, GetLineByIdListRequest,
        GetLineByIdRequest, GetLinesByNameRequest, GetRouteRequest, GetStationByCoordinatesRequest,
        GetStationByGroupIdRequest, GetStationByIdListRequest, GetStationByIdRequest,
        GetStationByLineIdRequest, GetStationsByLineGroupIdRequest, GetStationsByNameRequest,
        GetTrainTypesByStationIdRequest, MultipleLineResponse, MultipleStationResponse,
        MultipleTrainTypeResponse, Route, RouteMinimalResponse, RouteResponse, RouteTypeResponse,
        SingleLineResponse, SingleStationResponse, TransportType as GrpcTransportType,
    },
    use_case::{interactor::query::QueryInteractor, traits::query::QueryUseCase},
};
use tonic::Response;

/// Convert proto TransportType to domain TransportType
/// Returns None if unspecified (no filter), Some(type) if specified
fn convert_transport_type(proto_type: i32) -> Option<TransportType> {
    match GrpcTransportType::try_from(proto_type) {
        Ok(GrpcTransportType::Rail) => Some(TransportType::Rail),
        Ok(GrpcTransportType::Bus) => Some(TransportType::Bus),
        _ => None, // Unspecified or unknown = no filter
    }
}

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
        let request_ref = request.get_ref();
        let station_id = request_ref.id;
        let transport_type = request_ref.transport_type.and_then(convert_transport_type);

        let station = match self
            .query_use_case
            .find_station_by_id(station_id, transport_type)
            .await
        {
            Ok(Some(station)) => station,
            Ok(None) => {
                return Err(PresentationalError::NotFound(format!(
                    "Station with id {station_id} not found"
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
        let request_ref = request.get_ref();
        let station_ids = &request_ref.ids;
        let transport_type = request_ref.transport_type.and_then(convert_transport_type);

        let stations = match self
            .query_use_case
            .get_stations_by_id_vec(station_ids, transport_type)
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
        let request_ref = request.get_ref();
        let group_id = request_ref.group_id;
        let transport_type = request_ref.transport_type.and_then(convert_transport_type);

        match self
            .query_use_case
            .get_stations_by_group_id(group_id, transport_type)
            .await
        {
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
        let transport_type = request_ref.transport_type.and_then(convert_transport_type);
        let stations = match self
            .query_use_case
            .get_stations_by_coordinates(latitude, longitude, limit, transport_type)
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
        let request_ref = request.get_ref();
        let line_id = request_ref.line_id;
        let station_id = request_ref.station_id;
        let direction_id = request_ref.direction_id;
        let transport_type = request_ref.transport_type.and_then(convert_transport_type);

        match self
            .query_use_case
            .get_stations_by_line_id(line_id, station_id, direction_id, transport_type)
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
        let query_station_name = &request_ref.station_name;
        let query_limit = request_ref.limit;
        let from_station_group_id = request_ref.from_station_group_id;
        let transport_type = request_ref.transport_type.and_then(convert_transport_type);
        match self
            .query_use_case
            .get_stations_by_name(
                query_station_name.to_string(),
                query_limit,
                from_station_group_id,
                transport_type,
            )
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
        let transport_type = request_ref.transport_type.and_then(convert_transport_type);

        match self
            .query_use_case
            .get_stations_by_line_group_id(query_line_group_id, transport_type)
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
        let via_line_id = request.get_ref().via_line_id;

        match self
            .query_use_case
            .get_routes(from_id, to_id, via_line_id)
            .await
        {
            Ok(routes) => {
                return Ok(Response::new(RouteResponse {
                    routes,
                    next_page_token: "".to_string(),
                }));
            }
            Err(err) => Err(PresentationalError::from(err).into()),
        }
    }

    async fn get_routes_minimal(
        &self,
        request: tonic::Request<GetRouteRequest>,
    ) -> Result<tonic::Response<RouteMinimalResponse>, tonic::Status> {
        let from_id = request.get_ref().from_station_group_id;
        let to_id = request.get_ref().to_station_group_id;
        let via_line_id = request.get_ref().via_line_id;

        match self
            .query_use_case
            .get_routes_minimal(from_id, to_id, via_line_id)
            .await
        {
            Ok(response) => {
                return Ok(Response::new(response));
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
        let via_line_id = request.get_ref().via_line_id;

        match self
            .query_use_case
            .get_train_types(from_id, to_id, via_line_id)
            .await
        {
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
                    "Line with id {line_id} not found"
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

    async fn get_line_by_id_list(
        &self,
        request: tonic::Request<GetLineByIdListRequest>,
    ) -> Result<tonic::Response<MultipleLineResponse>, tonic::Status> {
        let line_ids = &request.get_ref().line_ids;

        let lines = match self.query_use_case.get_lines_by_id_vec(line_ids).await {
            Ok(lines) => lines,
            Err(err) => {
                return Err(PresentationalError::OtherError(anyhow::anyhow!(err).into()).into())
            }
        };

        Ok(Response::new(MultipleLineResponse {
            lines: lines.into_iter().map(|line| line.into()).collect(),
        }))
    }

    async fn get_lines_by_name(
        &self,
        request: tonic::Request<GetLinesByNameRequest>,
    ) -> Result<tonic::Response<MultipleLineResponse>, tonic::Status> {
        let request_ref = request.get_ref();
        let line_name = &request_ref.line_name;
        let limit = request_ref.limit;

        match self
            .query_use_case
            .get_lines_by_name(line_name.to_string(), limit)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::entity::{
            company::Company, gtfs::TransportType, line::Line, line_symbol::LineSymbol,
            station::Station, station_number::StationNumber, train_type::TrainType,
        },
        proto::RouteMinimalResponse,
        use_case::{error::UseCaseError, traits::query::QueryUseCase},
    };
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    // ============================================
    // Unit tests for convert_transport_type
    // ============================================

    #[test]
    fn test_convert_transport_type_rail() {
        let rail_value = GrpcTransportType::Rail as i32;
        let result = convert_transport_type(rail_value);
        assert_eq!(result, Some(TransportType::Rail));
    }

    #[test]
    fn test_convert_transport_type_bus() {
        let bus_value = GrpcTransportType::Bus as i32;
        let result = convert_transport_type(bus_value);
        assert_eq!(result, Some(TransportType::Bus));
    }

    #[test]
    fn test_convert_transport_type_unspecified() {
        let unspecified_value = GrpcTransportType::Unspecified as i32;
        let result = convert_transport_type(unspecified_value);
        assert_eq!(result, None);
    }

    #[test]
    fn test_convert_transport_type_unknown_value() {
        // Test with an unknown/invalid integer value
        let unknown_value = 999;
        let result = convert_transport_type(unknown_value);
        assert_eq!(result, None);
    }

    #[test]
    fn test_convert_transport_type_negative_value() {
        // Negative values should return None
        let negative_value = -1;
        let result = convert_transport_type(negative_value);
        assert_eq!(result, None);
    }

    // ============================================
    // Mock QueryUseCase for integration tests
    // ============================================

    /// Tracks which transport_type filter was passed to the mock
    #[derive(Clone, Default)]
    struct MockQueryUseCase {
        /// Captured transport_type from get_stations_by_coordinates calls
        captured_coordinates_transport_type: Arc<Mutex<Option<Option<TransportType>>>>,
        /// Captured transport_type from get_stations_by_name calls
        captured_name_transport_type: Arc<Mutex<Option<Option<TransportType>>>>,
        /// Stations to return (can be configured per test)
        stations_to_return: Arc<Mutex<Vec<Station>>>,
    }

    impl MockQueryUseCase {
        fn with_stations(stations: Vec<Station>) -> Self {
            Self {
                stations_to_return: Arc::new(Mutex::new(stations)),
                ..Default::default()
            }
        }

        fn get_captured_coordinates_transport_type(&self) -> Option<Option<TransportType>> {
            self.captured_coordinates_transport_type
                .lock()
                .unwrap()
                .clone()
        }

        fn get_captured_name_transport_type(&self) -> Option<Option<TransportType>> {
            self.captured_name_transport_type.lock().unwrap().clone()
        }
    }

    fn create_test_station(id: u32, transport_type: TransportType) -> Station {
        Station {
            station_cd: id as i32,
            station_g_cd: id as i32,
            station_name: format!("Station {}", id),
            station_name_k: "テスト駅".to_string(),
            station_name_r: Some("Test Station".to_string()),
            station_name_zh: None,
            station_name_ko: None,
            station_numbers: vec![],
            station_number1: None,
            station_number2: None,
            station_number3: None,
            station_number4: None,
            three_letter_code: None,
            line_cd: 1,
            line: None,
            lines: vec![],
            pref_cd: 13,
            post: "100-0001".to_string(),
            address: "Test Address".to_string(),
            lon: 139.7673068,
            lat: 35.6809591,
            open_ymd: "19900101".to_string(),
            close_ymd: "99991231".to_string(),
            e_status: 0,
            e_sort: 1,
            stop_condition: crate::proto::StopCondition::All,
            distance: None,
            has_train_types: false,
            train_type: None,
            company_cd: None,
            line_name: Some("Test Line".to_string()),
            line_name_k: Some("テストライン".to_string()),
            line_name_h: Some("テストライン".to_string()),
            line_name_r: Some("Test Line".to_string()),
            line_name_zh: None,
            line_name_ko: None,
            line_color_c: None,
            line_type: None,
            line_symbol1: None,
            line_symbol2: None,
            line_symbol3: None,
            line_symbol4: None,
            line_symbol1_color: None,
            line_symbol2_color: None,
            line_symbol3_color: None,
            line_symbol4_color: None,
            line_symbol1_shape: None,
            line_symbol2_shape: None,
            line_symbol3_shape: None,
            line_symbol4_shape: None,
            average_distance: None,
            type_id: None,
            sst_id: None,
            type_cd: None,
            line_group_cd: None,
            pass: None,
            type_name: None,
            type_name_k: None,
            type_name_r: None,
            type_name_zh: None,
            type_name_ko: None,
            color: None,
            direction: None,
            kind: None,
            transport_type,
        }
    }

    #[async_trait]
    impl QueryUseCase for MockQueryUseCase {
        async fn find_station_by_id(
            &self,
            _station_id: u32,
            _transport_type: Option<TransportType>,
        ) -> Result<Option<Station>, UseCaseError> {
            Ok(None)
        }

        async fn get_stations_by_id_vec(
            &self,
            _station_ids: &[u32],
            _transport_type: Option<TransportType>,
        ) -> Result<Vec<Station>, UseCaseError> {
            Ok(vec![])
        }

        async fn get_stations_by_group_id(
            &self,
            _station_group_id: u32,
            _transport_type: Option<TransportType>,
        ) -> Result<Vec<Station>, UseCaseError> {
            Ok(vec![])
        }

        async fn get_stations_by_group_id_vec(
            &self,
            _station_group_id_vec: &[u32],
        ) -> Result<Vec<Station>, UseCaseError> {
            Ok(vec![])
        }

        async fn get_stations_by_coordinates(
            &self,
            _latitude: f64,
            _longitude: f64,
            _limit: Option<u32>,
            transport_type: Option<TransportType>,
        ) -> Result<Vec<Station>, UseCaseError> {
            // Capture the transport_type that was passed
            *self.captured_coordinates_transport_type.lock().unwrap() = Some(transport_type);

            // Return stations, filtering by transport_type if specified
            let stations = self.stations_to_return.lock().unwrap().clone();
            let filtered: Vec<Station> = match transport_type {
                Some(tt) => stations
                    .into_iter()
                    .filter(|s| s.transport_type == tt)
                    .collect(),
                None => stations,
            };
            Ok(filtered)
        }

        async fn get_stations_by_line_id(
            &self,
            _line_id: u32,
            _station_id: Option<u32>,
            _direction_id: Option<u32>,
            _transport_type: Option<TransportType>,
        ) -> Result<Vec<Station>, UseCaseError> {
            Ok(vec![])
        }

        async fn get_stations_by_name(
            &self,
            _station_name: String,
            _limit: Option<u32>,
            _from_station_group_id: Option<u32>,
            transport_type: Option<TransportType>,
        ) -> Result<Vec<Station>, UseCaseError> {
            // Capture the transport_type that was passed
            *self.captured_name_transport_type.lock().unwrap() = Some(transport_type);

            // Return stations, filtering by transport_type if specified
            let stations = self.stations_to_return.lock().unwrap().clone();
            let filtered: Vec<Station> = match transport_type {
                Some(tt) => stations
                    .into_iter()
                    .filter(|s| s.transport_type == tt)
                    .collect(),
                None => stations,
            };
            Ok(filtered)
        }

        async fn find_company_by_id_vec(
            &self,
            _company_id_vec: &[u32],
        ) -> Result<Vec<Company>, UseCaseError> {
            Ok(vec![])
        }

        async fn update_station_vec_with_attributes(
            &self,
            stations: Vec<Station>,
            _line_group_id: Option<u32>,
            _transport_type: Option<TransportType>,
        ) -> Result<Vec<Station>, UseCaseError> {
            Ok(stations)
        }

        async fn get_lines_by_station_group_id(
            &self,
            _station_group_id: u32,
        ) -> Result<Vec<Line>, UseCaseError> {
            Ok(vec![])
        }

        async fn get_lines_by_station_group_id_vec(
            &self,
            _station_group_id_vec: &[u32],
        ) -> Result<Vec<Line>, UseCaseError> {
            Ok(vec![])
        }

        fn get_station_numbers(&self, _station: &Station) -> Vec<StationNumber> {
            vec![]
        }

        fn get_line_symbols(&self, _line: &Line) -> Vec<LineSymbol> {
            vec![]
        }

        fn extract_line_from_station(&self, station: &Station) -> Line {
            Line {
                line_cd: station.line_cd,
                company_cd: 0,
                company: None,
                line_name: "Test Line".to_string(),
                line_name_k: "テストライン".to_string(),
                line_name_h: "テストライン".to_string(),
                line_name_r: Some("Test Line".to_string()),
                line_name_zh: None,
                line_name_ko: None,
                line_color_c: None,
                line_type: None,
                line_symbols: vec![],
                line_symbol1: None,
                line_symbol2: None,
                line_symbol3: None,
                line_symbol4: None,
                line_symbol1_color: None,
                line_symbol2_color: None,
                line_symbol3_color: None,
                line_symbol4_color: None,
                line_symbol1_shape: None,
                line_symbol2_shape: None,
                line_symbol3_shape: None,
                line_symbol4_shape: None,
                e_status: 0,
                e_sort: 0,
                average_distance: None,
                station: None,
                train_type: None,
                line_group_cd: None,
                station_cd: None,
                station_g_cd: None,
                type_cd: None,
                transport_type: TransportType::Rail,
            }
        }

        async fn get_stations_by_line_group_id(
            &self,
            _line_group_id: u32,
            _transport_type: Option<TransportType>,
        ) -> Result<Vec<Station>, UseCaseError> {
            Ok(vec![])
        }

        async fn get_train_types_by_station_id(
            &self,
            _station_id: u32,
        ) -> Result<Vec<TrainType>, UseCaseError> {
            Ok(vec![])
        }

        async fn get_train_types_by_station_id_vec(
            &self,
            _station_id_vec: &[u32],
            _line_group_id: Option<u32>,
        ) -> Result<Vec<TrainType>, UseCaseError> {
            Ok(vec![])
        }

        async fn get_routes(
            &self,
            _from_station_id: u32,
            _to_station_id: u32,
            _via_line_id: Option<u32>,
        ) -> Result<Vec<crate::proto::Route>, UseCaseError> {
            Ok(vec![])
        }

        async fn get_routes_minimal(
            &self,
            _from_station_id: u32,
            _to_station_id: u32,
            _via_line_id: Option<u32>,
        ) -> Result<RouteMinimalResponse, UseCaseError> {
            Ok(RouteMinimalResponse {
                routes: vec![],
                lines: vec![],
                next_page_token: String::new(),
            })
        }

        async fn get_train_types(
            &self,
            _from_station_id: u32,
            _to_station_id: u32,
            _via_line_id: Option<u32>,
        ) -> Result<Vec<TrainType>, UseCaseError> {
            Ok(vec![])
        }

        async fn find_line_by_id(&self, _line_id: u32) -> Result<Option<Line>, UseCaseError> {
            Ok(None)
        }

        async fn get_lines_by_id_vec(&self, _line_ids: &[u32]) -> Result<Vec<Line>, UseCaseError> {
            Ok(vec![])
        }

        async fn get_lines_by_name(
            &self,
            _line_name: String,
            _limit: Option<u32>,
        ) -> Result<Vec<Line>, UseCaseError> {
            Ok(vec![])
        }

        async fn get_connected_stations(
            &self,
            _from_station_id: u32,
            _to_station_id: u32,
        ) -> Result<Vec<Station>, UseCaseError> {
            Ok(vec![])
        }
    }

    // ============================================
    // Integration tests for transport_type filtering
    // ============================================

    #[tokio::test]
    async fn test_get_stations_by_coordinates_with_rail_filter() {
        let rail_station = create_test_station(1, TransportType::Rail);
        let bus_station = create_test_station(2, TransportType::Bus);
        let mock = MockQueryUseCase::with_stations(vec![rail_station, bus_station]);

        let result = mock
            .get_stations_by_coordinates(35.0, 139.0, Some(10), Some(TransportType::Rail))
            .await
            .unwrap();

        // Verify the transport_type was captured correctly
        assert_eq!(
            mock.get_captured_coordinates_transport_type(),
            Some(Some(TransportType::Rail))
        );

        // Verify filtering works
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].transport_type, TransportType::Rail);
    }

    #[tokio::test]
    async fn test_get_stations_by_coordinates_with_bus_filter() {
        let rail_station = create_test_station(1, TransportType::Rail);
        let bus_station = create_test_station(2, TransportType::Bus);
        let mock = MockQueryUseCase::with_stations(vec![rail_station, bus_station]);

        let result = mock
            .get_stations_by_coordinates(35.0, 139.0, Some(10), Some(TransportType::Bus))
            .await
            .unwrap();

        // Verify the transport_type was captured correctly
        assert_eq!(
            mock.get_captured_coordinates_transport_type(),
            Some(Some(TransportType::Bus))
        );

        // Verify filtering works
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].transport_type, TransportType::Bus);
    }

    #[tokio::test]
    async fn test_get_stations_by_coordinates_with_no_filter() {
        let rail_station = create_test_station(1, TransportType::Rail);
        let bus_station = create_test_station(2, TransportType::Bus);
        let mock = MockQueryUseCase::with_stations(vec![rail_station, bus_station]);

        let result = mock
            .get_stations_by_coordinates(35.0, 139.0, Some(10), None)
            .await
            .unwrap();

        // Verify no filter was applied
        assert_eq!(mock.get_captured_coordinates_transport_type(), Some(None));

        // Verify all stations are returned
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_get_stations_by_name_with_rail_filter() {
        let rail_station = create_test_station(1, TransportType::Rail);
        let bus_station = create_test_station(2, TransportType::Bus);
        let mock = MockQueryUseCase::with_stations(vec![rail_station, bus_station]);

        let result = mock
            .get_stations_by_name(
                "Test".to_string(),
                Some(10),
                None,
                Some(TransportType::Rail),
            )
            .await
            .unwrap();

        // Verify the transport_type was captured correctly
        assert_eq!(
            mock.get_captured_name_transport_type(),
            Some(Some(TransportType::Rail))
        );

        // Verify filtering works
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].transport_type, TransportType::Rail);
    }

    #[tokio::test]
    async fn test_get_stations_by_name_with_bus_filter() {
        let rail_station = create_test_station(1, TransportType::Rail);
        let bus_station = create_test_station(2, TransportType::Bus);
        let mock = MockQueryUseCase::with_stations(vec![rail_station, bus_station]);

        let result = mock
            .get_stations_by_name("Test".to_string(), Some(10), None, Some(TransportType::Bus))
            .await
            .unwrap();

        // Verify the transport_type was captured correctly
        assert_eq!(
            mock.get_captured_name_transport_type(),
            Some(Some(TransportType::Bus))
        );

        // Verify filtering works
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].transport_type, TransportType::Bus);
    }

    #[tokio::test]
    async fn test_get_stations_by_name_with_no_filter() {
        let rail_station = create_test_station(1, TransportType::Rail);
        let bus_station = create_test_station(2, TransportType::Bus);
        let mock = MockQueryUseCase::with_stations(vec![rail_station, bus_station]);

        let result = mock
            .get_stations_by_name("Test".to_string(), Some(10), None, None)
            .await
            .unwrap();

        // Verify no filter was applied
        assert_eq!(mock.get_captured_name_transport_type(), Some(None));

        // Verify all stations are returned
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_convert_transport_type_integration_with_request_extraction() {
        // Test that Option<i32>.and_then(convert_transport_type) works correctly
        // This mirrors how it's used in the controller

        // Case 1: Some(Rail) -> Some(TransportType::Rail)
        let request_transport_type: Option<i32> = Some(GrpcTransportType::Rail as i32);
        let result = request_transport_type.and_then(convert_transport_type);
        assert_eq!(result, Some(TransportType::Rail));

        // Case 2: Some(Bus) -> Some(TransportType::Bus)
        let request_transport_type: Option<i32> = Some(GrpcTransportType::Bus as i32);
        let result = request_transport_type.and_then(convert_transport_type);
        assert_eq!(result, Some(TransportType::Bus));

        // Case 3: Some(Unspecified) -> None
        let request_transport_type: Option<i32> = Some(GrpcTransportType::Unspecified as i32);
        let result = request_transport_type.and_then(convert_transport_type);
        assert_eq!(result, None);

        // Case 4: None -> None
        let request_transport_type: Option<i32> = None;
        let result = request_transport_type.and_then(convert_transport_type);
        assert_eq!(result, None);
    }
}
