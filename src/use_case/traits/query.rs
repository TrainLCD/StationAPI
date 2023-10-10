use async_trait::async_trait;
use mockall::automock;

use crate::{
    domain::entity::{
        company::Company, line::Line, line_symbol::LineSymbol, station::Station,
        station_number::StationNumber, train_type::TrainType,
    },
    use_case::error::UseCaseError,
};

#[automock]
#[async_trait]
pub trait QueryUseCase: Send + Sync + 'static {
    async fn find_station_by_id(&self, station_id: u32) -> Result<Option<Station>, UseCaseError>;
    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_group_id_vec(
        &self,
        station_group_id_vec: Vec<u32>,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_line_id(
        &self,
        line_id: u32,
        via_station_id: &Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_name(
        &self,
        station_name: String,
        get_stations_by_name: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn find_company_by_id_vec(
        &self,
        company_id_vec: Vec<u32>,
    ) -> Result<Vec<Company>, UseCaseError>;
    async fn update_station_vec_with_attributes(
        &self,
        station: &mut Vec<Station>,
    ) -> Result<(), UseCaseError>;
    async fn get_lines_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Line>, UseCaseError>;
    async fn get_lines_by_station_group_id_vec(
        &self,
        station_group_id_vec: Vec<u32>,
    ) -> Result<Vec<Line>, UseCaseError>;
    fn get_station_numbers(&self, station: &Station) -> Vec<StationNumber>;
    fn get_line_symbols(&self, line: &Line) -> Vec<LineSymbol>;
    fn extract_line_from_station(&self, station: &Station) -> Line;
    async fn get_stations_by_line_group_id(
        &self,
        line_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_train_types_by_station_id(
        &self,
        station_id: u32,
    ) -> Result<Vec<TrainType>, UseCaseError>;
    async fn get_train_types_by_station_id_vec(
        &self,
        station_id_vec: Vec<u32>,
    ) -> Result<Vec<TrainType>, UseCaseError>;
}
