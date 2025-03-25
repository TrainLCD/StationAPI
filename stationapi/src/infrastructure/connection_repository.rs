use async_trait::async_trait;
use sqlx::SqliteConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::{
    entity::connection::Connection, error::DomainError,
    repository::connection_repository::ConnectionRepository,
};

#[derive(sqlx::FromRow, Clone, Debug)]
pub struct ConnectionRow {
    pub id: i64,
    pub station_cd1: i64,
    pub station_cd2: i64,
    pub distance: f64,
}

impl From<ConnectionRow> for Connection {
    fn from(row: ConnectionRow) -> Self {
        Self {
            id: row.id as u32,
            station_cd1: row.station_cd1 as u32,
            station_cd2: row.station_cd2 as u32,
            distance: row.distance,
        }
    }
}

pub struct MyConnectionRepository {
    conn: Arc<Mutex<SqliteConnection>>,
}

impl MyConnectionRepository {
    pub fn new(conn: Arc<Mutex<SqliteConnection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl ConnectionRepository for MyConnectionRepository {
    async fn get_all(&self) -> Result<Vec<Connection>, DomainError> {
        let mut conn = self.conn.lock().await;
        InternalConnectionRepository::get_all(&mut conn).await
    }
}

pub struct InternalConnectionRepository {}

impl InternalConnectionRepository {
    async fn get_all(conn: &mut SqliteConnection) -> Result<Vec<Connection>, DomainError> {
        let query = sqlx::query_as!(
            ConnectionRow,
            "SELECT id, station_cd1, station_cd2, distance FROM `connections`"
        );
        let rows = query.fetch_all(conn).await?;
        let conns: Vec<Connection> = rows.into_iter().map(|row| row.into()).collect();
        Ok(conns)
    }
}
