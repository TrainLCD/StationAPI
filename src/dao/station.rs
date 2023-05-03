use crate::entities::station::Station;
use async_trait::async_trait;
use sqlx::{MySql, Pool};

#[async_trait]
pub trait StationDao<'a> {
    async fn find_one(&self, id: i64) -> Result<Station, sqlx::Error>;
}

pub struct StationMySQLDao<'a>(pub &'a Pool<MySql>);

#[async_trait]
impl<'a> StationDao<'a> for StationMySQLDao<'a> {
    async fn find_one(&self, id: i64) -> Result<Station, sqlx::Error> {
        sqlx::query_as!(Station, "SELECT * FROM stations WHERE station_cd = ?", id)
            .fetch_one(self.0)
            .await
    }
}
