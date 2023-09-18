use async_trait::async_trait;
use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use sqlx::{MySql, MySqlConnection, Pool};

use crate::{
    domain::{
        entity::station::Station, error::DomainError,
        repository::station_repository::StationRepository,
    },
    pb::StopCondition,
};

#[derive(sqlx::FromRow, Clone)]
struct StationRow {
    station_cd: u32,
    station_g_cd: u32,
    station_name: String,
    station_name_k: String,
    station_name_r: String,
    station_name_zh: String,
    station_name_ko: String,
    primary_station_number: Option<String>,
    secondary_station_number: Option<String>,
    extra_station_number: Option<String>,
    three_letter_code: Option<String>,
    line_cd: u32,
    pref_cd: u32,
    post: String,
    address: String,
    lon: BigDecimal,
    lat: BigDecimal,
    open_ymd: String,
    close_ymd: String,
    e_status: u32,
    e_sort: u32,
    #[sqlx(default)]
    pass: i64,
    #[sqlx(default)]
    station_types_count: i64,
    // linesからJOIN
    pub company_cd: u32,
    pub line_name: String,
    pub line_name_k: String,
    pub line_name_h: String,
    pub line_name_r: String,
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
}

impl From<StationRow> for Station {
    fn from(row: StationRow) -> Self {
        let stop_condition = match row.pass {
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
            lon: row
                .lon
                .to_f64()
                .expect("Failed to convert BigDecimal to f64"),
            lat: row
                .lat
                .to_f64()
                .expect("Failed to convert BigDecimal to f64"),
            open_ymd: row.open_ymd,
            close_ymd: row.close_ymd,
            e_status: row.e_status,
            e_sort: row.e_sort,
            stop_condition,
            distance: None,
            station_types_count: row.station_types_count,
            train_type: None,
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

const DEFAULT_COLUMN_COUNT: u32 = 1;

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

    async fn get_by_station_group_and_line_id(
        &self,
        station_group_id: u32,
        line_id: u32,
    ) -> Result<Option<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_station_group_and_line_id(
            station_group_id,
            line_id,
            &mut conn,
        )
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
}

struct InternalStationRepository {}

impl InternalStationRepository {
    async fn find_by_id(
        id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Option<Station>, DomainError> {
        let rows: Option<StationRow> = sqlx::query_as(
            "SELECT l.*,
            s.*,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            (
              SELECT
                COUNT(line_group_cd)
              FROM
                station_station_types AS sst
              WHERE
                s.station_cd = sst.station_cd
                AND sst.pass <> 1
            ) AS station_types_count
            FROM
            (`stations` AS s, `lines` AS l)
            LEFT OUTER JOIN `line_aliases` AS la
                ON
                    la.station_cd = ?
            LEFT OUTER JOIN `aliases` AS a
                ON
                    la.alias_cd = a.id                   
            WHERE
            s.line_cd = l.line_cd
            AND s.station_cd = ?
            AND s.e_status = 0
          ORDER BY
            s.e_sort,
            s.station_cd",
        )
        .bind(id)
        .bind(id)
        .fetch_optional(conn)
        .await?;

        let station: Option<Station> = rows.map(|row| row.into());
        let Some(station) = station else {
            return Ok(None);
        };

        Ok(Some(station))
    }
    async fn get_by_line_id(
        line_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let station_row: Vec<StationRow> = sqlx::query_as(
            "SELECT DISTINCT l.*,
            s.*,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            (
              SELECT
                COUNT(line_group_cd)
              FROM
                station_station_types AS sst
              WHERE
                s.station_cd = sst.station_cd
                AND sst.pass <> 1
            ) AS station_types_count
          FROM
            (`stations` AS s, `lines` AS l)
            LEFT OUTER JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT OUTER JOIN `aliases` AS a ON la.alias_cd = a.id
          WHERE
            s.line_cd = ?
            AND s.line_cd = l.line_cd
            AND s.e_status = 0
          ORDER BY
            s.e_sort,
            s.station_cd",
        )
        .bind(line_id)
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = station_row.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_station_group_id(
        group_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows: Vec<StationRow> = sqlx::query_as(
            "SELECT l.*,
            s.*,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            (
              SELECT
                COUNT(line_group_cd)
              FROM
                station_station_types AS sst
              WHERE
                s.station_cd = sst.station_cd
                AND sst.pass <> 1
            ) AS station_types_count
          FROM
            (`stations` AS s, `lines` AS l)
            LEFT OUTER JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT OUTER JOIN `aliases` AS a ON a.id = la.alias_cd
          WHERE
            s.station_g_cd = ?
            AND s.line_cd = l.line_cd
            AND s.e_status = 0",
        )
        .bind(group_id)
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_station_group_id_vec(
        group_id_vec: Vec<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        if group_id_vec.len().is_zero() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(group_id_vec.len() - 1));
        let query_str = format!(
            "SELECT l.*,
            s.*,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            (
              SELECT
                COUNT(line_group_cd)
              FROM
                station_station_types AS sst
              WHERE
                s.station_cd = sst.station_cd
                AND sst.pass <> 1
            ) AS station_types_count
          FROM
            (`stations` AS s, `lines` AS l)
            LEFT OUTER JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT OUTER JOIN `aliases` AS a ON a.id = la.alias_cd
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

    async fn get_by_station_group_and_line_id(
        station_group_id: u32,
        line_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Option<Station>, DomainError> {
        let rows: Option<StationRow> = sqlx::query_as(
            "SELECT l.*,
            s.*,
            (
              SELECT
                COUNT(line_group_cd)
              FROM
                station_station_types AS sst
              WHERE
                s.station_cd = sst.station_cd
                AND sst.pass <> 1
            ) AS station_types_count
          FROM
            `stations` AS s,
            `lines` AS l
          WHERE
            s.station_g_cd = ?
            AND s.line_cd = ?
            AND s.e_status = 0
            AND l.line_cd = s.line_cd
          ORDER BY
            s.e_sort,
            s.station_cd",
        )
        .bind(station_group_id)
        .bind(line_id)
        .fetch_optional(conn)
        .await?;

        let station: Option<Station> = rows.map(|row| row.into());
        let Some(station) = station else {
            return Ok(None);
        };
        Ok(Some(station))
    }

    async fn get_by_coordinates(
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows = sqlx::query_as::<_, StationRow>(
            "SELECT l.*,
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
            (
              SELECT
                COUNT(line_group_cd)
              FROM
                station_station_types AS sst
              WHERE
                s.station_cd = sst.station_cd
                AND sst.pass <> 1
            ) AS station_types_count
          FROM
            (`stations` AS s, `lines` AS l)
            LEFT OUTER JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT OUTER JOIN `aliases` AS a ON a.id = la.alias_cd
          WHERE
            s.line_cd = l.line_cd
            AND s.e_status = 0
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

    async fn get_by_name(
        station_name: String,
        limit: Option<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let station_name = format!("%{}%", station_name);

        let rows = sqlx::query_as::<_, StationRow>(
            "SELECT l.*,
            s.*,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            (
                SELECT COUNT(line_group_cd)
                FROM station_station_types AS sst
                WHERE s.station_cd = sst.station_cd
                AND sst.pass <> 1
            ) AS station_types_count
            FROM
            (`stations` AS s,
            `lines` AS l)
            LEFT OUTER JOIN `line_aliases` AS la
                ON
                    la.station_cd = s.station_cd
            LEFT OUTER JOIN `aliases` AS a
                ON
                    la.alias_cd = a.id
            WHERE s.line_cd = l.line_cd
            AND (
                station_name LIKE ?
                OR station_name_r LIKE ?
                OR station_name_k LIKE ?
                OR station_name_zh LIKE ?
                OR station_name_ko LIKE ?
        )
            AND s.e_status = 0
            ORDER BY s.e_sort, s.station_cd
            LIMIT ?",
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
        let rows: Vec<StationRow> = sqlx::query_as(
            "SELECT
            DISTINCT l.*,
            s.*,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            (
              SELECT
                COUNT(line_group_cd)
              FROM
                `station_station_types` AS sst
              WHERE
                s.station_cd = sst.station_cd
                AND sst.pass <> 1
            ) AS station_types_count,
            CONVERT(sst.pass, SIGNED) AS pass
          FROM
            (
              `lines` AS l, `stations` AS s, `station_station_types` AS sst
            )
            LEFT OUTER JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT OUTER JOIN `aliases` AS a ON la.alias_cd = a.id
          WHERE
            sst.line_group_cd = ?
            AND sst.station_cd = s.station_cd
            AND s.line_cd = l.line_cd
            AND s.e_status = 0",
        )
        .bind(line_group_id)
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }
}
