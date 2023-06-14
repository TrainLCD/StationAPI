use anyhow::Result;
use bigdecimal::{BigDecimal, ToPrimitive};
use sqlx::{MySql, Pool};

use crate::domain::models::station::{
    station_model::Station, station_repository::StationRepository,
};

#[derive(sqlx::FromRow, Clone, Debug)]
pub struct StationEntity {
    pub station_cd: u32,
    pub station_g_cd: u32,
    pub station_name: String,
    pub station_name_k: String,
    pub station_name_r: String,
    pub station_name_zh: String,
    pub station_name_ko: String,
    pub primary_station_number: Option<String>,
    pub secondary_station_number: Option<String>,
    pub extra_station_number: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: u32,
    pub pref_cd: u32,
    pub post: String,
    pub address: String,
    pub lon: BigDecimal,
    pub lat: BigDecimal,
    pub open_ymd: String,
    pub close_ymd: String,
    pub e_status: u32,
    pub e_sort: u32,
}

#[derive(sqlx::FromRow, Clone)]
pub struct StationWithDistanceEntity {
    pub station_cd: u32,
    pub station_g_cd: u32,
    pub station_name: String,
    pub station_name_k: String,
    pub station_name_r: String,
    pub station_name_zh: String,
    pub station_name_ko: String,
    pub primary_station_number: Option<String>,
    pub secondary_station_number: Option<String>,
    pub extra_station_number: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: u32,
    pub pref_cd: u32,
    pub post: String,
    pub address: String,
    pub lon: BigDecimal,
    pub lat: BigDecimal,
    pub open_ymd: String,
    pub close_ymd: String,
    pub e_status: u32,
    pub e_sort: u32,
    pub distance: Option<f64>,
}

impl From<StationEntity> for Station {
    fn from(entity: StationEntity) -> Station {
        Station {
            station_cd: entity.station_cd,
            station_g_cd: entity.station_g_cd,
            station_name: entity.station_name,
            station_name_k: entity.station_name_k,
            station_name_r: entity.station_name_r,
            station_name_zh: entity.station_name_zh,
            station_name_ko: entity.station_name_ko,
            primary_station_number: entity.primary_station_number,
            secondary_station_number: entity.secondary_station_number,
            extra_station_number: entity.extra_station_number,
            three_letter_code: entity.three_letter_code,
            line_cd: entity.line_cd,
            pref_cd: entity.pref_cd,
            post: entity.post,
            address: entity.address,
            lon: entity.lon.to_f64().unwrap_or(0.0),
            lat: entity.lat.to_f64().unwrap_or(0.0),
            open_ymd: entity.open_ymd,
            close_ymd: entity.close_ymd,
            e_status: entity.e_status,
            e_sort: entity.e_sort,
            distance: Some(0.0),
        }
    }
}

impl From<StationWithDistanceEntity> for Station {
    fn from(entity: StationWithDistanceEntity) -> Station {
        Station {
            station_cd: entity.station_cd,
            station_g_cd: entity.station_g_cd,
            station_name: entity.station_name,
            station_name_k: entity.station_name_k,
            station_name_r: entity.station_name_r,
            station_name_zh: entity.station_name_zh,
            station_name_ko: entity.station_name_ko,
            primary_station_number: entity.primary_station_number,
            secondary_station_number: entity.secondary_station_number,
            extra_station_number: entity.extra_station_number,
            three_letter_code: entity.three_letter_code,
            line_cd: entity.line_cd,
            pref_cd: entity.pref_cd,
            post: entity.post,
            address: entity.address,
            lon: entity.lon.to_f64().unwrap_or(0.0),
            lat: entity.lat.to_f64().unwrap_or(0.0),
            open_ymd: entity.open_ymd,
            close_ymd: entity.close_ymd,
            e_status: entity.e_status,
            e_sort: entity.e_sort,
            distance: entity.distance,
        }
    }
}

pub struct StationRepositoryImpl {
    pub pool: Box<Pool<MySql>>,
}

#[async_trait::async_trait]
impl StationRepository for StationRepositoryImpl {
    async fn find_by_id(&self, id: u32) -> Result<Station> {
        let result = sqlx::query_as!(
            StationEntity,
            "SELECT * FROM stations WHERE station_cd = ? LIMIT 1",
            id
        )
        .fetch_one(self.pool.as_ref())
        .await;
        match result.map(|entity| entity.into()) {
            Ok(station) => Ok(station),
            Err(err) => Err(err.into()),
        }
    }

    async fn find_by_group_id(&self, group_id: u32) -> Result<Vec<Station>> {
        let result = sqlx::query_as!(
            StationEntity,
            "SELECT * FROM stations WHERE station_g_cd = ?",
            group_id
        )
        .fetch_all(self.pool.as_ref())
        .await;
        match result {
            Ok(stations) => Ok(stations.into_iter().map(|station| station.into()).collect()),
            Err(err) => Err(err.into()),
        }
    }

    async fn find_by_name(&self, name: &str) -> Result<Vec<Station>> {
        let query_str = format!(
            "SELECT * FROM stations
            WHERE (
                station_name LIKE '%{}%'
                OR station_name_r LIKE '%{}%'
                OR station_name_k LIKE '%{}%'
                OR station_name_zh LIKE '%{}%'
                OR station_name_ko LIKE '%{}%'
        )
            AND e_status = 0
            ORDER BY e_sort, station_cd",
            name, name, name, name, name
        );
        let result = sqlx::query_as::<_, StationEntity>(&query_str)
            .fetch_all(self.pool.as_ref())
            .await;
        match result {
            Ok(stations) => Ok(stations.into_iter().map(|station| station.into()).collect()),
            Err(err) => Err(err.into()),
        }
    }

    async fn find_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<i32>,
    ) -> Result<Vec<Station>> {
        let result = sqlx::query_as!(
            StationWithDistanceEntity,
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
            limit.or(Some(1))
        )
        .fetch_all(self.pool.as_ref())
        .await;
        match result {
            Ok(stations) => Ok(stations.into_iter().map(|station| station.into()).collect()),
            Err(err) => Err(err.into()),
        }
    }

    async fn find_by_line_id(&self, line_id: u32) -> Result<Vec<Station>> {
        let result = sqlx::query_as!(
            StationEntity,
            "SELECT *
            FROM stations
            WHERE line_cd = ?
            AND e_status = 0
            ORDER BY e_sort, station_cd",
            line_id
        )
        .fetch_all(self.pool.as_ref())
        .await;
        match result {
            Ok(stations) => Ok(stations.into_iter().map(|station| station.into()).collect()),
            Err(err) => Err(err.into()),
        }
    }
}
