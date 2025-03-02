use async_trait::async_trait;
use sqlx::{MySql, MySqlConnection, Pool};
use std::sync::Arc;

use crate::domain::{
    entity::connection::Connection, error::DomainError,
    repository::connection_repository::ConnectionRepository,
};

#[derive(sqlx::FromRow, Clone, Debug)]
pub struct ConnectionRow {
    pub id: u32,
    pub station_cd1: u32,
    pub station_cd2: u32,
    pub distance: f64,
}

impl From<ConnectionRow> for Connection {
    fn from(row: ConnectionRow) -> Self {
        Self {
            id: row.id,
            station_cd1: row.station_cd1,
            station_cd2: row.station_cd2,
            distance: row.distance,
        }
    }
}

pub struct MyConnectionRepository {
    pool: Arc<Pool<MySql>>,
}

impl MyConnectionRepository {
    pub fn new(pool: Arc<Pool<MySql>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ConnectionRepository for MyConnectionRepository {
    async fn get_all(&self) -> Result<Vec<Connection>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalConnectionRepository::get_all(&mut conn).await
    }
}

pub struct InternalConnectionRepository {}

impl InternalConnectionRepository {
    async fn get_all(conn: &mut MySqlConnection) -> Result<Vec<Connection>, DomainError> {
        let query = sqlx::query_as!(ConnectionRow, "SELECT * FROM `connections`");
        let rows = query.fetch_all(conn).await?;
        let conns: Vec<Connection> = rows.into_iter().map(|row| row.into()).collect();
        Ok(conns)
    }
}
