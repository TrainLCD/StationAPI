use async_trait::async_trait;

use crate::domain::{
    entity::{misc::StationIdWithDistance, station::Station},
    error::DomainError,
};

#[async_trait]
pub trait StationRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: i32) -> Result<Option<Station>, DomainError>;
    async fn get_by_id_vec(&self, ids: Vec<i32>) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_id(
        &self,
        line_id: i32,
        station_id: Option<i32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id(
        &self,
        station_group_id: i32,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: Vec<i32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<i32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<i32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_group_id(&self, line_group_id: i32) -> Result<Vec<Station>, DomainError>;
    async fn get_station_id_and_distance_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        line_id: Option<i32>,
    ) -> Result<StationIdWithDistance, DomainError>;
}
