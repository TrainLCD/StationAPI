use async_trait::async_trait;
use mockall::automock;

use crate::domain::{entity::train_type::TrainType, error::DomainError};

#[automock]
#[async_trait]
pub trait TrainTypeRepository: Send + Sync + 'static {
    async fn get_by_line_group_id(&self, line_group_id: u32)
        -> Result<Vec<TrainType>, DomainError>;
    async fn get_by_station_id(&self, station_id: u32) -> Result<Vec<TrainType>, DomainError>;
    async fn find_by_line_group_id_and_line_id(
        &self,
        line_group_id: u32,
        line_id: u32,
    ) -> Result<Option<TrainType>, DomainError>;
}
