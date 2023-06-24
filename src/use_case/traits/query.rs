use async_trait::async_trait;
use mockall::automock;

use crate::{domain::entity::station::Station, use_case::error::UseCaseError};

#[automock]
#[async_trait]
pub trait QueryUseCase: Send + Sync + 'static {
    async fn find_station_by_id(&self, station_id: u32) -> Result<Option<Station>, UseCaseError>;
    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_line_id(&self, line_id: u32) -> Result<Vec<Station>, UseCaseError>;
    async fn get_stations_by_name(
        &self,
        station_name: String,
        get_stations_by_name: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError>;
}
