use async_trait::async_trait;

use crate::{domain::error::DomainError, station_api::Route};

#[async_trait]
pub trait ConnectionRepository: Send + Sync + 'static {
    async fn get_routes(
        &self,
        from_station_id: u32,
        to_station_id: u32,
    ) -> Result<Vec<Route>, DomainError>;
}
