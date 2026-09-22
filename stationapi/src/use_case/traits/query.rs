use async_trait::async_trait;

use crate::{
    domain::{
        arrival_estimation::EstimatedStop,
        entity::{
            company::Company, gtfs::TransportTypeFilter, line::Line, line_symbol::LineSymbol,
            station::Station, station_number::StationNumber, train_type::TrainType,
        },
    },
    model::{ConnectedRoute, Route, RouteLegRequest, TrainRouteSegment},
    use_case::error::UseCaseError,
};

#[async_trait]
pub trait QueryUseCase: Send + Sync + 'static {
    async fn find_station_by_id(
        &self,
        station_id: u32,
        transport_type: TransportTypeFilter,
    ) -> Result<Option<Station>, UseCaseError>;
    async fn get_stations_by_id_vec(
        &self,
        station_ids: &[u32],
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
        direction_id: Option<u32>,
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_line_id_vec(
        &self,
        line_ids: &[u32],
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
        from_station_group_id: Option<u32>,
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn find_company_by_id_vec(
        &self,
        company_id_vec: &[u32],
    ) -> Result<Vec<Company>, UseCaseError>;
    async fn get_lines_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Line>, UseCaseError>;
    async fn get_lines_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, UseCaseError>;
    fn get_station_numbers(&self, station: &Station) -> Vec<StationNumber>;
    fn get_line_symbols(&self, line: &Line) -> Vec<LineSymbol>;
    fn extract_line_from_station(&self, station: &Station) -> Line;
    async fn get_stations_by_line_group_id(
        &self,
        line_group_id: u32,
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_line_group_id_vec(
        &self,
        line_group_ids: &[u32],
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_train_types_by_station_id(
        &self,
        station_id: u32,
    ) -> Result<Vec<TrainType>, UseCaseError>;
    async fn get_train_types_by_station_id_vec(
        &self,
        station_id_vec: &[u32],
        line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, UseCaseError>;
    async fn get_routes(
        &self,
        from_station_id: u32,
        to_station_id: u32,
        via_line_id: Option<u32>,
    ) -> Result<Vec<Route>, UseCaseError>;
    async fn get_train_types(
        &self,
        from_station_id: u32,
        to_station_id: u32,
        via_line_id: Option<u32>,
    ) -> Result<Vec<TrainType>, UseCaseError>;
    async fn get_train_route(
        &self,
        from_station_id: u32,
        to_station_id: u32,
        line_group_id: Option<u32>,
    ) -> Result<Vec<TrainRouteSegment>, UseCaseError>;
    async fn find_line_by_id(&self, line_id: u32) -> Result<Option<Line>, UseCaseError>;
    async fn get_lines_by_id_vec(&self, line_ids: &[u32]) -> Result<Vec<Line>, UseCaseError>;
    async fn get_lines_by_name(
        &self,
        line_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Line>, UseCaseError>;
    async fn get_connected_routes(
        &self,
        from_station_group_id: u32,
        to_station_group_id: u32,
        via_line_id: Option<u32>,
    ) -> Result<Vec<ConnectedRoute>, UseCaseError>;
    /// 乗換経路 (`connectedRoutes` の区間の並び) の各駅の推定到着時間。
    /// 出発駅からの累積で、乗換ごとに徒歩と乗換先の待ち時間の見込みを加える。
    async fn estimate_connected_route_arrival_times(
        &self,
        legs: &[RouteLegRequest],
    ) -> Result<Vec<EstimatedStop>, UseCaseError>;
    /// 乗換経路 (`connectedRoutes` の区間の並び) の走行区間を、区間の順につなげたもの。
    async fn get_connected_train_route(
        &self,
        legs: &[RouteLegRequest],
    ) -> Result<Vec<TrainRouteSegment>, UseCaseError>;
    async fn estimate_route_arrival_times(
        &self,
        from_station_id: u32,
        to_station_id: u32,
        via_line_ids: &[u32],
        direction_id: Option<u32>,
    ) -> Result<Vec<EstimatedStop>, UseCaseError>;
}
