use async_trait::async_trait;

use crate::domain::{entity::train_type::TrainType, error::DomainError};

#[async_trait]
pub trait TrainTypeRepository: Send + Sync + 'static {
    async fn get_by_line_group_id(&self, line_group_id: i64)
        -> Result<Vec<TrainType>, DomainError>;
    async fn get_by_station_id(&self, station_id: i64) -> Result<Vec<TrainType>, DomainError>;
    async fn find_by_line_group_id_and_line_id(
        &self,
        line_group_id: i64,
        line_id: i64,
    ) -> Result<Option<TrainType>, DomainError>;
    async fn get_by_station_id_vec(
        &self,
        station_id_vec: Vec<i64>,
        line_group_id: Option<i64>,
    ) -> Result<Vec<TrainType>, DomainError>;
    async fn get_types_by_station_id_vec(
        &self,
        station_id_vec: Vec<i64>,
        line_group_id: Option<i64>,
    ) -> Result<Vec<TrainType>, DomainError>;
    async fn get_by_line_group_id_vec(
        &self,
        line_group_id_vec: Vec<i64>,
    ) -> Result<Vec<TrainType>, DomainError>;
}
