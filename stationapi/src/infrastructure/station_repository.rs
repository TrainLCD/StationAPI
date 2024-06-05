use async_trait::async_trait;
use sqlx::{MySql, MySqlConnection, Pool};

use crate::{
    domain::{
        entity::{misc::StationIdWithDistance, station::Station},
        error::DomainError,
        repository::station_repository::StationRepository,
    },
    station_api::StopCondition,
};

#[derive(sqlx::FromRow, Clone)]
struct StationRow {
    station_cd: u32,
    station_g_cd: u32,
    station_name: String,
    station_name_k: String,
    station_name_r: Option<String>,
    station_name_zh: Option<String>,
    station_name_ko: Option<String>,
    primary_station_number: Option<String>,
    secondary_station_number: Option<String>,
    extra_station_number: Option<String>,
    three_letter_code: Option<String>,
    line_cd: u32,
    pref_cd: u32,
    post: String,
    address: String,
    lon: f64,
    lat: f64,
    open_ymd: String,
    close_ymd: String,
    e_status: u32,
    e_sort: u32,
    #[sqlx(default)]
    has_train_types: i64,
    // linesからJOIN
    pub company_cd: u32,
    pub line_name: String,
    pub line_name_k: String,
    pub line_name_h: String,
    pub line_name_r: Option<String>,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: String,
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
    average_distance: f64,
    // station_station_typesからJOIN
    #[sqlx(default)]
    pub type_cd: Option<u32>,
    #[sqlx(default)]
    pub line_group_cd: Option<u32>,
    #[sqlx(default)]
    pub pass: Option<u32>,
    // typesからJOIN
    #[sqlx(default)]
    pub type_name: Option<String>,
    #[sqlx(default)]
    pub type_name_k: Option<String>,
    #[sqlx(default)]
    pub type_name_r: Option<String>,
    #[sqlx(default)]
    pub type_name_zh: Option<String>,
    #[sqlx(default)]
    pub type_name_ko: Option<String>,
    #[sqlx(default)]
    pub color: Option<String>,
    #[sqlx(default)]
    pub direction: Option<u32>,
}

impl From<StationRow> for Station {
    fn from(row: StationRow) -> Self {
        let stop_condition = match row.pass.unwrap_or(0) {
            0 => StopCondition::All,
            1 => StopCondition::Not,
            2 => StopCondition::Partial,
            3 => StopCondition::Weekday,
            4 => StopCondition::Holiday,
            5 => StopCondition::PartialStop,
            _ => StopCondition::All,
        };

        Self {
            station_cd: row.station_cd,
            station_g_cd: row.station_g_cd,
            station_name: row.station_name,
            station_name_k: row.station_name_k,
            station_name_r: row.station_name_r,
            station_name_zh: row.station_name_zh,
            station_name_ko: row.station_name_ko,
            station_numbers: vec![],
            primary_station_number: row.primary_station_number,
            secondary_station_number: row.secondary_station_number,
            extra_station_number: row.extra_station_number,
            three_letter_code: row.three_letter_code,
            line_cd: row.line_cd,
            line: None,
            lines: vec![],
            pref_cd: row.pref_cd,
            post: row.post,
            address: row.address,
            lon: row.lon,
            lat: row.lat,
            open_ymd: row.open_ymd,
            close_ymd: row.close_ymd,
            e_status: row.e_status,
            e_sort: row.e_sort,
            stop_condition,
            distance: None,
            train_type: None,
            has_train_types: row.has_train_types != 0,
            company_cd: row.company_cd,
            line_name: row.line_name,
            line_name_k: row.line_name_k,
            line_name_h: row.line_name_h,
            line_name_r: row.line_name_r,
            line_name_zh: row.line_name_zh,
            line_name_ko: row.line_name_ko,
            line_color_c: row.line_color_c,
            line_type: row.line_type,
            line_symbol_primary: row.line_symbol_primary,
            line_symbol_secondary: row.line_symbol_secondary,
            line_symbol_extra: row.line_symbol_extra,
            line_symbol_primary_color: row.line_symbol_primary_color,
            line_symbol_secondary_color: row.line_symbol_secondary_color,
            line_symbol_extra_color: row.line_symbol_extra_color,
            line_symbol_primary_shape: row.line_symbol_primary_shape,
            line_symbol_secondary_shape: row.line_symbol_secondary_shape,
            line_symbol_extra_shape: row.line_symbol_extra_shape,
            average_distance: row.average_distance,
            type_cd: row.type_cd,
            line_group_cd: row.line_group_cd,
            pass: row.pass,
            type_name: row.type_name,
            type_name_k: row.type_name_k,
            type_name_r: row.type_name_r,
            type_name_zh: row.type_name_zh,
            type_name_ko: row.type_name_ko,
            color: row.color,
            direction: row.direction,
        }
    }
}

#[derive(sqlx::FromRow, Clone)]
struct DistanceWithIdRow {
    station_cd: u32,
    distance: f64,
    average_distance: f64,
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

const DEFAULT_COLUMN_COUNT: u32 = 1;

#[async_trait]
impl StationRepository for MyStationRepository {
    async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::find_by_id(id, &mut conn).await
    }
    async fn get_by_id_vec(&self, ids: Vec<u32>) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_id_vec(ids, &mut conn).await
    }
    async fn get_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_line_id(line_id, station_id, &mut conn).await
    }
    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn: sqlx::pool::PoolConnection<MySql> = self.pool.acquire().await?;
        InternalStationRepository::get_by_station_group_id(station_group_id, &mut conn).await
    }
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: Vec<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn: sqlx::pool::PoolConnection<MySql> = self.pool.acquire().await?;
        InternalStationRepository::get_by_station_group_id_vec(station_group_id_vec, &mut conn)
            .await
    }

    // ほぼ確実にキャッシュがヒットしないと思うのでキャッシュを使わない
    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_coordinates(latitude, longitude, limit, &mut conn).await
    }

    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_name(station_name, limit, &mut conn).await
    }

    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_line_group_id(line_group_id, &mut conn).await
    }
    async fn get_station_id_and_distance_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        line_id: Option<u32>,
    ) -> Result<StationIdWithDistance, DomainError> {
        let mut conn = self.pool.acquire().await?;
        match line_id {
            Some(line_id) => {
                InternalStationRepository::get_station_id_and_distance_by_coordinates_and_line_id(
                    latitude, longitude, line_id, &mut conn,
                )
                .await
            }
            None => {
                InternalStationRepository::get_station_id_and_distance_by_coordinates(
                    latitude, longitude, &mut conn,
                )
                .await
            }
        }
    }
}

struct InternalStationRepository {}

impl InternalStationRepository {
    async fn find_by_id(
        id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Option<Station>, DomainError> {
        let rows: Option<StationRow> = sqlx::query_as!(
            StationRow,
            "SELECT DISTINCT s.*,
            l.company_cd,
            l.line_type,
            l.line_symbol_primary,
            l.line_symbol_secondary,
            l.line_symbol_extra,
            l.line_symbol_primary_color,
            l.line_symbol_secondary_color,
            l.line_symbol_extra_color,
            l.line_symbol_primary_shape,
            l.line_symbol_secondary_shape,
            l.line_symbol_extra_shape,
            l.average_distance,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            IFNULL(s.station_cd = sst.station_cd, 0) AS has_train_types,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction
          FROM `stations` AS s
            JOIN `lines` AS l ON l.line_cd = s.line_cd
            AND l.e_status = 0
            LEFT JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd
            LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT JOIN `aliases` AS a ON la.alias_cd = a.id
          WHERE s.station_cd = ?
            AND s.e_status = 0
          ORDER BY s.e_sort,
            s.station_cd",
            id,
        )
        .fetch_optional(conn)
        .await?;

        let station: Option<Station> = rows.map(|row| row.into());
        let Some(station) = station else {
            return Ok(None);
        };

        Ok(Some(station))
    }

    async fn get_by_id_vec(
        ids: Vec<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(ids.len() - 1));

        let query_str = format!(
            "SELECT
              l.*,
              s.*,
              COALESCE(a.line_name, l.line_name) AS line_name,
              COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
              COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
              COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
              COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
              COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
              COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
              IFNULL(s.station_cd = sst.station_cd, 0) AS has_train_types
            FROM `stations` AS s
            JOIN `lines` AS l ON l.line_cd = s.line_cd AND l.e_status = 0
            LEFT JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd
            LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT JOIN `aliases` AS a ON la.alias_cd = a.id
            WHERE
              s.station_cd IN ( {} )
              AND s.line_cd = l.line_cd
              AND s.e_status = 0
            ORDER BY FIELD(s.station_cd, {})",
            params, params
        );

        let mut query = sqlx::query_as::<_, StationRow>(&query_str);
        for id in ids.clone() {
            query = query.bind(id);
        }
        for id in ids {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let lines: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }

    async fn get_by_line_id(
        line_id: u32,
        station_id: Option<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let station_row: Vec<StationRow> = sqlx::query_as!(
            StationRow,
            "SELECT s.*,
            l.company_cd,
            l.line_type,
            l.line_symbol_primary,
            l.line_symbol_secondary,
            l.line_symbol_extra,
            l.line_symbol_primary_color,
            l.line_symbol_secondary_color,
            l.line_symbol_extra_color,
            l.line_symbol_primary_shape,
            l.line_symbol_secondary_shape,
            l.line_symbol_extra_shape,
            l.average_distance,
            t.type_cd,
            t.color,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.direction,
            sst.pass,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            sst.line_group_cd,
            IFNULL(s.station_cd = sst.station_cd, 0) AS has_train_types
            FROM `stations` AS s
            JOIN `lines` AS l ON l.line_cd = ?
            AND l.e_status = 0
            LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT JOIN `aliases` AS a ON la.alias_cd = a.id
            LEFT JOIN `station_station_types` AS sst ON sst.line_group_cd = (
              SELECT sst.line_group_cd
              FROM `station_station_types` AS sst
                LEFT JOIN `types` AS t ON sst.type_cd = t.type_cd
              WHERE CASE
                  WHEN ? IS NOT NULL THEN sst.station_cd = ?
                END
                AND CASE
                  WHEN t.top_priority = 1 THEN sst.type_cd = t.type_cd
                  ELSE t.kind IN (0, 1)
                END
              ORDER BY sst.id
              LIMIT 1
            )
            LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd
          WHERE sst.station_cd IS NULL
            AND s.line_cd = l.line_cd
            AND s.e_status = 0
          UNION
          DISTINCT (
            SELECT s.*,
              l.company_cd,
              l.line_type,
              l.line_symbol_primary,
              l.line_symbol_secondary,
              l.line_symbol_extra,
              l.line_symbol_primary_color,
              l.line_symbol_secondary_color,
              l.line_symbol_extra_color,
              l.line_symbol_primary_shape,
              l.line_symbol_secondary_shape,
              l.line_symbol_extra_shape,
              l.average_distance,
              t.type_cd,
              t.color,
              t.type_name,
              t.type_name_k,
              t.type_name_r,
              t.type_name_zh,
              t.type_name_ko,
              t.direction,
              sst.pass,
              COALESCE(a.line_name, l.line_name) AS line_name,
              COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
              COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
              COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
              COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
              COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
              COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
              sst.line_group_cd,
              IFNULL(s.station_cd = sst.station_cd, 0) AS has_train_types
            FROM `stations` AS s
              JOIN `lines` AS l ON l.line_cd = s.line_cd
              AND l.e_status = 0
              LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
              LEFT JOIN `aliases` AS a ON la.alias_cd = a.id
              LEFT JOIN `station_station_types` AS sst ON sst.line_group_cd = (
                SELECT sst.line_group_cd
                FROM `station_station_types` AS sst
                  LEFT JOIN `types` AS t ON sst.type_cd = t.type_cd
                WHERE CASE
                    WHEN ? IS NOT NULL THEN sst.station_cd = ?
                  END
                  AND CASE
                    WHEN t.top_priority = 1 THEN sst.type_cd = t.type_cd
                    ELSE t.kind IN (0, 1)
                  END
                ORDER BY sst.id
                LIMIT 1
              )
              LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd
            WHERE sst.station_cd IS NOT NULL
              AND s.station_cd = sst.station_cd
              AND s.line_cd = l.line_cd
              AND s.e_status = 0
          )",
            line_id,
            station_id,
            station_id,
            station_id,
            station_id
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = station_row.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_station_group_id(
        group_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows: Vec<StationRow> = sqlx::query_as!(
            StationRow,
            "SELECT s.*,
            l.company_cd,
            l.line_type,
            l.line_symbol_primary,
            l.line_symbol_secondary,
            l.line_symbol_extra,
            l.line_symbol_primary_color,
            l.line_symbol_secondary_color,
            l.line_symbol_extra_color,
            l.line_symbol_primary_shape,
            l.line_symbol_secondary_shape,
            l.line_symbol_extra_shape,
            l.average_distance,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            IFNULL(s.station_cd = sst.station_cd, 0) AS has_train_types,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction
          FROM
            `stations` AS s
            JOIN `lines` AS l ON l.line_cd = s.line_cd AND l.e_status = 0
            LEFT JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd
            LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT JOIN `aliases` AS a ON a.id = la.alias_cd
          WHERE
            s.station_g_cd = ?
            AND s.line_cd = l.line_cd
            AND s.e_status = 0",
            group_id
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_station_group_id_vec(
        group_id_vec: Vec<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        if group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(group_id_vec.len() - 1));
        let query_str = format!(
            "SELECT
            l.*,
            s.*,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            IFNULL(s.station_cd = sst.station_cd, 0) AS has_train_types
          FROM
            `stations` AS s
            JOIN `lines` AS l ON l.line_cd = s.line_cd AND l.e_status = 0
            LEFT JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd  
            LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT JOIN `aliases` AS a ON a.id = la.alias_cd
          WHERE
            s.station_g_cd IN ( {} )
            AND s.line_cd = l.line_cd
            AND s.e_status = 0",
            params
        );

        let mut query = sqlx::query_as::<_, StationRow>(&query_str);
        for id in group_id_vec {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let lines: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }

    async fn get_by_coordinates(
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows = sqlx::query_as::<_, StationRow>(
            "SELECT 
                l.*, 
                s.*, 
                COALESCE(a.line_name, l.line_name) AS line_name, 
                COALESCE(a.line_name_k, l.line_name_k) AS line_name_k, 
                COALESCE(a.line_name_h, l.line_name_h) AS line_name_h, 
                COALESCE(a.line_name_r, l.line_name_r) AS line_name_r, 
                COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh, 
                COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko, 
                COALESCE(a.line_color_c, l.line_color_c) AS line_color_c, 
                (
                  6371 * acos(
                    cos(
                      radians(s.lat)
                    ) * cos(
                      radians(?)
                    ) * cos(
                      radians(?) - radians(s.lon)
                    ) + sin(
                      radians(s.lat)
                    ) * sin(
                      radians(?)
                    )
                  )
                ) AS distance, 
                IFNULL(s.station_cd = sst.station_cd, 0) AS has_train_types
              FROM `stations` AS s
              JOIN `lines` AS l ON l.line_cd IN (
                SELECT _s.line_cd
                FROM `stations` AS _s
                WHERE _s.station_g_cd = s.station_g_cd
              )
              LEFT JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd
              LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd  
              LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd 
              LEFT JOIN `aliases` AS a ON a.id = la.alias_cd 
              WHERE
                s.e_status = 0
              ORDER BY 
                distance 
              LIMIT
                ?",
        )
        .bind(latitude)
        .bind(longitude)
        .bind(latitude)
        .bind(limit.unwrap_or(DEFAULT_COLUMN_COUNT))
        .fetch_all(conn)
        .await?;

        let stations = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_station_id_and_distance_by_coordinates_and_line_id(
        latitude: f64,
        longitude: f64,
        line_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<StationIdWithDistance, DomainError> {
        let row = sqlx::query_as::<_, DistanceWithIdRow>(
            "SELECT
            s.station_cd,
            s.station_g_cd, 
            l.average_distance,
            (
              6371 * acos(
                cos(
                  radians(s.lat)
                ) * cos(
                  radians(?)
                ) * cos(
                  radians(?) - radians(s.lon)
                ) + sin(
                  radians(s.lat)
                ) * sin(
                  radians(?)
                )
              )
            ) AS distance
          FROM `stations` AS s
          JOIN `lines` AS l ON l.line_cd = ?
          WHERE
            s.line_cd = ?
            AND s.e_status = 0
          ORDER BY 
            distance
          LIMIT 
            1",
        )
        .bind(latitude)
        .bind(longitude)
        .bind(latitude)
        .bind(line_id)
        .bind(line_id)
        .fetch_one(conn)
        .await?;
        let id_with_distance = StationIdWithDistance {
            station_id: row.station_cd,
            distance: row.distance,
            average_distance: row.average_distance,
        };

        Ok(id_with_distance)
    }

    async fn get_station_id_and_distance_by_coordinates(
        latitude: f64,
        longitude: f64,
        conn: &mut MySqlConnection,
    ) -> Result<StationIdWithDistance, DomainError> {
        let row = sqlx::query_as::<_, DistanceWithIdRow>(
            "SELECT
          s.station_cd,
          s.station_g_cd,
          l.average_distance,
          (
            6371 * acos(
              cos(
                radians(s.lat)
              ) * cos(
                radians(?)
              ) * cos(
                radians(?) - radians(s.lon)
              ) + sin(
                radians(s.lat)
              ) * sin(
                radians(?)
              )
            )
          ) AS distance
        FROM `stations` AS s
        JOIN `lines` AS l ON l.line_cd = s.line_cd
        WHERE
          s.e_status = 0
        ORDER BY 
          distance
        LIMIT 
          1",
        )
        .bind(latitude)
        .bind(longitude)
        .bind(latitude)
        .fetch_one(conn)
        .await?;
        let id_with_distance = StationIdWithDistance {
            station_id: row.station_cd,
            distance: row.distance,
            average_distance: row.average_distance,
        };

        Ok(id_with_distance)
    }

    async fn get_by_name(
        station_name: String,
        limit: Option<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let station_name = format!("%{}%", station_name);

        let rows = sqlx::query_as::<_, StationRow>(
            "SELECT DISTINCT
                    l.*,
                    s.*,
                    COALESCE(a.line_name, l.line_name) AS line_name,
                    COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
                    COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
                    COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
                    COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
                    COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
                    COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
                    IFNULL(s.station_cd = sst.station_cd, 0) AS has_train_types
                  FROM `stations` AS s
                  JOIN `lines` AS l ON l.line_cd = s.line_cd AND l.e_status = 0
                  LEFT JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd
                  LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd
                  LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
                  LEFT JOIN `aliases` AS a ON la.alias_cd = a.id
                  WHERE
                    (
                      station_name LIKE ?
                      OR station_name_r LIKE ?
                      OR station_name_k LIKE ?
                      OR station_name_zh LIKE ?
                      OR station_name_ko LIKE ?
                    )
                    AND s.e_status = 0
                  LIMIT
                    ?",
        )
        .bind(&station_name)
        .bind(&station_name)
        .bind(&station_name)
        .bind(&station_name)
        .bind(&station_name)
        .bind(limit.unwrap_or(DEFAULT_COLUMN_COUNT))
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_line_group_id(
        line_group_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows: Vec<StationRow> = sqlx::query_as!(
            StationRow,
            "SELECT DISTINCT s.*,
            l.company_cd,
            l.line_type,
            l.line_symbol_primary,
            l.line_symbol_secondary,
            l.line_symbol_extra,
            l.line_symbol_primary_color,
            l.line_symbol_secondary_color,
            l.line_symbol_extra_color,
            l.line_symbol_primary_shape,
            l.line_symbol_secondary_shape,
            l.line_symbol_extra_shape,
            l.average_distance,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            IFNULL(s.station_cd = sst.station_cd, 0) AS has_train_types,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction
          FROM `stations` AS s
          JOIN `lines` AS l ON l.line_cd = s.line_cd AND l.e_status = 0
          LEFT JOIN `station_station_types` AS sst ON sst.line_group_cd = ?
          LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd
          LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
          LEFT JOIN `aliases` AS a ON la.alias_cd = a.id
          WHERE
            s.line_cd = l.line_cd
            AND s.station_cd = sst.station_cd
            AND s.e_status = 0",
            line_group_id
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }
}
