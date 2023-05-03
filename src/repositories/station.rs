use async_trait::async_trait;
use sqlx::{MySql, Pool};

use crate::entities::station::Station;

#[async_trait]
pub trait StationRepository {
    async fn find_one(&self, id: i32) -> Option<Station>;
}

pub struct StationRepositoryImplOnMySQL<'a> {
    pub pool: &'a Pool<MySql>,
}

#[async_trait]
impl StationRepository for StationRepositoryImplOnMySQL<'_> {
    async fn find_one(&self, id: i32) -> Option<Station> {
        sqlx::query_as!(Station, "SELECT * FROM stations WHERE station_cd = ?", id)
            .fetch_one(self.pool)
            .await
            .ok()
    }
}
