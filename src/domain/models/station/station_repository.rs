use anyhow::Result;
use mockall::automock;

use super::station_model::Station;

#[async_trait::async_trait]
#[automock]
pub trait StationRepository {
    async fn find_by_id(&self, id: u32) -> Result<Station>;
    async fn get_by_group_id(&self, group_id: u32) -> Result<Vec<Station>>;
    async fn get_by_line_id(&self, line_id: u32) -> Result<Vec<Station>>;
    async fn get_by_name(&self, name: &str, limit: &Option<u32>) -> Result<Vec<Station>>;
    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: &Option<u32>,
    ) -> Result<Vec<Station>>;
}
