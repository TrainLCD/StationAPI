use async_trait::async_trait;
use mockall::automock;

use crate::domain::{entity::line::Line, error::DomainError};

#[automock]
#[async_trait]
pub trait LineRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: u32) -> Result<Option<Line>, DomainError>;
    async fn get_by_ids(&self, ids: Vec<u32>) -> Result<Vec<Line>, DomainError>;
    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Line>, DomainError>;
    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Line>, DomainError>;
}
