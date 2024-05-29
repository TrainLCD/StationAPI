use async_trait::async_trait;
use sqlx::{MySql, MySqlConnection, Pool};

use crate::{
    domain::{error::DomainError, repository::connection_repository::ConnectionRepository},
    station_api::Route,
};

#[derive(Debug, Clone)]
pub struct MyConnectionRepository {
    pool: Pool<MySql>,
}

impl MyConnectionRepository {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ConnectionRepository for MyConnectionRepository {
    async fn get_routes(
        &self,
        from_station_id: u32,
        to_station_id: u32,
    ) -> Result<Vec<Route>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalConnectionRepository::get_routes(from_station_id, to_station_id, &mut conn).await
    }
}

pub struct InternalConnectionRepository {}

impl InternalConnectionRepository {
    async fn get_routes(
        from_station_id: u32,
        to_station_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Route>, DomainError> {
        Ok(vec![])
    }
}
