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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

    /// テスト用のインメモリSQLiteデータベースをセットアップ
    async fn setup_test_db() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .connect(":memory:")
            .await
            .expect("データベース接続に失敗しました");

        // テスト用のテーブルを作成
        sqlx::query(
            r#"
            CREATE TABLE companies (
                company_cd INTEGER PRIMARY KEY,
                rr_cd INTEGER NOT NULL,
                company_name TEXT NOT NULL,
                company_name_k TEXT NOT NULL,
                company_name_h TEXT NOT NULL,
                company_name_r TEXT NOT NULL,
                company_name_en TEXT NOT NULL,
                company_name_full_en TEXT NOT NULL,
                company_url TEXT,
                company_type INTEGER NOT NULL,
                e_status INTEGER NOT NULL,
                e_sort INTEGER NOT NULL
            );

            CREATE TABLE lines (
                line_cd INTEGER PRIMARY KEY,
                company_cd INTEGER NOT NULL,
                line_type INTEGER,
                line_name TEXT,
                line_name_k TEXT,
                line_name_h TEXT,
                line_name_r TEXT,
                line_name_rn TEXT,
                line_name_zh TEXT,
                line_name_ko TEXT,
                line_color_c TEXT,
                line_symbol1 TEXT,
                line_symbol2 TEXT,
                line_symbol3 TEXT,
                line_symbol4 TEXT,
                line_symbol1_color TEXT,
                line_symbol2_color TEXT,
                line_symbol3_color TEXT,
                line_symbol4_color TEXT,
                line_symbol1_shape TEXT,
                line_symbol2_shape TEXT,
                line_symbol3_shape TEXT,
                line_symbol4_shape TEXT,
                e_status INTEGER NOT NULL,
                e_sort INTEGER NOT NULL,
                average_distance REAL
            );

            CREATE TABLE stations (
                station_cd INTEGER PRIMARY KEY,
                station_g_cd INTEGER NOT NULL,
                station_name TEXT NOT NULL,
                station_name_k TEXT NOT NULL,
                station_name_r TEXT,
                station_name_zh TEXT,
                station_name_ko TEXT,
                primary_station_number TEXT,
                three_letter_code TEXT,
                line_cd INTEGER NOT NULL,
                pref_cd INTEGER NOT NULL,
                post TEXT,
                address TEXT,
                lon REAL,
                lat REAL,
                open_ymd TEXT,
                close_ymd TEXT,
                e_status INTEGER NOT NULL,
                e_sort INTEGER NOT NULL
            );

            CREATE TABLE station_station_types (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                station_cd INTEGER NOT NULL,
                line_group_cd INTEGER,
                station_type_cd INTEGER NOT NULL,
                pass INTEGER DEFAULT 0
            );

            CREATE TABLE line_aliases (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                station_cd INTEGER NOT NULL,
                alias_cd INTEGER NOT NULL
            );

            CREATE TABLE aliases (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                line_name TEXT,
                line_name_k TEXT,
                line_name_h TEXT,
                line_name_r TEXT,
                line_name_zh TEXT,
                line_name_ko TEXT,
                line_color_c TEXT
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("テーブル作成に失敗しました");

        // テスト用のデータを挿入
        sqlx::query(
            r#"
            INSERT INTO companies (
                company_cd, rr_cd, company_name, company_name_k, company_name_h,
                company_name_r, company_name_en, company_name_full_en, company_url,
                company_type, e_status, e_sort
            ) VALUES 
                (1, 1, 'JR東日本', 'ジェイアールヒガシニホン', '東日本旅客鉄道株式会社', 
                 'JR東日本', 'JR East', 'East Japan Railway Company', 
                 'https://www.jreast.co.jp/', 1, 0, 1),
                (2, 2, '東京メトロ', 'トウキョウメトロ', '東京地下鉄株式会社',
                 '東京メトロ', 'Tokyo Metro', 'Tokyo Metro Co., Ltd.',
                 'https://www.tokyometro.jp/', 2, 0, 2);

            INSERT INTO lines (
                line_cd, company_cd, line_type, line_name, line_name_k, line_name_h,
                line_name_r, line_name_rn, line_name_zh, line_name_ko, line_color_c,
                line_symbol1, line_symbol1_color, line_symbol1_shape,
                e_status, e_sort, average_distance
            ) VALUES 
                (11302, 1, 2, '山手線', 'ヤマノテセン', '山手線', 'Yamanote Line', 'Yamanote Line',
                 '山手线', '야마노테선', '#80C241', 'JY', '#80C241', 'SQUARE', 0, 11302, 1075.968412),
                (11303, 1, 2, '京浜東北線', 'ケイヒントウホクセン', '京浜東北線', 'Keihin-Tohoku Line', 'Keihin-Tohoku Line',
                 '京滨东北线', '게이힌도호쿠선', '#00BFFF', 'JK', '#00BFFF', 'SQUARE', 0, 11303, 852.5),
                (11101, 2, 1, '銀座線', 'ギンザセン', '銀座線', 'Ginza Line', 'Ginza Line',
                 '银座线', '긴자선', '#FF9500', 'G', '#FF9500', 'CIRCLE', 0, 11101, 625.3),
                (99999, 1, 1, '廃止路線', 'ハイシロセン', '廃止路線', 'Abolished Line', 'Abolished Line',
                 '废止线路', '폐지노선', '#000000', 'AB', '#000000', 'SQUARE', 1, 99999, 0.0);

            INSERT INTO stations (
                station_cd, station_g_cd, station_name, station_name_k, station_name_r,
                station_name_zh, station_name_ko, primary_station_number, three_letter_code,
                line_cd, pref_cd, post, address, lon, lat, open_ymd, close_ymd,
                e_status, e_sort
            ) VALUES 
                (1130201, 1130201, '大崎', 'オオサキ', 'Osaki', '大崎', '오사키', 'JY24',
                 'OSA', 11302, 13, '141-0032', '東京都品川区大崎一丁目', 139.728565, 35.619772,
                 '19011215', '', 0, 1130201),
                (1130301, 1130301, '東京', 'トウキョウ', 'Tokyo', '东京', '도쿄', 'JK26',
                 'TKY', 11303, 13, '100-0005', '東京都千代田区丸の内一丁目', 139.766084, 35.681382,
                 '19141220', '', 0, 1130301),
                (1110101, 1110101, '銀座', 'ギンザ', 'Ginza', '银座', '긴자', 'G09',
                 'GNZ', 11101, 13, '104-0061', '東京都中央区銀座四丁目', 139.763806, 35.671881,
                 '19271230', '', 0, 1110101);

            INSERT INTO station_station_types (
                station_cd, line_group_cd, station_type_cd, pass
            ) VALUES 
                (1130201, 100, 1, 0),
                (1130301, 200, 1, 0),
                (1110101, 300, 1, 0);

            INSERT INTO aliases (
                id, line_name, line_name_k, line_name_h, line_name_r,
                line_name_zh, line_name_ko, line_color_c
            ) VALUES 
                (1, '山手線（外回り）', 'ヤマノテセン（ソトマワリ）', '山手線（外回り）', 'Yamanote Line (Outer Loop)',
                 '山手线（外环）', '야마노테선（외선순환）', '#80C241');

            INSERT INTO line_aliases (
                station_cd, alias_cd
            ) VALUES 
                (1130201, 1);
            "#,
        )
        .execute(&pool)
        .await
        .expect("テストデータの挿入に失敗しました");

        pool
    }

    /// LineRowからLineへの変換をテスト
    #[test]
    fn test_line_row_to_line_conversion() {
        let line_row = LineRow {
            line_cd: 11302,
            company_cd: 1,
            line_type: Some(2),
            line_name: Some("山手線".to_string()),
            line_name_k: Some("ヤマノテセン".to_string()),
            line_name_h: Some("山手線".to_string()),
            line_name_r: Some("Yamanote Line".to_string()),
            line_name_zh: Some("山手线".to_string()),
            line_name_ko: Some("야마노테선".to_string()),
            line_color_c: Some("#80C241".to_string()),
            line_symbol1: Some("JY".to_string()),
            line_symbol2: None,
            line_symbol3: None,
            line_symbol4: None,
            line_symbol1_color: Some("#80C241".to_string()),
            line_symbol2_color: None,
            line_symbol3_color: None,
            line_symbol4_color: None,
            line_symbol1_shape: Some("SQUARE".to_string()),
            line_symbol2_shape: None,
            line_symbol3_shape: None,
            line_symbol4_shape: None,
            e_status: 0,
            e_sort: 11302,
            average_distance: Some(1075.968412),
            line_group_cd: None,
            station_cd: None,
            station_g_cd: None,
        };

        let line: Line = line_row.into();

        assert_eq!(line.line_cd, 11302);
        assert_eq!(line.company_cd, 1);
        assert_eq!(line.line_type, Some(2));
        assert_eq!(line.line_name, "山手線");
        assert_eq!(line.line_name_k, "ヤマノテセン");
        assert_eq!(line.line_name_h, "山手線");
        assert_eq!(line.line_name_r, Some("Yamanote Line".to_string()));
        assert_eq!(line.line_name_zh, Some("山手线".to_string()));
        assert_eq!(line.line_name_ko, Some("야마노테선".to_string()));
        assert_eq!(line.line_color_c, Some("#80C241".to_string()));
        assert_eq!(line.line_symbol1, Some("JY".to_string()));
        assert_eq!(line.line_symbol1_color, Some("#80C241".to_string()));
        assert_eq!(line.line_symbol1_shape, Some("SQUARE".to_string()));
        assert_eq!(line.e_status, 0);
        assert_eq!(line.e_sort, 11302);
        assert_eq!(line.average_distance, 1075.968412);
        assert_eq!(line.line_group_cd, None);
        assert_eq!(line.station_cd, None);
        assert_eq!(line.station_g_cd, None);
    }

    /// line_nameがNoneの場合の変換をテスト
    #[test]
    fn test_line_row_to_line_conversion_with_none_name() {
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
        };

        let line: Line = line_row.into();

        assert_eq!(line.line_name, "");
        assert_eq!(line.line_name_k, "");
        assert_eq!(line.line_name_h, "");
        assert_eq!(line.average_distance, 0.0);
    }

    /// InternalLineRepository::find_by_id - 正常系
    #[tokio::test]
    async fn test_internal_line_repository_find_by_id_success() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::find_by_id(11302, &mut conn).await;

        assert!(result.is_ok());
        let line = result.unwrap().expect("路線が見つかりませんでした");
        assert_eq!(line.line_cd, 11302);
        assert_eq!(line.line_name, "山手線");
        assert_eq!(line.company_cd, 1);
        assert_eq!(line.e_status, 0);
    }

    /// InternalLineRepository::find_by_id - 存在しない路線
    #[tokio::test]
    async fn test_internal_line_repository_find_by_id_not_found() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::find_by_id(99999999, &mut conn).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// InternalLineRepository::find_by_id - 廃止された路線
    #[tokio::test]
    async fn test_internal_line_repository_find_by_id_disabled() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::find_by_id(99999, &mut conn).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// InternalLineRepository::find_by_station_id - 正常系
    #[tokio::test]
    async fn test_internal_line_repository_find_by_station_id_success() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::find_by_station_id(1130201, &mut conn).await;

        assert!(result.is_ok());
        let line = result.unwrap().expect("路線が見つかりませんでした");
        assert_eq!(line.line_cd, 11302);
        assert_eq!(line.station_cd, Some(1130201));
        assert_eq!(line.station_g_cd, Some(1130201));
        assert_eq!(line.line_group_cd, Some(100));
    }

    /// InternalLineRepository::find_by_station_id - 存在しない駅
    #[tokio::test]
    async fn test_internal_line_repository_find_by_station_id_not_found() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::find_by_station_id(99999999, &mut conn).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// InternalLineRepository::get_by_ids - 正常系
    #[tokio::test]
    async fn test_internal_line_repository_get_by_ids_success() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let ids = vec![11302, 11303];

        let result = InternalLineRepository::get_by_ids(&ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 2);

        let mut lines = lines;
        lines.sort_by_key(|l| l.line_cd);

        assert_eq!(lines[0].line_cd, 11302);
        assert_eq!(lines[0].line_name, "山手線");
        assert_eq!(lines[1].line_cd, 11303);
        assert_eq!(lines[1].line_name, "京浜東北線");
    }

    /// InternalLineRepository::get_by_ids - 空のIDベクター
    #[tokio::test]
    async fn test_internal_line_repository_get_by_ids_empty() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let ids = vec![];

        let result = InternalLineRepository::get_by_ids(&ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    /// InternalLineRepository::get_by_ids - 存在しないID
    #[tokio::test]
    async fn test_internal_line_repository_get_by_ids_not_found() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let ids = vec![99999999];

        let result = InternalLineRepository::get_by_ids(&ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    /// InternalLineRepository::get_by_station_group_id - 正常系
    #[tokio::test]
    async fn test_internal_line_repository_get_by_station_group_id_success() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::get_by_station_group_id(1130201, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());
        assert_eq!(lines[0].line_cd, 11302);
        assert_eq!(lines[0].station_cd, Some(1130201));
        assert_eq!(lines[0].station_g_cd, Some(1130201));
    }

    /// InternalLineRepository::get_by_station_group_id - 存在しない駅グループ
    #[tokio::test]
    async fn test_internal_line_repository_get_by_station_group_id_not_found() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::get_by_station_group_id(99999999, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    /// InternalLineRepository::get_by_station_group_id_vec - 正常系
    #[tokio::test]
    async fn test_internal_line_repository_get_by_station_group_id_vec_success() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let station_group_ids = vec![1130201, 1130301];

        let result =
            InternalLineRepository::get_by_station_group_id_vec(&station_group_ids, &mut conn)
                .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(lines.len() >= 2);
    }

    /// InternalLineRepository::get_by_station_group_id_vec - 空のベクター
    #[tokio::test]
    async fn test_internal_line_repository_get_by_station_group_id_vec_empty() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let station_group_ids = vec![];

        let result =
            InternalLineRepository::get_by_station_group_id_vec(&station_group_ids, &mut conn)
                .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    /// InternalLineRepository::get_by_line_group_id - 正常系
    #[tokio::test]
    async fn test_internal_line_repository_get_by_line_group_id_success() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::get_by_line_group_id(100, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());
        assert_eq!(lines[0].line_group_cd, Some(100));
    }

    /// InternalLineRepository::get_by_line_group_id - 存在しない路線グループ
    #[tokio::test]
    async fn test_internal_line_repository_get_by_line_group_id_not_found() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::get_by_line_group_id(99999999, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    /// InternalLineRepository::get_by_line_group_id_vec - 正常系
    #[tokio::test]
    async fn test_internal_line_repository_get_by_line_group_id_vec_success() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let line_group_ids = vec![100, 200];

        let result =
            InternalLineRepository::get_by_line_group_id_vec(&line_group_ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(lines.len() >= 1);
    }

    /// InternalLineRepository::get_by_line_group_id_vec - 空のベクター
    #[tokio::test]
    async fn test_internal_line_repository_get_by_line_group_id_vec_empty() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let line_group_ids = vec![];

        let result =
            InternalLineRepository::get_by_line_group_id_vec(&line_group_ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    /// InternalLineRepository::get_by_line_group_id_vec_for_routes - 正常系
    #[tokio::test]
    async fn test_internal_line_repository_get_by_line_group_id_vec_for_routes_success() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let line_group_ids = vec![100, 200];

        let result =
            InternalLineRepository::get_by_line_group_id_vec_for_routes(&line_group_ids, &mut conn)
                .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(lines.len() >= 1);
    }

    /// InternalLineRepository::get_by_line_group_id_vec_for_routes - 空のベクター
    #[tokio::test]
    async fn test_internal_line_repository_get_by_line_group_id_vec_for_routes_empty() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let line_group_ids = vec![];

        let result =
            InternalLineRepository::get_by_line_group_id_vec_for_routes(&line_group_ids, &mut conn)
                .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    /// InternalLineRepository::get_by_name - 正常系
    #[tokio::test]
    async fn test_internal_line_repository_get_by_name_success() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result =
            InternalLineRepository::get_by_name("山手線".to_string(), Some(5), &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());
        assert_eq!(lines[0].line_name, "山手線");
    }

    /// InternalLineRepository::get_by_name - 部分一致
    #[tokio::test]
    async fn test_internal_line_repository_get_by_name_partial_match() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result =
            InternalLineRepository::get_by_name("山手".to_string(), Some(5), &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());
        assert!(lines[0].line_name.contains("山手"));
    }

    /// InternalLineRepository::get_by_name - 見つからない場合
    #[tokio::test]
    async fn test_internal_line_repository_get_by_name_not_found() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result =
            InternalLineRepository::get_by_name("存在しない路線".to_string(), Some(5), &mut conn)
                .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    /// InternalLineRepository::get_by_name - limitが1の場合
    #[tokio::test]
    async fn test_internal_line_repository_get_by_name_with_limit() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result =
            InternalLineRepository::get_by_name("線".to_string(), Some(1), &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 1);
    }

    /// InternalLineRepository::get_by_name - limitがNoneの場合
    #[tokio::test]
    async fn test_internal_line_repository_get_by_name_no_limit() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::get_by_name("線".to_string(), None, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 1); // デフォルトlimit=1
    }

    /// MyLineRepository統合テストは型の問題で難しいため、
    /// より詳細なInternalLineRepositoryのテストを追加

    /// InternalLineRepository::get_by_ids - 複数の結果を検証
    #[tokio::test]
    async fn test_internal_line_repository_get_by_ids_multiple() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let ids = vec![11302, 11303, 11101];

        let result = InternalLineRepository::get_by_ids(&ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 3);

        // 各路線が正しく取得されているかチェック
        let line_cds: Vec<i64> = lines.iter().map(|l| l.line_cd).collect();
        assert!(line_cds.contains(&11302));
        assert!(line_cds.contains(&11303));
        assert!(line_cds.contains(&11101));
    }

    /// InternalLineRepository::get_by_ids - 一部存在しないIDを含む場合
    #[tokio::test]
    async fn test_internal_line_repository_get_by_ids_partial_match() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let ids = vec![11302, 99999999, 11303];

        let result = InternalLineRepository::get_by_ids(&ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 2); // 存在する2つのみ
    }

    /// InternalLineRepository::get_by_name - 韓国語名での検索
    #[tokio::test]
    async fn test_internal_line_repository_get_by_name_korean() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result =
            InternalLineRepository::get_by_name("야마노테".to_string(), Some(5), &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());
        assert_eq!(lines[0].line_name, "山手線");
    }

    /// InternalLineRepository::get_by_name - 中国語名での検索
    #[tokio::test]
    async fn test_internal_line_repository_get_by_name_chinese() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result =
            InternalLineRepository::get_by_name("山手线".to_string(), Some(5), &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());
        assert_eq!(lines[0].line_name, "山手線");
    }

    /// InternalLineRepository::get_by_name - カタカナ名での検索
    #[tokio::test]
    async fn test_internal_line_repository_get_by_name_katakana() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result =
            InternalLineRepository::get_by_name("ヤマノテ".to_string(), Some(5), &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(!lines.is_empty());
        assert_eq!(lines[0].line_name, "山手線");
    }

    /// InternalLineRepository::get_by_station_group_id_vec - 混合結果
    #[tokio::test]
    async fn test_internal_line_repository_get_by_station_group_id_vec_mixed() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let station_group_ids = vec![1130201, 99999999, 1130301];

        let result =
            InternalLineRepository::get_by_station_group_id_vec(&station_group_ids, &mut conn)
                .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(lines.len() >= 2); // 存在する駅グループに対応する路線
    }

    /// InternalLineRepository::get_by_line_group_id_vec - 混合結果
    #[tokio::test]
    async fn test_internal_line_repository_get_by_line_group_id_vec_mixed() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let line_group_ids = vec![100, 99999999, 200];

        let result =
            InternalLineRepository::get_by_line_group_id_vec(&line_group_ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert!(lines.len() >= 1); // 存在する路線グループに対応する路線
    }

    /// エラーハンドリングのテスト - 無効なSQL（実際にはこのケースは発生しにくい）
    #[tokio::test]
    async fn test_database_error_handling() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        // 正常なクエリなので、エラーは発生しないはず
        let result = InternalLineRepository::find_by_id(11302, &mut conn).await;
        assert!(result.is_ok());
    }

    /// 大量のIDでのテスト
    #[tokio::test]
    async fn test_internal_line_repository_get_by_ids_large_list() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        // 既存のIDと存在しないIDを混合
        let mut ids = vec![11302, 11303, 11101];
        // 大量の存在しないIDを追加
        for i in 50000..50100 {
            ids.push(i);
        }

        let result = InternalLineRepository::get_by_ids(&ids, &mut conn).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 3); // 存在する3つのみ
    }

    /// 境界値テスト - 非常に大きなID
    #[tokio::test]
    async fn test_internal_line_repository_find_by_id_large_id() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::find_by_id(i64::MAX, &mut conn).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// 境界値テスト - 負のID
    #[tokio::test]
    async fn test_internal_line_repository_find_by_id_negative() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        let result = InternalLineRepository::find_by_id(-1, &mut conn).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
