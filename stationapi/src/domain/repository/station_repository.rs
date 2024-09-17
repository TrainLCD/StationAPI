use async_trait::async_trait;

use crate::domain::{
    entity::{misc::StationIdWithDistance, station::Station},
    error::DomainError,
};

#[async_trait]
pub trait StationRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError>;
    async fn get_by_id_vec(&self, ids: &[u32]) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
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
        from_station_group_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Station>, DomainError>;
    async fn get_station_id_and_distance_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        line_id: Option<u32>,
    ) -> Result<StationIdWithDistance, DomainError>;
    async fn get_route_stops(
        &self,
        from_station_id: u32,
        to_station_id: u32,
    ) -> Result<Vec<Station>, DomainError>;
}
