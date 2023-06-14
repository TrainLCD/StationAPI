use anyhow::Result;
use mockall::automock;

use super::line_model::Line;

#[async_trait::async_trait]
#[automock]
pub trait LineRepository {
    async fn find_by_id(&self, id: u32) -> Result<Line>;
    async fn find_by_station_id(&self, id: u32) -> Result<Line>;
    async fn get_by_station_group_id(&self, station_group_cd: u32) -> Result<Vec<Line>>;
}
