use async_trait::async_trait;
use sqlx::{MySql, Pool};

use crate::entities::station::Station;

#[async_trait]
pub trait StationRepository {
    async fn find_one(&self, id: u32) -> Option<Station>;
    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<i32>,
    ) -> Option<Vec<Station>>;
}

pub struct StationRepositoryImplOnMySQL<'a> {
    pub pool: &'a Pool<MySql>,
}

#[async_trait]
impl StationRepository for StationRepositoryImplOnMySQL<'_> {
    async fn find_one(&self, id: u32) -> Option<Station> {
        sqlx::query_as::<_, Station>("SELECT * FROM stations WHERE station_cd = ?")
            .bind(id)
            .fetch_one(self.pool)
            .await
            .ok()
    }

    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<i32>,
    ) -> Option<Vec<Station>> {
        sqlx::query_as!(
            Station,
            "SELECT *,
        (
          6371 * acos(
          cos(radians(?))
          * cos(radians(lat))
          * cos(radians(lon) - radians(?))
          + sin(radians(?))
          * sin(radians(lat))
          )
        ) AS distance
        FROM
        stations as s1
        WHERE
        e_status = 0
        AND
        station_cd = (
          SELECT station_cd 
          FROM stations as s2
          WHERE s1.station_g_cd = s2.station_g_cd
          LIMIT 1
        )
        ORDER BY
        distance
        LIMIT ?",
            latitude,
            longitude,
            latitude,
            limit.unwrap_or(1)
        )
        .fetch_all(self.pool)
        .await
        .ok()
    }
}
