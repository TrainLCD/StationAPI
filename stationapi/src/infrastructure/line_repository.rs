use async_trait::async_trait;
use sqlx::SqliteConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::{
    entity::line::Line, error::DomainError, repository::line_repository::LineRepository,
};

#[derive(sqlx::FromRow, Clone)]
pub struct LineRow {
    pub line_cd: i64,
    pub company_cd: i64,
    pub line_type: Option<i64>,
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
    pub e_status: i64,
    pub e_sort: i64,
    pub average_distance: Option<f64>,
    pub line_group_cd: Option<i64>,
    pub station_cd: Option<i64>,
    pub station_g_cd: Option<i64>,
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
            average_distance: row.average_distance.unwrap_or(0.0),
        }
    }
}

pub struct MyLineRepository {
    conn: Arc<Mutex<SqliteConnection>>,
}

impl MyLineRepository {
    pub fn new(conn: Arc<Mutex<SqliteConnection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl LineRepository for MyLineRepository {
    async fn find_by_id(&self, id: u32) -> Result<Option<Line>, DomainError> {
        let id: i64 = id as i64;
        let mut conn = self.conn.lock().await;
        InternalLineRepository::find_by_id(id, &mut conn).await
    }
    async fn find_by_station_id(&self, station_id: u32) -> Result<Option<Line>, DomainError> {
        let station_id: i64 = station_id as i64;
        let mut conn = self.conn.lock().await;
        InternalLineRepository::find_by_station_id(station_id, &mut conn).await
    }
    async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<Line>, DomainError> {
        let ids: Vec<i64> = ids.iter().map(|x| *x as i64).collect();
        let mut conn = self.conn.lock().await;
        InternalLineRepository::get_by_ids(&ids, &mut conn).await
    }
    async fn get_by_station_group_id(&self, id: u32) -> Result<Vec<Line>, DomainError> {
        let id: i64 = id as i64;
        let mut conn = self.conn.lock().await;
        InternalLineRepository::get_by_station_group_id(id, &mut conn).await
    }
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        let station_group_id_vec: Vec<i64> =
            station_group_id_vec.iter().map(|x| *x as i64).collect();
        let mut conn = self.conn.lock().await;
        InternalLineRepository::get_by_station_group_id_vec(&station_group_id_vec, &mut conn).await
    }
    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Line>, DomainError> {
        let line_group_id: i64 = line_group_id as i64;
        let mut conn = self.conn.lock().await;
        InternalLineRepository::get_by_line_group_id(line_group_id, &mut conn).await
    }
    async fn get_by_line_group_id_vec(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        let line_group_id_vec: Vec<i64> = line_group_id_vec.iter().map(|x| *x as i64).collect();
        let mut conn = self.conn.lock().await;
        InternalLineRepository::get_by_line_group_id_vec(&line_group_id_vec, &mut conn).await
    }
    async fn get_by_line_group_id_vec_for_routes(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        let line_group_id_vec: Vec<i64> = line_group_id_vec.iter().map(|x| *x as i64).collect();
        let mut conn = self.conn.lock().await;
        InternalLineRepository::get_by_line_group_id_vec_for_routes(&line_group_id_vec, &mut conn)
            .await
    }
    async fn get_by_name(
        &self,
        line_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Line>, DomainError> {
        let limit = limit.map(|l| l as i64);
        let mut conn = self.conn.lock().await;
        InternalLineRepository::get_by_name(line_name, limit, &mut conn).await
    }
}

pub struct InternalLineRepository {}

impl InternalLineRepository {
    async fn find_by_id(id: i64, conn: &mut SqliteConnection) -> Result<Option<Line>, DomainError> {
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
            l.average_distance,
            CAST(NULL AS INTEGER) AS line_group_cd,
            CAST(NULL AS INTEGER) AS station_cd,
            CAST(NULL AS INTEGER) AS station_g_cd
            FROM `lines` AS l
            WHERE l.line_cd = ?
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
        conn: &mut SqliteConnection,
    ) -> Result<Option<Line>, DomainError> {
        let rows: Option<LineRow> = sqlx::query_as!(
            LineRow,
            "SELECT DISTINCT l.line_cd,
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
            l.average_distance,
            s.station_cd,
            s.station_g_cd,
            sst.line_group_cd,
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c
        FROM `lines` AS l
            JOIN `stations` AS s ON s.station_cd = ?
            JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd AND sst.pass <> 1
            LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT JOIN `aliases` AS a ON la.alias_cd = a.id
        WHERE l.line_cd = s.line_cd",
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

    async fn get_by_ids(
        ids: &[i64],
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Line>, DomainError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(ids.len() - 1));
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
                average_distance,
                CAST(NULL AS INTEGER) AS line_group_cd,
                CAST(NULL AS INTEGER) AS station_cd,
                CAST(NULL AS INTEGER) AS station_g_cd
            FROM `lines` WHERE line_cd IN ( {} ) AND e_status = 0",
            params
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
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Line>, DomainError> {
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
            l.average_distance,
            sst.line_group_cd,
            s.station_cd,
            s.station_g_cd
        FROM `lines` AS l
        JOIN `stations` AS s ON s.station_g_cd = ?
            AND s.e_status = 0
        JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd AND sst.pass <> 1
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
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Line>, DomainError> {
        if station_group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(station_group_id_vec.len() - 1));
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
                l.average_distance,
                s.station_cd,
                s.station_g_cd,
                sst.line_group_cd,
                COALESCE(a.line_name, l.line_name) AS line_name,
                COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
                COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
                COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
                COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
                COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
                COALESCE(a.line_color_c, l.line_color_c) AS line_color_c
            FROM `lines` AS l
            JOIN `stations` AS s ON s.station_g_cd IN ( {} )
            AND s.e_status = 0
            LEFT JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT JOIN `aliases` AS a ON la.alias_cd = a.id
            WHERE l.line_cd = s.line_cd
            AND l.e_status = 0
            AND (
                (sst.line_group_cd IS NOT NULL AND sst.pass <> 1)
                OR sst.line_group_cd IS NULL
            )
            GROUP BY s.station_cd",
            params
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
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Line>, DomainError> {
        let rows = sqlx::query_as!(
            LineRow,
            "SELECT DISTINCT l.line_cd,
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
            l.average_distance,
            s.station_cd,
            s.station_g_cd,
            sst.line_group_cd,
            l.line_name,
            l.line_name_k,
            l.line_name_h,
            l.line_name_r,
            l.line_name_zh,
            l.line_name_ko,
            l.line_color_c
        FROM `lines` AS l
            JOIN `station_station_types` AS sst ON sst.line_group_cd = ? AND sst.pass <> 1
            JOIN `stations` AS s ON s.station_cd = sst.station_cd
            AND s.e_status = 0
            LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT JOIN `aliases` AS a ON la.alias_cd = a.id
        WHERE l.line_cd = s.line_cd
            AND l.e_status = 0
            GROUP BY l.line_cd",
            line_group_id
        )
        .fetch_all(conn)
        .await?;
        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();
        Ok(lines)
    }

    async fn get_by_line_group_id_vec(
        line_group_id_vec: &[i64],
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Line>, DomainError> {
        if line_group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(line_group_id_vec.len() - 1));
        let query_str = format!(
            "SELECT DISTINCT
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
                l.average_distance,
                sst.line_group_cd,
                s.station_cd,
                s.station_g_cd
            FROM `lines` AS l
            JOIN `station_station_types` AS sst ON sst.line_group_cd IN ( {} ) AND sst.pass <> 1
            JOIN `stations` AS s ON s.station_cd = sst.station_cd AND s.e_status = 0
            LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT JOIN `aliases` AS a ON la.alias_cd = a.id
            WHERE
                l.line_cd = s.line_cd
                AND l.e_status = 0
            GROUP BY sst.line_group_cd, l.line_cd",
            params
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
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Line>, DomainError> {
        if line_group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(line_group_id_vec.len() - 1));
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
                l.average_distance,
                sst.line_group_cd,
                s.station_cd,
                s.station_g_cd
            FROM `lines` AS l
            JOIN `station_station_types` AS sst ON sst.line_group_cd IN ( {} ) AND sst.pass <> 1
            JOIN `stations` AS s ON s.station_cd = sst.station_cd AND s.e_status = 0 AND s.line_cd = l.line_cd
            LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT JOIN `aliases` AS a ON la.alias_cd = a.id
            WHERE l.e_status = 0
            GROUP BY l.line_cd",
            params
        );

        let mut query = sqlx::query_as::<_, LineRow>(&query_str);
        for id in line_group_id_vec {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }

    async fn get_by_name(
        line_name: String,
        limit: Option<i64>,
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Line>, DomainError> {
        let limit = limit.unwrap_or(1);
        let line_name = &format!("%{}%", line_name);

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
            l.average_distance,
            CAST(NULL AS INTEGER) AS line_group_cd,
            CAST(NULL AS INTEGER) AS station_cd,
            CAST(NULL AS INTEGER) AS station_g_cd
            FROM `lines` AS l
            WHERE (
                    l.line_name LIKE ?
                    OR l.line_name_rn LIKE ?
                    OR l.line_name_k LIKE ?
                    OR l.line_name_zh LIKE ?
                    OR l.line_name_ko LIKE ?
                )
                AND l.e_status = 0
            LIMIT ?",
            line_name,
            line_name,
            line_name,
            line_name,
            line_name,
            limit
        )
        .fetch_all(conn)
        .await?;

        let lines: Vec<Line> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }
}
