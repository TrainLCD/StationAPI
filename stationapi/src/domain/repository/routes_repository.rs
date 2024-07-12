use async_trait::async_trait;

use crate::{domain::error::DomainError, infrastructure::routes_repository::RouteRow};

#[async_trait]
pub trait RoutesRepository: Send + Sync + 'static {
    async fn get_routes(
        &self,
        from_station_id: u32,
        to_station_id: u32,
    ) -> Result<Vec<RouteRow>, DomainError>;
}
