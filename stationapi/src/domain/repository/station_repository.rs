use async_trait::async_trait;

use crate::domain::{entity::station::Station, error::DomainError};

#[async_trait]
pub trait StationRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: i64) -> Result<Option<Station>, DomainError>;
    async fn get_by_id_vec(&self, ids: Vec<i64>) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_id(
        &self,
        line_id: i64,
        station_id: Option<i64>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id(
        &self,
        station_group_id: i64,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: Vec<i64>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<i64>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<i64>,
        from_station_group_id: Option<i64>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_group_id(&self, line_group_id: i64) -> Result<Vec<Station>, DomainError>;
    async fn get_route_stops(
        &self,
        from_station_id: i64,
        to_station_id: i64,
    ) -> Result<Vec<Station>, DomainError>;
}
