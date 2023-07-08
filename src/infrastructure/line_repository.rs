use async_trait::async_trait;

use bigdecimal::{BigDecimal, ToPrimitive, Zero};

use moka::sync::Cache;
use sqlx::{MySql, MySqlConnection, Pool};

use crate::domain::{
    entity::line::Line, error::DomainError, repository::line_repository::LineRepository,
};

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
    pub lon: BigDecimal,
    pub lat: BigDecimal,
    pub zoom: u32,
    pub e_status: u32,
    pub e_sort: u32,
}

impl From<LineRow> for Line {
    fn from(row: LineRow) -> Self {
        Self {
            line_cd: row.line_cd,
            company_cd: row.company_cd,
            company: None,
            line_name: row.line_name,
            line_name_k: row.line_name_k,
            line_name_h: row.line_name_h,
            line_name_r: row.line_name_r,
            line_name_zh: row.line_name_zh,
            line_name_ko: row.line_name_ko,
            line_color_c: row.line_color_c,
            line_color_t: row.line_color_t,
            line_type: row.line_type,
            line_symbols: vec![],
            line_symbol_primary: row.line_symbol_primary,
            line_symbol_secondary: row.line_symbol_secondary,
            line_symbol_extra: row.line_symbol_extra,
            line_symbol_primary_color: row.line_symbol_primary_color,
            line_symbol_secondary_color: row.line_symbol_secondary_color,
            line_symbol_extra_color: row.line_symbol_extra_color,
            line_symbol_primary_shape: row.line_symbol_primary_shape,
            line_symbol_secondary_shape: row.line_symbol_secondary_shape,
            line_symbol_extra_shape: row.line_symbol_extra_shape,
            lon: row.lon.to_f64().unwrap_or(0.0),
            lat: row.lat.to_f64().unwrap_or(0.0),
            zoom: row.zoom,
            e_status: row.e_status,
            e_sort: row.e_sort,
            station: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MyLineRepository {
    pool: Pool<MySql>,
    cache: Cache<String, Vec<Line>>,
}

impl MyLineRepository {
    pub fn new(pool: Pool<MySql>, cache: Cache<String, Vec<Line>>) -> Self {
        Self { pool, cache }
    }
}

#[async_trait]
impl LineRepository for MyLineRepository {
    async fn find_by_id(&self, id: u32) -> Result<Option<Line>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::find_by_id(id, &mut conn, &self.cache).await
    }
    async fn get_by_ids(&self, ids: Vec<u32>) -> Result<Vec<Line>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_ids(ids, &mut conn, &self.cache).await
    }
    async fn get_by_station_group_id(&self, id: u32) -> Result<Vec<Line>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_station_group_id(id, &mut conn, &self.cache).await
    }
    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Line>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_line_group_id(line_group_id, &mut conn, &self.cache).await
    }
}

pub struct InternalLineRepository {}

impl InternalLineRepository {
    async fn find_by_id(
        id: u32,
        conn: &mut MySqlConnection,
        cache: &Cache<String, Vec<Line>>,
    ) -> Result<Option<Line>, DomainError> {
        let cache_key = format!("line_repository:find_by_id:{}", id);
        if let Some(cache_data) = cache.get(&cache_key) {
            if let Some(cache_data) = cache_data.first() {
                return Ok(Some(cache_data.clone()));
            }
        };

        let rows: Option<LineRow> =
            sqlx::query_as("SELECT * FROM `lines` WHERE line_cd = ? AND e_status = 0")
                .bind(id)
                .fetch_optional(conn)
                .await?;
        let line: Option<Line> = rows.map(|row| row.into());

        if let Some(line) = line.clone() {
            cache.insert(cache_key, vec![line]);
        }

        Ok(line)
    }

    async fn get_by_ids(
        ids: Vec<u32>,
        conn: &mut MySqlConnection,
        cache: &Cache<String, Vec<Line>>,
    ) -> Result<Vec<Line>, DomainError> {
        if ids.len().is_zero() {
            return Ok(vec![]);
        }

        let cache_key = format!(
            "line_repository:get_by_ids:{}",
            ids.clone()
                .into_iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",")
        );

        if let Some(cache_data) = cache.get(&cache_key) {
            return Ok(cache_data);
        };

        let params = format!("?{}", ", ?".repeat(ids.len() - 1));
        let query_str = format!(
            "SELECT * FROM `lines` WHERE line_cd IN ( {} ) AND e_status = 0",
            params
        );

        let mut query = sqlx::query_as::<_, LineRow>(&query_str);
        for id in ids {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();

        cache.insert(cache_key, lines.clone());
        Ok(lines)
    }

    async fn get_by_station_group_id(
        station_group_id: u32,
        conn: &mut MySqlConnection,
        cache: &Cache<String, Vec<Line>>,
    ) -> Result<Vec<Line>, DomainError> {
        let cache_key = format!(
            "line_repository:get_by_station_group_id:{}",
            station_group_id
        );
        if let Some(cache_data) = cache.get(&cache_key) {
            return Ok(cache_data);
        };

        let rows: Vec<LineRow> =
            sqlx::query_as("SELECT * FROM `lines` WHERE line_cd IN (SELECT line_cd FROM stations WHERE station_g_cd = ? AND e_status = 0) AND e_status = 0")
                .bind(station_group_id)
                .fetch_all(conn)
                .await?;
        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();

        cache.insert(cache_key, lines.clone());
        Ok(lines)
    }

    async fn get_by_line_group_id(
        line_group_id: u32,
        conn: &mut MySqlConnection,
        cache: &Cache<String, Vec<Line>>,
    ) -> Result<Vec<Line>, DomainError> {
        let cache_key = format!("line_repository:get_by_line_group_id:{}", line_group_id);
        if let Some(cache_data) = cache.get(&cache_key) {
            return Ok(cache_data);
        };

        let rows: Vec<LineRow> = sqlx::query_as(
            "SELECT l.*
            FROM `lines` AS l
            WHERE line_cd
            IN (
                SELECT line_cd
                FROM stations AS s
                INNER JOIN station_station_types AS sst
                ON s.station_cd = sst.station_cd
                AND line_group_cd = ?
            )
            AND e_status = 0",
        )
        .bind(line_group_id)
        .fetch_all(conn)
        .await?;
        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();

        cache.insert(cache_key, lines.clone());
        Ok(lines)
    }
}
