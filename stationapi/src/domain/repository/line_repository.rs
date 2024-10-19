use async_trait::async_trait;

use crate::domain::{entity::line::Line, error::DomainError};

#[async_trait]
pub trait LineRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: u32) -> Result<Option<Line>, DomainError>;
    async fn find_by_station_id(&self, station_id: u32) -> Result<Option<Line>, DomainError>;
    async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<Line>, DomainError>;
    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Line>, DomainError>;
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError>;
    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Line>, DomainError>;
    async fn get_by_line_group_id_vec(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError>;
    // FIXME: もっとマシな命名
    async fn get_by_line_group_id_vec_for_routes(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError>;
    async fn get_by_name(
        &self,
        line_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Line>, DomainError>;
}
