use async_trait::async_trait;
use sqlx::{PgConnection, Pool, Postgres};
use std::sync::Arc;

use crate::domain::{
    entity::{gtfs::TransportType, line::Line},
    error::DomainError,
    repository::line_repository::LineRepository,
};

#[derive(sqlx::FromRow, Clone)]
pub struct LineRow {
    pub line_cd: i32,
    pub company_cd: i32,
    pub line_type: Option<i32>,
    pub line_name: Option<String>,
    pub line_name_k: Option<String>,
    pub line_name_h: Option<String>,
    pub line_name_r: Option<String>,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: Option<String>,
    pub line_symbol1: Option<String>,
    pub line_symbol2: Option<String>,
    pub line_symbol3: Option<String>,
    pub line_symbol4: Option<String>,
    pub line_symbol1_color: Option<String>,
    pub line_symbol2_color: Option<String>,
    pub line_symbol3_color: Option<String>,
    pub line_symbol4_color: Option<String>,
    pub line_symbol1_shape: Option<String>,
    pub line_symbol2_shape: Option<String>,
    pub line_symbol3_shape: Option<String>,
    pub line_symbol4_shape: Option<String>,
    pub e_status: i32,
    pub e_sort: i32,
    pub average_distance: Option<f64>,
    pub line_group_cd: Option<i32>,
    pub station_cd: Option<i32>,
    pub station_g_cd: Option<i32>,
    pub type_cd: Option<i32>,
    pub transport_type: Option<i32>,
}

impl From<LineRow> for Line {
    fn from(row: LineRow) -> Self {
        Self {
            line_cd: row.line_cd,
            company_cd: row.company_cd,
            company: None,
            line_name: row.line_name.unwrap_or_default(),
            line_name_k: row.line_name_k.unwrap_or_default(),
            line_name_h: row.line_name_h.unwrap_or_default(),
            line_name_r: row.line_name_r,
            line_name_zh: row.line_name_zh,
            line_name_ko: row.line_name_ko,
            line_color_c: row.line_color_c,
            line_type: row.line_type,
            line_symbols: vec![],
            line_symbol1: row.line_symbol1,
            line_symbol2: row.line_symbol2,
            line_symbol3: row.line_symbol3,
            line_symbol4: row.line_symbol4,
            line_symbol1_color: row.line_symbol1_color,
            line_symbol2_color: row.line_symbol2_color,
            line_symbol3_color: row.line_symbol3_color,
            line_symbol4_color: row.line_symbol4_color,
            line_symbol1_shape: row.line_symbol1_shape,
            line_symbol2_shape: row.line_symbol2_shape,
            line_symbol3_shape: row.line_symbol3_shape,
            line_symbol4_shape: row.line_symbol4_shape,
            e_status: row.e_status,
            e_sort: row.e_sort,
            station: None,
            train_type: None,
            line_group_cd: row.line_group_cd,
            station_cd: row.station_cd,
            station_g_cd: row.station_g_cd,
            average_distance: row.average_distance,
            type_cd: row.type_cd,
            transport_type: TransportType::from(row.transport_type.unwrap_or(0)),
        }
    }
}

pub struct MyLineRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyLineRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LineRepository for MyLineRepository {
    async fn find_by_id(&self, id: u32) -> Result<Option<Line>, DomainError> {
        let id: i64 = id as i64;
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::find_by_id(id, &mut conn).await
    }
    async fn find_by_station_id(&self, station_id: u32) -> Result<Option<Line>, DomainError> {
        let station_id: i64 = station_id as i64;
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::find_by_station_id(station_id, &mut conn).await
    }
    async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<Line>, DomainError> {
        let ids: Vec<i64> = ids.iter().map(|x| *x as i64).collect();
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_ids(&ids, &mut conn).await
    }
    async fn get_by_station_group_id(&self, id: u32) -> Result<Vec<Line>, DomainError> {
        let id: i64 = id as i64;
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_station_group_id(id, &mut conn).await
    }
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        let station_group_id_vec: Vec<i64> =
            station_group_id_vec.iter().map(|x| *x as i64).collect();
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_station_group_id_vec(&station_group_id_vec, &mut conn).await
    }
    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Line>, DomainError> {
        let line_group_id: i64 = line_group_id as i64;
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_line_group_id(line_group_id, &mut conn).await
    }
    async fn get_by_line_group_id_vec(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        let line_group_id_vec: Vec<i64> = line_group_id_vec.iter().map(|x| *x as i64).collect();
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_line_group_id_vec(&line_group_id_vec, &mut conn).await
    }
    async fn get_by_line_group_id_vec_for_routes(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        let line_group_id_vec: Vec<i64> = line_group_id_vec.iter().map(|x| *x as i64).collect();
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_line_group_id_vec_for_routes(&line_group_id_vec, &mut conn)
            .await
    }
    async fn get_by_name(
        &self,
        line_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Line>, DomainError> {
        let limit = limit.map(|l| l as i64);
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_name(line_name, limit, &mut conn).await
    }
}

pub struct InternalLineRepository {}

impl InternalLineRepository {
    async fn find_by_id(id: i64, conn: &mut PgConnection) -> Result<Option<Line>, DomainError> {
        let id = id as i32;
        let rows: Option<LineRow> = sqlx::query_as!(
            LineRow,
            "SELECT 
            l.line_cd,
            l.company_cd,
            l.line_type,
            l.line_name,
            l.line_name_k,
            l.line_name_h,
            l.line_name_r,
            l.line_name_zh,
            l.line_name_ko,
            l.line_color_c,
            l.line_symbol1,
            l.line_symbol2,
            l.line_symbol3,
            l.line_symbol4,
            l.line_symbol1_color,
            l.line_symbol2_color,
            l.line_symbol3_color,
            l.line_symbol4_color,
            l.line_symbol1_shape,
            l.line_symbol2_shape,
            l.line_symbol3_shape,
            l.line_symbol4_shape,
            l.e_status,
            l.e_sort,
            COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
            CAST(NULL AS INTEGER) AS line_group_cd,
            CAST(NULL AS INTEGER) AS station_cd,
            CAST(NULL AS INTEGER) AS station_g_cd,
            CAST(NULL AS INTEGER) AS type_cd,
            l.transport_type
            FROM lines AS l
            WHERE l.line_cd = $1
            AND l.e_status = 0",
            id
        )
        .fetch_optional(conn)
        .await?;
        let line: Option<Line> = rows.map(|row| row.into());

        let Some(line) = line else {
            return Ok(None);
        };

        Ok(Some(line))
    }

    async fn find_by_station_id(
        station_id: i64,
        conn: &mut PgConnection,
    ) -> Result<Option<Line>, DomainError> {
        let station_id = station_id as i32;
        let rows: Option<LineRow> = sqlx::query_as!(
            LineRow,
            r#"SELECT l.line_cd,
            l.company_cd,
            l.line_type,
            COALESCE(alias_data.line_name, l.line_name) AS line_name,
            COALESCE(alias_data.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(alias_data.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(alias_data.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(alias_data.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(alias_data.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(alias_data.line_color_c, l.line_color_c) AS line_color_c,
            l.line_symbol1,
            l.line_symbol2,
            l.line_symbol3,
            l.line_symbol4,
            l.line_symbol1_color,
            l.line_symbol2_color,
            l.line_symbol3_color,
            l.line_symbol4_color,
            l.line_symbol1_shape,
            l.line_symbol2_shape,
            l.line_symbol3_shape,
            l.line_symbol4_shape,
            l.e_status,
            l.e_sort,
            COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
            sst.line_group_cd AS "line_group_cd?",
            s.station_cd,
            s.station_g_cd,
            sst.type_cd AS "type_cd?",
            l.transport_type
        FROM lines AS l
            JOIN stations AS s ON s.station_cd = $1
            LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd AND sst.pass <> 1
            LEFT JOIN (
                SELECT DISTINCT ON (la.station_cd)
                    la.station_cd,
                    a.line_name,
                    a.line_name_k,
                    a.line_name_h,
                    a.line_name_r,
                    a.line_name_zh,
                    a.line_name_ko,
                    a.line_color_c
                FROM line_aliases AS la
                JOIN aliases AS a ON la.alias_cd = a.id
                WHERE la.station_cd = $1
                LIMIT 1
            ) AS alias_data ON alias_data.station_cd = s.station_cd
        WHERE l.line_cd = s.line_cd"#,
            station_id,
        )
        .fetch_optional(conn)
        .await?;
        let line: Option<Line> = rows.map(|row| row.into());

        let Some(line) = line else {
            return Ok(None);
        };

        Ok(Some(line))
    }

    async fn get_by_ids(ids: &[i64], conn: &mut PgConnection) -> Result<Vec<Line>, DomainError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let params = (1..=ids.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let query_str = format!(
            "SELECT 
                line_cd,
                company_cd,
                line_type,
                line_name,
                line_name_k,
                line_name_h,
                line_name_r,
                line_name_zh,
                line_name_ko,
                line_color_c,
                line_symbol1,
                line_symbol2,
                line_symbol3,
                line_symbol4,
                line_symbol1_color,
                line_symbol2_color,
                line_symbol3_color,
                line_symbol4_color,
                line_symbol1_shape,
                line_symbol2_shape,
                line_symbol3_shape,
                line_symbol4_shape,
                e_status,
                e_sort,
                COALESCE(average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
                CAST(NULL AS INTEGER) AS line_group_cd,
                CAST(NULL AS INTEGER) AS station_cd,
                CAST(NULL AS INTEGER) AS station_g_cd,
                CAST(NULL AS INTEGER) AS type_cd,
                transport_type
            FROM lines WHERE line_cd IN ( {params} ) AND e_status = 0"
        );

        let mut query = sqlx::query_as::<_, LineRow>(&query_str);
        for id in ids {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }

    async fn get_by_station_group_id(
        station_group_id: i64,
        conn: &mut PgConnection,
    ) -> Result<Vec<Line>, DomainError> {
        let station_group_id = station_group_id as i32;
        let rows = sqlx::query_as!(
            LineRow,
            "SELECT DISTINCT l.line_cd,
            l.line_name,
            l.line_name_k,
            l.line_name_h,
            l.line_name_r,
            l.line_name_zh,
            l.line_name_ko,
            l.line_color_c,
            l.company_cd,
            l.line_type,
            l.line_symbol1,
            l.line_symbol2,
            l.line_symbol3,
            l.line_symbol4,
            l.line_symbol1_color,
            l.line_symbol2_color,
            l.line_symbol3_color,
            l.line_symbol4_color,
            l.line_symbol1_shape,
            l.line_symbol2_shape,
            l.line_symbol3_shape,
            l.line_symbol4_shape,
            l.e_status,
            l.e_sort,
            COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
            sst.line_group_cd,
            sst.type_cd,
            s.station_cd,
            s.station_g_cd,
            l.transport_type
        FROM lines AS l
        JOIN stations AS s ON s.station_g_cd = $1
            AND s.e_status = 0
        LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd AND sst.pass <> 1
        WHERE l.line_cd = s.line_cd
            AND l.e_status = 0",
            station_group_id
        )
        .fetch_all(conn)
        .await?;
        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }

    async fn get_by_station_group_id_vec(
        station_group_id_vec: &[i64],
        conn: &mut PgConnection,
    ) -> Result<Vec<Line>, DomainError> {
        if station_group_id_vec.is_empty() {
            return Ok(vec![]);
        }
        /*  */
        let params = (1..=station_group_id_vec.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let query_str = format!(
            "SELECT DISTINCT ON (s.station_cd)
                l.line_cd,
                l.company_cd,
                l.line_type,
                l.line_symbol1,
                l.line_symbol2,
                l.line_symbol3,
                l.line_symbol4,
                l.line_symbol1_color,
                l.line_symbol2_color,
                l.line_symbol3_color,
                l.line_symbol4_color,
                l.line_symbol1_shape,
                l.line_symbol2_shape,
                l.line_symbol3_shape,
                l.line_symbol4_shape,
                l.e_status,
                l.e_sort,
                COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
                s.station_cd,
                s.station_g_cd,
                sst.line_group_cd,
                sst.type_cd,
                COALESCE(a.line_name, l.line_name) AS line_name,
                COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
                COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
                COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
                COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
                COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
                COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
                l.transport_type
            FROM lines AS l
            JOIN stations AS s ON s.station_g_cd IN ( {params} )
            AND s.e_status = 0
            AND s.line_cd = l.line_cd
            LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON la.alias_cd = a.id
            WHERE l.e_status = 0
            AND (
                (sst.line_group_cd IS NOT NULL AND sst.pass <> 1)
                OR sst.line_group_cd IS NULL
            )"
        );

        let mut query = sqlx::query_as::<_, LineRow>(&query_str);
        for id in station_group_id_vec {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }

    async fn get_by_line_group_id(
        line_group_id: i64,
        conn: &mut PgConnection,
    ) -> Result<Vec<Line>, DomainError> {
        let line_group_id = line_group_id as i32;
        let rows = sqlx::query_as!(
            LineRow,
            "SELECT 
            l.line_cd,
            l.company_cd,
            l.line_type,
            l.line_symbol1,
            l.line_symbol2,
            l.line_symbol3,
            l.line_symbol4,
            l.line_symbol1_color,
            l.line_symbol2_color,
            l.line_symbol3_color,
            l.line_symbol4_color,
            l.line_symbol1_shape,
            l.line_symbol2_shape,
            l.line_symbol3_shape,
            l.line_symbol4_shape,
            l.e_status,
            l.e_sort,
            COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
            s.station_cd,
            s.station_g_cd,
            sst.line_group_cd,
            sst.type_cd,
            l.line_name,
            l.line_name_k,
            l.line_name_h,
            l.line_name_r,
            l.line_name_zh,
            l.line_name_ko,
            l.line_color_c,
            l.transport_type
        FROM lines AS l
            JOIN station_station_types AS sst ON sst.line_group_cd = $1 AND sst.pass <> 1
            JOIN stations AS s ON s.station_cd = sst.station_cd
            AND s.e_status = 0
            AND l.line_cd = s.line_cd
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON la.alias_cd = a.id
        WHERE l.e_status = 0",
            line_group_id
        )
        .fetch_all(conn)
        .await?;
        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();
        Ok(lines)
    }

    async fn get_by_line_group_id_vec(
        line_group_id_vec: &[i64],
        conn: &mut PgConnection,
    ) -> Result<Vec<Line>, DomainError> {
        if line_group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = (1..=line_group_id_vec.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let query_str = format!(
            "SELECT
                l.line_cd,
                l.company_cd,
                l.line_type,
                l.line_name,
                l.line_name_k,
                l.line_name_h,
                l.line_name_r,
                l.line_name_zh,
                l.line_name_ko,
                l.line_color_c,
                l.line_symbol1,
                l.line_symbol2,
                l.line_symbol3,
                l.line_symbol4,
                l.line_symbol1_color,
                l.line_symbol2_color,
                l.line_symbol3_color,
                l.line_symbol4_color,
                l.line_symbol1_shape,
                l.line_symbol2_shape,
                l.line_symbol3_shape,
                l.line_symbol4_shape,
                l.e_status,
                l.e_sort,
                COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
                sst.line_group_cd,
                sst.type_cd,
                s.station_cd,
                s.station_g_cd,
                l.transport_type
            FROM lines AS l
            JOIN station_station_types AS sst ON sst.line_group_cd IN ( {params} ) AND sst.pass <> 1
            JOIN stations AS s ON s.station_cd = sst.station_cd AND s.e_status = 0
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON la.alias_cd = a.id
            WHERE
                l.line_cd = s.line_cd
                AND l.e_status = 0"
        );

        let mut query = sqlx::query_as::<_, LineRow>(&query_str);
        for id in line_group_id_vec {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }
    async fn get_by_line_group_id_vec_for_routes(
        line_group_id_vec: &[i64],
        conn: &mut PgConnection,
    ) -> Result<Vec<Line>, DomainError> {
        if line_group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let line_group_id_vec: Vec<i32> = line_group_id_vec.iter().map(|x| *x as i32).collect();

        let rows = sqlx::query_as!(
            LineRow,
            "SELECT DISTINCT ON (sst.id, l.line_cd)
                l.line_cd,
                l.company_cd,
                l.line_type,
                l.line_name,
                l.line_name_k,
                l.line_name_h,
                l.line_name_r,
                l.line_name_zh,
                l.line_name_ko,
                l.line_color_c,
                l.line_symbol1,
                l.line_symbol2,
                l.line_symbol3,
                l.line_symbol4,
                l.line_symbol1_color,
                l.line_symbol2_color,
                l.line_symbol3_color,
                l.line_symbol4_color,
                l.line_symbol1_shape,
                l.line_symbol2_shape,
                l.line_symbol3_shape,
                l.line_symbol4_shape,
                l.e_status,
                l.e_sort,
                COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
                sst.line_group_cd,
                sst.type_cd,
                s.station_cd,
                s.station_g_cd,
                l.transport_type
            FROM lines AS l
            JOIN station_station_types AS sst ON sst.line_group_cd = ANY($1) AND sst.pass <> 1
            JOIN stations AS s ON s.station_cd = sst.station_cd AND s.e_status = 0 AND s.line_cd = l.line_cd
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON la.alias_cd = a.id
            WHERE l.e_status = 0
            ORDER BY sst.id, l.line_cd",
            &line_group_id_vec
        )
        .fetch_all(conn)
        .await?;
        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }

    async fn get_by_name(
        line_name: String,
        limit: Option<i64>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Line>, DomainError> {
        let limit = limit.unwrap_or(1);
        let line_name = &format!("%{line_name}%");

        let rows = sqlx::query_as!(
            LineRow,
            "SELECT 
            l.line_cd,
            l.company_cd,
            l.line_type,
            l.line_name,
            l.line_name_k,
            l.line_name_h,
            l.line_name_r,
            l.line_name_zh,
            l.line_name_ko,
            l.line_color_c,
            l.line_symbol1,
            l.line_symbol2,
            l.line_symbol3,
            l.line_symbol4,
            l.line_symbol1_color,
            l.line_symbol2_color,
            l.line_symbol3_color,
            l.line_symbol4_color,
            l.line_symbol1_shape,
            l.line_symbol2_shape,
            l.line_symbol3_shape,
            l.line_symbol4_shape,
            l.e_status,
            l.e_sort,
            COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
            CAST(NULL AS INTEGER) AS line_group_cd,
            CAST(NULL AS INTEGER) AS station_cd,
            CAST(NULL AS INTEGER) AS station_g_cd,
            CAST(NULL AS INTEGER) AS type_cd,
            l.transport_type
            FROM lines AS l
            WHERE (
                    l.line_name LIKE $1
                    OR l.line_name_rn LIKE $2
                    OR l.line_name_k LIKE $3
                    OR l.line_name_zh LIKE $4
                    OR l.line_name_ko LIKE $5
                )
                AND l.e_status = 0
            LIMIT $6",
            line_name,
            line_name,
            line_name,
            line_name,
            line_name,
            limit as i32
        )
        .fetch_all(conn)
        .await?;

        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use std::env;

    // テスト用のヘルパー関数
    async fn setup_test_db() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://test:test@localhost/stationapi_test".to_string());

        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    async fn setup_test_data(pool: &PgPool) {
        // テスト用のテーブルとデータを作成
        sqlx::query("DROP TABLE IF EXISTS lines CASCADE")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("DROP TABLE IF EXISTS stations CASCADE")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("DROP TABLE IF EXISTS station_station_types CASCADE")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("DROP TABLE IF EXISTS line_aliases CASCADE")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("DROP TABLE IF EXISTS aliases CASCADE")
            .execute(pool)
            .await
            .unwrap();

        // テーブル作成
        sqlx::query(
            "CREATE TABLE lines (
                line_cd INTEGER PRIMARY KEY,
                company_cd INTEGER NOT NULL,
                line_type INTEGER,
                line_name VARCHAR(255),
                line_name_k VARCHAR(255),
                line_name_h VARCHAR(255),
                line_name_r VARCHAR(255),
                line_name_rn VARCHAR(255),
                line_name_zh VARCHAR(255),
                line_name_ko VARCHAR(255),
                line_color_c VARCHAR(7),
                line_symbol1 VARCHAR(10),
                line_symbol2 VARCHAR(10),
                line_symbol3 VARCHAR(10),
                line_symbol4 VARCHAR(10),
                line_symbol1_color VARCHAR(7),
                line_symbol2_color VARCHAR(7),
                line_symbol3_color VARCHAR(7),
                line_symbol4_color VARCHAR(7),
                line_symbol1_shape VARCHAR(10),
                line_symbol2_shape VARCHAR(10),
                line_symbol3_shape VARCHAR(10),
                line_symbol4_shape VARCHAR(10),
                e_status INTEGER NOT NULL DEFAULT 0,
                e_sort INTEGER NOT NULL DEFAULT 0,
                average_distance DOUBLE PRECISION
            )",
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE stations (
                station_cd INTEGER PRIMARY KEY,
                station_g_cd INTEGER NOT NULL,
                line_cd INTEGER NOT NULL,
                e_status INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE station_station_types (
                station_cd INTEGER NOT NULL,
                line_group_cd INTEGER,
                type_cd INTEGER,
                pass INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE aliases (
                id INTEGER PRIMARY KEY,
                line_name VARCHAR(255),
                line_name_k VARCHAR(255),
                line_name_h VARCHAR(255),
                line_name_r VARCHAR(255),
                line_name_zh VARCHAR(255),
                line_name_ko VARCHAR(255),
                line_color_c VARCHAR(7)
            )",
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE line_aliases (
                station_cd INTEGER NOT NULL,
                alias_cd INTEGER NOT NULL
            )",
        )
        .execute(pool)
        .await
        .unwrap();

        // テストデータの挿入
        sqlx::query(
            "INSERT INTO lines (line_cd, company_cd, line_type, line_name, line_name_k, line_name_h, line_name_r, line_name_zh, line_name_ko, line_color_c, line_symbol1, e_status, e_sort, average_distance) VALUES 
            (1, 1, 1, 'Test Line 1', 'テストライン1', 'テストライン1', 'Test Line 1', '测试线路1', '테스트라인1', '#FF0000', 'T1', 0, 1, 1.5),
            (2, 1, 2, 'Test Line 2', 'テストライン2', 'テストライン2', 'Test Line 2', '测试线路2', '테스트라인2', '#00FF00', 'T2', 0, 2, 2.0),
            (3, 2, 1, 'Inactive Line', 'インアクティブライン', 'インアクティブライン', 'Inactive Line', '非活动线路', '비활성라인', '#0000FF', 'I1', 1, 3, 1.0)"
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO stations (station_cd, station_g_cd, line_cd, e_status) VALUES 
            (101, 201, 1, 0),
            (102, 202, 1, 0),
            (103, 203, 2, 0),
            (104, 204, 3, 1)",
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO station_station_types (station_cd, line_group_cd, type_cd, pass) VALUES 
            (101, 301, 1, 0),
            (102, 302, 2, 0),
            (103, 303, 1, 1),
            (104, 304, 3, 0)",
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO aliases (id, line_name, line_name_k, line_name_h, line_name_r, line_name_zh, line_name_ko, line_color_c) VALUES 
            (1, 'Alias Line 1', 'エイリアスライン1', 'エイリアスライン1', 'Alias Line 1', '别名线路1', '별명라인1', '#FFFF00')"
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO line_aliases (station_cd, alias_cd) VALUES 
            (101, 1)",
        )
        .execute(pool)
        .await
        .unwrap();
    }

    async fn cleanup_test_data(pool: &PgPool) {
        sqlx::query("DROP TABLE IF EXISTS line_aliases CASCADE")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DROP TABLE IF EXISTS aliases CASCADE")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DROP TABLE IF EXISTS station_station_types CASCADE")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DROP TABLE IF EXISTS stations CASCADE")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DROP TABLE IF EXISTS lines CASCADE")
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_line_row_to_line_conversion() {
        let line_row = LineRow {
            line_cd: 1,
            company_cd: 1,
            line_type: Some(1),
            line_name: Some("Test Line".to_string()),
            line_name_k: Some("テストライン".to_string()),
            line_name_h: Some("テストライン".to_string()),
            line_name_r: Some("Test Line".to_string()),
            line_name_zh: Some("测试线路".to_string()),
            line_name_ko: Some("테스트라인".to_string()),
            line_color_c: Some("#FF0000".to_string()),
            line_symbol1: Some("T1".to_string()),
            line_symbol2: None,
            line_symbol3: None,
            line_symbol4: None,
            line_symbol1_color: Some("#000000".to_string()),
            line_symbol2_color: None,
            line_symbol3_color: None,
            line_symbol4_color: None,
            line_symbol1_shape: Some("circle".to_string()),
            line_symbol2_shape: None,
            line_symbol3_shape: None,
            line_symbol4_shape: None,
            e_status: 0,
            e_sort: 1,
            average_distance: Some(1.5),
            line_group_cd: Some(301),
            station_cd: Some(101),
            station_g_cd: Some(201),
            type_cd: Some(1),
            transport_type: Some(0),
        };

        let line: Line = line_row.into();

        assert_eq!(line.line_cd, 1);
        assert_eq!(line.company_cd, 1);
        assert_eq!(line.line_name, "Test Line");
        assert_eq!(line.line_name_k, "テストライン");
        assert_eq!(line.line_name_h, "テストライン");
        assert_eq!(line.line_name_r, Some("Test Line".to_string()));
        assert_eq!(line.line_name_zh, Some("测试线路".to_string()));
        assert_eq!(line.line_name_ko, Some("테스트라인".to_string()));
        assert_eq!(line.line_color_c, Some("#FF0000".to_string()));
        assert_eq!(line.line_symbol1, Some("T1".to_string()));
        assert_eq!(line.e_status, 0);
        assert_eq!(line.e_sort, 1);
        assert_eq!(line.average_distance, Some(1.5));
        assert_eq!(line.line_group_cd, Some(301));
        assert_eq!(line.station_cd, Some(101));
        assert_eq!(line.station_g_cd, Some(201));
        assert_eq!(line.type_cd, Some(1));
    }

    #[tokio::test]
    async fn test_line_row_to_line_conversion_with_defaults() {
        let line_row = LineRow {
            line_cd: 1,
            company_cd: 1,
            line_type: None,
            line_name: None,
            line_name_k: None,
            line_name_h: None,
            line_name_r: None,
            line_name_zh: None,
            line_name_ko: None,
            line_color_c: None,
            line_symbol1: None,
            line_symbol2: None,
            line_symbol3: None,
            line_symbol4: None,
            line_symbol1_color: None,
            line_symbol2_color: None,
            line_symbol3_color: None,
            line_symbol4_color: None,
            line_symbol1_shape: None,
            line_symbol2_shape: None,
            line_symbol3_shape: None,
            line_symbol4_shape: None,
            e_status: 0,
            e_sort: 1,
            average_distance: None,
            line_group_cd: None,
            station_cd: None,
            station_g_cd: None,
            type_cd: None,
            transport_type: None,
        };

        let line: Line = line_row.into();

        assert_eq!(line.line_cd, 1);
        assert_eq!(line.company_cd, 1);
        assert_eq!(line.line_name, ""); // Default empty string
        assert_eq!(line.line_name_k, ""); // Default empty string
        assert_eq!(line.line_name_h, ""); // Default empty string
        assert_eq!(line.line_name_r, None);
        assert_eq!(line.line_color_c, None);
        assert_eq!(line.average_distance, None);
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_find_by_id_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result = InternalLineRepository::find_by_id(1, &mut conn).await;

        assert!(result.is_ok());
        let line = result.unwrap();
        assert!(line.is_some());

        let line = line.unwrap();
        assert_eq!(line.line_cd, 1);
        assert_eq!(line.line_name, "Test Line 1");
        assert_eq!(line.company_cd, 1);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_find_by_id_not_found() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result = InternalLineRepository::find_by_id(999, &mut conn).await;

        assert!(result.is_ok());
        let line = result.unwrap();
        assert!(line.is_none());

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_find_by_id_inactive_line() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result = InternalLineRepository::find_by_id(3, &mut conn).await;

        assert!(result.is_ok());
        let line = result.unwrap();
        assert!(line.is_none()); // e_status = 1 なので見つからない

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_find_by_station_id_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result = InternalLineRepository::find_by_station_id(101, &mut conn).await;

        assert!(result.is_ok());
        let line = result.unwrap();
        assert!(line.is_some());

        let line = line.unwrap();
        assert_eq!(line.station_cd, Some(101));
        // エイリアスがあるのでエイリアス名が使われる
        assert_eq!(line.line_name, "Alias Line 1");

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_find_by_station_id_not_found() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result = InternalLineRepository::find_by_station_id(999, &mut conn).await;

        assert!(result.is_ok());
        let line = result.unwrap();
        assert!(line.is_none());

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_ids_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let ids = vec![1, 2];
        let result = InternalLineRepository::get_by_ids(&ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 2);

        // ソートされていることを確認
        assert_eq!(lines[0].line_cd, 1);
        assert_eq!(lines[1].line_cd, 2);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_ids_empty() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let ids = vec![];
        let result = InternalLineRepository::get_by_ids(&ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_ids_with_inactive() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let ids = vec![1, 3]; // 3 は e_status = 1
        let result = InternalLineRepository::get_by_ids(&ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 1); // アクティブな線路のみ
        assert_eq!(lines[0].line_cd, 1);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_station_group_id_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result = InternalLineRepository::get_by_station_group_id(201, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_station_group_id_vec_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let station_group_ids = vec![201, 202];
        let result =
            InternalLineRepository::get_by_station_group_id_vec(&station_group_ids, &mut conn)
                .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_station_group_id_vec_empty() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let station_group_ids = vec![];
        let result =
            InternalLineRepository::get_by_station_group_id_vec(&station_group_ids, &mut conn)
                .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_line_group_id_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result = InternalLineRepository::get_by_line_group_id(301, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_line_group_id_vec_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let line_group_ids = vec![301, 302];
        let result =
            InternalLineRepository::get_by_line_group_id_vec(&line_group_ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_line_group_id_vec_empty() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let line_group_ids = vec![];
        let result =
            InternalLineRepository::get_by_line_group_id_vec(&line_group_ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_line_group_id_vec_for_routes_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let line_group_ids = vec![301, 302];
        let result =
            InternalLineRepository::get_by_line_group_id_vec_for_routes(&line_group_ids, &mut conn)
                .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_line_group_id_vec_for_routes_empty() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let line_group_ids = vec![];
        let result =
            InternalLineRepository::get_by_line_group_id_vec_for_routes(&line_group_ids, &mut conn)
                .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_name_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result =
            InternalLineRepository::get_by_name("Test".to_string(), Some(10), &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());

        // テスト名に "Test" を含む線路が見つかることを確認
        for line in &lines {
            assert!(line.line_name.contains("Test"));
        }

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_name_with_limit() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result =
            InternalLineRepository::get_by_name("Test".to_string(), Some(1), &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(lines.len() <= 1);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_name_no_results() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result =
            InternalLineRepository::get_by_name("NonExistent".to_string(), Some(10), &mut conn)
                .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    async fn test_my_line_repository_new() {
        let database_url = "postgres://test:test@localhost/stationapi_test";
        let pool = PgPool::connect(database_url).await;

        if let Ok(pool) = pool {
            let pool = Arc::new(pool);
            let repository = MyLineRepository::new(pool.clone());

            // プールが正しく設定されていることを確認
            assert!(Arc::ptr_eq(&repository.pool, &pool));
        }
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_line_repository_find_by_id() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyLineRepository::new(pool);

        let result = repository.find_by_id(1).await;
        assert!(result.is_ok());

        let line = result.unwrap();
        assert!(line.is_some());

        let line = line.unwrap();
        assert_eq!(line.line_cd, 1);

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_line_repository_find_by_station_id() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyLineRepository::new(pool);

        let result = repository.find_by_station_id(101).await;
        assert!(result.is_ok());

        let line = result.unwrap();
        assert!(line.is_some());

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_line_repository_get_by_ids() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyLineRepository::new(pool);

        let ids = vec![1, 2];
        let result = repository.get_by_ids(&ids).await;
        assert!(result.is_ok());

        let lines = result.unwrap();
        assert_eq!(lines.len(), 2);

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_line_repository_get_by_station_group_id() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyLineRepository::new(pool);

        let result = repository.get_by_station_group_id(201).await;
        assert!(result.is_ok());

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_line_repository_get_by_station_group_id_vec() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyLineRepository::new(pool);

        let station_group_ids = vec![201, 202];
        let result = repository
            .get_by_station_group_id_vec(&station_group_ids)
            .await;
        assert!(result.is_ok());

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_line_repository_get_by_line_group_id() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyLineRepository::new(pool);

        let result = repository.get_by_line_group_id(301).await;
        assert!(result.is_ok());

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_line_repository_get_by_line_group_id_vec() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyLineRepository::new(pool);

        let line_group_ids = vec![301, 302];
        let result = repository.get_by_line_group_id_vec(&line_group_ids).await;
        assert!(result.is_ok());

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_line_repository_get_by_line_group_id_vec_for_routes() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyLineRepository::new(pool);

        let line_group_ids = vec![301, 302];
        let result = repository
            .get_by_line_group_id_vec_for_routes(&line_group_ids)
            .await;
        assert!(result.is_ok());

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_line_repository_get_by_name() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyLineRepository::new(pool);

        let result = repository.get_by_name("Test".to_string(), Some(10)).await;
        assert!(result.is_ok());

        let lines = result.unwrap();
        assert!(!lines.is_empty());

        cleanup_test_data(&repository.pool).await;
    }
}
