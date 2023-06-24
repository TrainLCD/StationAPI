use async_trait::async_trait;
use bigdecimal::{BigDecimal, ToPrimitive};
use sqlx::{MySql, MySqlConnection, Pool};

use crate::domain::{
    entity::station::Station, error::DomainError, repository::station_repository::StationRepository,
};

#[derive(sqlx::FromRow, Clone)]
pub struct CompanyRow {
    pub company_cd: u32,
    pub rr_cd: u32,
    pub company_name: String,
    pub company_name_k: String,
    pub company_name_h: String,
    pub company_name_r: String,
    pub company_name_en: String,
    pub company_name_full_en: String,
    pub company_url: String,
    pub company_type: i32,
    pub e_status: u32,
    pub e_sort: u32,
}

#[derive(sqlx::FromRow, Clone)]
pub struct LineRow {
    pub line_cd: u32,
    pub company_cd: u32,
    pub line_name: String,
    pub line_name_k: String,
    pub line_name_h: String,
    pub line_name_r: String,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: String,
    pub line_color_t: String,
    pub line_type: u32,
    pub line_symbol_primary: Option<String>,
    pub line_symbol_secondary: Option<String>,
    pub line_symbol_extra: Option<String>,
    pub line_symbol_primary_color: Option<String>,
    pub line_symbol_secondary_color: Option<String>,
    pub line_symbol_extra_color: Option<String>,
    pub line_symbol_primary_shape: Option<String>,
    pub line_symbol_secondary_shape: Option<String>,
    pub line_symbol_extra_shape: Option<String>,
    pub lon: f64,
    pub lat: f64,
    pub zoom: u32,
    pub e_status: u32,
    pub e_sort: u32,
}

#[derive(sqlx::FromRow)]
pub struct StationRow {
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

impl From<StationRow> for Station {
    fn from(row: StationRow) -> Self {
        Self {
            station_cd: row.station_cd,
            station_g_cd: row.station_g_cd,
            station_name: row.station_name,
            station_name_k: row.station_name_k,
            station_name_r: row.station_name_r,
            station_name_zh: row.station_name_zh,
            station_name_ko: row.station_name_ko,
            station_numbers: vec![],
            three_letter_code: row.three_letter_code,
            line: None,
            lines: vec![],
            pref_cd: row.pref_cd,
            post: row.post,
            address: row.address,
            lon: row
                .lat
                .to_f64()
                .expect("Failed to convert BigDecimal to f64"),
            lat: row
                .lon
                .to_f64()
                .expect("Failed to convert BigDecimal to f64"),
            open_ymd: row.open_ymd,
            close_ymd: row.close_ymd,
            e_status: row.e_status,
            e_sort: row.e_sort,
            distance: None,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct StationWithDistanceRow {
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

impl From<StationWithDistanceRow> for Station {
    fn from(row: StationWithDistanceRow) -> Self {
        Self {
            station_cd: row.station_cd,
            station_g_cd: row.station_g_cd,
            station_name: row.station_name,
            station_name_k: row.station_name_k,
            station_name_r: row.station_name_r,
            station_name_zh: row.station_name_zh,
            station_name_ko: row.station_name_ko,
            station_numbers: vec![],
            three_letter_code: row.three_letter_code,
            line: None,
            lines: vec![],
            pref_cd: row.pref_cd,
            post: row.post,
            address: row.address,
            lon: row
                .lat
                .to_f64()
                .expect("Failed to convert BigDecimal to f64"),
            lat: row
                .lon
                .to_f64()
                .expect("Failed to convert BigDecimal to f64"),
            open_ymd: row.open_ymd,
            close_ymd: row.close_ymd,
            e_status: row.e_status,
            e_sort: row.e_sort,
            distance: row.distance,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MyStationRepository {
    pool: Pool<MySql>,
}

impl MyStationRepository {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
}

const MAXIMUM_COLUMN_COUNT: u32 = 100;

#[async_trait]
impl StationRepository for MyStationRepository {
    async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::find_by_id(id, &mut conn).await
    }
    async fn get_by_line_id(&self, line_id: u32) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_line_id(line_id, &mut conn).await
    }
    async fn get_by_stations_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn: sqlx::pool::PoolConnection<MySql> = self.pool.acquire().await?;
        InternalStationRepository::get_by_stations_group_id(station_group_id, &mut conn).await
    }

    async fn get_stations_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_stations_by_coordinates(
            latitude, longitude, limit, &mut conn,
        )
        .await
    }

    async fn get_stations_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
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
            ORDER BY e_sort, station_cd
            LIMIT {}
        ",
            station_name,
            station_name,
            station_name,
            station_name,
            station_name,
            limit.unwrap_or(MAXIMUM_COLUMN_COUNT)
        );
        let mut conn = self.pool.acquire().await?;
        let result = sqlx::query_as::<_, StationRow>(&query_str)
            .fetch_all(&mut conn)
            .await;
        match result {
            Ok(rows) => Ok(rows.into_iter().map(|row| row.into()).collect()),
            Err(err) => Err(err.into()),
        }
    }
}

pub struct InternalStationRepository {}

impl InternalStationRepository {
    async fn find_by_id(
        id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Option<Station>, DomainError> {
        let row: Option<StationRow> =
            sqlx::query_as("SELECT * FROM stations WHERE station_cd = ? AND e_status = 0")
                .bind(id)
                .fetch_optional(conn)
                .await?;

        let station = row.map(|row| {
            Station::new(
                row.station_cd,
                row.station_g_cd,
                row.station_name,
                row.station_name_k,
                row.station_name_r,
                row.station_name_zh,
                row.station_name_ko,
                vec![],
                row.three_letter_code,
                None,
                vec![],
                row.pref_cd,
                row.post,
                row.address,
                row.lon,
                row.lat,
                row.open_ymd,
                row.close_ymd,
                row.e_status,
                row.e_sort,
                None,
            )
        });

        Ok(station)
    }
    async fn get_by_line_id(
        line_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let station_row: Vec<StationRow> =
            sqlx::query_as("SELECT * FROM stations WHERE line_cd = ? AND e_status = 0")
                .bind(line_id)
                .fetch_all(conn)
                .await?;

        let stations = station_row
            .iter()
            .map(|row| {
                Station::new(
                    row.station_cd,
                    row.station_g_cd,
                    row.station_name.clone(),
                    row.station_name_k.clone(),
                    row.station_name_r.clone(),
                    row.station_name_zh.clone(),
                    row.station_name_ko.clone(),
                    vec![],
                    row.three_letter_code.clone(),
                    None,
                    vec![],
                    row.pref_cd,
                    row.post.clone(),
                    row.address.clone(),
                    row.lon.clone(),
                    row.lat.clone(),
                    row.open_ymd.clone(),
                    row.close_ymd.clone(),
                    row.e_status,
                    row.e_sort,
                    None,
                )
            })
            .collect();

        Ok(stations)
    }

    async fn get_by_stations_group_id(
        group_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows: Vec<StationRow> =
            sqlx::query_as("SELECT * FROM stations WHERE station_g_cd = ? AND e_status = 0")
                .bind(group_id)
                .fetch_all(conn)
                .await?;

        let stations = rows
            .iter()
            .map(|row| {
                Station::new(
                    row.station_cd,
                    row.station_g_cd,
                    row.station_name.clone(),
                    row.station_name_k.clone(),
                    row.station_name_r.clone(),
                    row.station_name_zh.clone(),
                    row.station_name_ko.clone(),
                    vec![],
                    row.three_letter_code.clone(),
                    None,
                    vec![],
                    row.pref_cd,
                    row.post.clone(),
                    row.address.clone(),
                    row.lon.clone(),
                    row.lat.clone(),
                    row.open_ymd.clone(),
                    row.close_ymd.clone(),
                    row.e_status,
                    row.e_sort,
                    None,
                )
            })
            .collect();

        Ok(stations)
    }

    async fn get_stations_by_coordinates(
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let query_str = "SELECT *,
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
        LIMIT ?";

        let rows = sqlx::query_as::<_, StationRow>(query_str)
            .bind(latitude)
            .bind(longitude)
            .bind(latitude)
            .bind(limit.unwrap_or(100)) // TODO: 100 is a magic number
            .fetch_all(conn)
            .await?;

        let stations = rows
            .iter()
            .map(|row| {
                Station::new(
                    row.station_cd,
                    row.station_g_cd,
                    row.station_name.clone(),
                    row.station_name_k.clone(),
                    row.station_name_r.clone(),
                    row.station_name_zh.clone(),
                    row.station_name_ko.clone(),
                    vec![],
                    row.three_letter_code.clone(),
                    None,
                    vec![],
                    row.pref_cd,
                    row.post.clone(),
                    row.address.clone(),
                    row.lon.clone(),
                    row.lat.clone(),
                    row.open_ymd.clone(),
                    row.close_ymd.clone(),
                    row.e_status,
                    row.e_sort,
                    None,
                )
            })
            .collect();

        Ok(stations)
    }
}