use async_trait::async_trait;
use mockall::automock;

use crate::domain::{entity::station::Station, error::DomainError};

#[automock]
#[async_trait]
pub trait StationRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError>;
    async fn get_by_line_id(&self, line_id: u32) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: Vec<u32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Station>, DomainError>;
}
