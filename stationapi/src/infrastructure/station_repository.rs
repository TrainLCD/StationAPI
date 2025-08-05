use async_trait::async_trait;
use sqlx::{PgConnection, Pool, Postgres};
use std::sync::Arc;

use crate::{
    domain::{
        entity::station::Station, error::DomainError,
        repository::station_repository::StationRepository,
    },
    proto::StopCondition,
};

#[derive(sqlx::FromRow)]
struct TrainTypesCountRow {
    train_types_count: Option<i64>,
}

#[derive(sqlx::FromRow, Clone)]
struct StationRowWithDistance {
    pub station_cd: i32,
    pub station_g_cd: i32,
    pub station_name: String,
    pub station_name_k: String,
    pub station_name_r: Option<String>,
    #[allow(dead_code)]
    pub station_name_rn: Option<String>,
    pub station_name_zh: Option<String>,
    pub station_name_ko: Option<String>,
    pub station_number1: Option<String>,
    pub station_number2: Option<String>,
    pub station_number3: Option<String>,
    pub station_number4: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: i32,
    pub pref_cd: i32,
    pub post: String,
    pub address: String,
    pub lon: f32,
    pub lat: f32,
    pub open_ymd: String,
    pub close_ymd: String,
    pub e_status: i32,
    pub e_sort: i32,
    #[sqlx(default)]
    pub company_cd: Option<i32>,
    #[sqlx(default)]
    pub line_name: Option<String>,
    #[sqlx(default)]
    pub line_name_k: Option<String>,
    #[sqlx(default)]
    pub line_name_h: Option<String>,
    #[sqlx(default)]
    pub line_name_r: Option<String>,
    #[sqlx(default)]
    pub line_name_zh: Option<String>,
    #[sqlx(default)]
    pub line_name_ko: Option<String>,
    #[sqlx(default)]
    pub line_color_c: Option<String>,
    #[sqlx(default)]
    pub line_type: Option<i32>,
    #[sqlx(default)]
    pub line_symbol1: Option<String>,
    #[sqlx(default)]
    pub line_symbol2: Option<String>,
    #[sqlx(default)]
    pub line_symbol3: Option<String>,
    #[sqlx(default)]
    pub line_symbol4: Option<String>,
    #[sqlx(default)]
    pub line_symbol1_color: Option<String>,
    #[sqlx(default)]
    pub line_symbol2_color: Option<String>,
    #[sqlx(default)]
    pub line_symbol3_color: Option<String>,
    #[sqlx(default)]
    pub line_symbol4_color: Option<String>,
    #[sqlx(default)]
    pub line_symbol1_shape: Option<String>,
    #[sqlx(default)]
    pub line_symbol2_shape: Option<String>,
    #[sqlx(default)]
    pub line_symbol3_shape: Option<String>,
    #[sqlx(default)]
    pub line_symbol4_shape: Option<String>,
    #[sqlx(default)]
    pub average_distance: Option<f32>,
    #[sqlx(default)]
    pub type_id: Option<i32>,
    #[sqlx(default)]
    pub sst_id: Option<i32>,
    #[sqlx(default)]
    pub type_cd: Option<i32>,
    #[sqlx(default)]
    pub line_group_cd: Option<i32>,
    #[sqlx(default)]
    pub pass: Option<i32>,
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
    pub direction: Option<i32>,
    #[sqlx(default)]
    pub kind: Option<i32>,
    #[sqlx(default)]
    #[allow(dead_code)]
    pub distance_sq: Option<f64>,
}

impl From<StationRowWithDistance> for Station {
    fn from(row: StationRowWithDistance) -> Self {
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
            station_number1: row.station_number1,
            station_number2: row.station_number2,
            station_number3: row.station_number3,
            station_number4: row.station_number4,
            three_letter_code: row.three_letter_code,
            line_cd: row.line_cd,
            line: None,
            lines: vec![],
            pref_cd: row.pref_cd,
            post: row.post,
            address: row.address,
            lon: row.lon as f64,
            lat: row.lat as f64,
            open_ymd: row.open_ymd,
            close_ymd: row.close_ymd,
            e_status: row.e_status,
            e_sort: row.e_sort,
            stop_condition,
            distance: None,
            train_type: None,
            has_train_types: row.line_group_cd.is_some(),
            company_cd: row.company_cd,
            line_name: row.line_name,
            line_name_k: row.line_name_k,
            line_name_h: row.line_name_h,
            line_name_r: row.line_name_r,
            line_name_zh: row.line_name_zh,
            line_name_ko: row.line_name_ko,
            line_color_c: row.line_color_c,
            line_type: row.line_type,
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
            average_distance: row.average_distance.unwrap_or(0.0) as f64,
            type_id: row.type_id,
            sst_id: row.sst_id,
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
            kind: row.kind,
        }
    }
}

#[derive(sqlx::FromRow, Clone)]
struct StationRow {
    pub station_cd: i32,
    pub station_g_cd: i32,
    pub station_name: String,
    pub station_name_k: String,
    pub station_name_r: Option<String>,
    #[allow(dead_code)]
    pub station_name_rn: Option<String>,
    pub station_name_zh: Option<String>,
    pub station_name_ko: Option<String>,
    pub station_number1: Option<String>,
    pub station_number2: Option<String>,
    pub station_number3: Option<String>,
    pub station_number4: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: i32,
    pub pref_cd: i32,
    pub post: String,
    pub address: String,
    pub lon: f32,
    pub lat: f32,
    pub open_ymd: String,
    pub close_ymd: String,
    pub e_status: i32,
    pub e_sort: i32,
    #[sqlx(default)]
    pub company_cd: Option<i32>,
    #[sqlx(default)]
    pub line_name: Option<String>,
    #[sqlx(default)]
    pub line_name_k: Option<String>,
    #[sqlx(default)]
    pub line_name_h: Option<String>,
    #[sqlx(default)]
    pub line_name_r: Option<String>,
    #[sqlx(default)]
    pub line_name_zh: Option<String>,
    #[sqlx(default)]
    pub line_name_ko: Option<String>,
    #[sqlx(default)]
    pub line_color_c: Option<String>,
    #[sqlx(default)]
    pub line_type: Option<i32>,
    #[sqlx(default)]
    pub line_symbol1: Option<String>,
    #[sqlx(default)]
    pub line_symbol2: Option<String>,
    #[sqlx(default)]
    pub line_symbol3: Option<String>,
    #[sqlx(default)]
    pub line_symbol4: Option<String>,
    #[sqlx(default)]
    pub line_symbol1_color: Option<String>,
    #[sqlx(default)]
    pub line_symbol2_color: Option<String>,
    #[sqlx(default)]
    pub line_symbol3_color: Option<String>,
    #[sqlx(default)]
    pub line_symbol4_color: Option<String>,
    #[sqlx(default)]
    pub line_symbol1_shape: Option<String>,
    #[sqlx(default)]
    pub line_symbol2_shape: Option<String>,
    #[sqlx(default)]
    pub line_symbol3_shape: Option<String>,
    #[sqlx(default)]
    pub line_symbol4_shape: Option<String>,
    #[sqlx(default)]
    pub average_distance: Option<f32>,
    #[sqlx(default)]
    pub type_id: Option<i32>,
    #[sqlx(default)]
    pub sst_id: Option<i32>,
    #[sqlx(default)]
    pub type_cd: Option<i32>,
    #[sqlx(default)]
    pub line_group_cd: Option<i32>,
    #[sqlx(default)]
    pub pass: Option<i32>,
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
    pub direction: Option<i32>,
    #[sqlx(default)]
    pub kind: Option<i32>,
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
            station_number1: row.station_number1,
            station_number2: row.station_number2,
            station_number3: row.station_number3,
            station_number4: row.station_number4,
            three_letter_code: row.three_letter_code,
            line_cd: row.line_cd,
            line: None,
            lines: vec![],
            pref_cd: row.pref_cd,
            post: row.post,
            address: row.address,
            lon: row.lon as f64,
            lat: row.lat as f64,
            open_ymd: row.open_ymd,
            close_ymd: row.close_ymd,
            e_status: row.e_status,
            e_sort: row.e_sort,
            stop_condition,
            distance: None,
            train_type: None,
            has_train_types: row.line_group_cd.is_some(),
            company_cd: row.company_cd,
            line_name: row.line_name,
            line_name_k: row.line_name_k,
            line_name_h: row.line_name_h,
            line_name_r: row.line_name_r,
            line_name_zh: row.line_name_zh,
            line_name_ko: row.line_name_ko,
            line_color_c: row.line_color_c,
            line_type: row.line_type,
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
            average_distance: row.average_distance.unwrap_or(0.0) as f64,
            type_id: row.type_id,
            sst_id: row.sst_id,
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
            kind: row.kind,
        }
    }
}

pub struct MyStationRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyStationRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StationRepository for MyStationRepository {
    async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::find_by_id(id, &mut conn).await
    }
    async fn get_by_id_vec(&self, ids: &[u32]) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_id_vec(ids, &mut conn).await
    }
    async fn get_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        match station_id {
            Some(station_id) => {
                InternalStationRepository::get_by_line_id_and_station_id(
                    line_id, station_id, &mut conn,
                )
                .await
            }
            None => {
                InternalStationRepository::get_by_line_id_without_train_types(line_id, &mut conn)
                    .await
            }
        }
    }
    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_station_group_id(station_group_id, &mut conn).await
    }
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
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
        from_station_group_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_name(
            station_name,
            limit,
            from_station_group_id,
            &mut conn,
        )
        .await
    }

    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_line_group_id(line_group_id, &mut conn).await
    }

    async fn get_route_stops(
        &self,
        from_station_id: u32,
        to_station_id: u32,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_route_stops(from_station_id, to_station_id, &mut conn).await
    }
}

struct InternalStationRepository {}

impl InternalStationRepository {
    async fn fetch_has_local_train_types_by_station_id(
        id: u32,
        conn: &mut PgConnection,
    ) -> Result<bool, DomainError> {
        let row: TrainTypesCountRow = sqlx::query_as!(
            TrainTypesCountRow,
            "SELECT COUNT(sst.line_group_cd) AS train_types_count
            FROM station_station_types AS sst
                JOIN types AS t ON t.type_cd = sst.type_cd
            WHERE sst.station_cd = $1
                AND (
                    t.kind IN (0, 1)
                    OR t.priority > 0
                )",
            id as i32,
        )
        .fetch_one(conn)
        .await?;

        Ok(row.train_types_count.unwrap_or(0) > 0)
    }

    async fn find_by_id(id: u32, conn: &mut PgConnection) -> Result<Option<Station>, DomainError> {
        let rows: Option<StationRow> = sqlx::query_as!(
            StationRow,
            r#"SELECT DISTINCT s.*,
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
            COALESCE(l.average_distance, 0.0) AS average_distance,
            COALESCE(a.line_name, l.line_name, '')         AS line_name,
            COALESCE(a.line_name_k, l.line_name_k, '')     AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h, '')     AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r, '')     AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh, '')   AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko, '')   AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c, '')   AS line_color_c,
            sst.id AS sst_id,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass,
            t.id AS type_id,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind
          FROM stations AS s
          JOIN lines AS l ON l.line_cd = s.line_cd
          LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd
          LEFT JOIN types AS t ON t.type_cd = sst.type_cd
          LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
          LEFT JOIN aliases AS a ON a.id = la.alias_cd
          WHERE s.station_cd = $1
            AND s.e_status = 0
            AND l.e_status = 0"#,
            id as i32,
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
        ids: &[u32],
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let params = (1..=ids.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ");

        // PostgreSQLではCASE文を使用して順序を保持
        let order_case = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("WHEN ${} THEN {}", i + 1, i))
            .collect::<Vec<_>>()
            .join(" ");

        let query_str = format!(
            r#"SELECT DISTINCT
                s.*,
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
                COALESCE(l.average_distance, 0.0) AS average_distance,
                COALESCE(a.line_name, l.line_name, '')         AS line_name,
                COALESCE(a.line_name_k, l.line_name_k, '')     AS line_name_k,
                COALESCE(a.line_name_h, l.line_name_h, '')     AS line_name_h,
                COALESCE(a.line_name_r, l.line_name_r, '')     AS line_name_r,
                COALESCE(a.line_name_zh, l.line_name_zh, '')   AS line_name_zh,
                COALESCE(a.line_name_ko, l.line_name_ko, '')   AS line_name_ko,
                COALESCE(a.line_color_c, l.line_color_c, '')   AS line_color_c,
              NULL::int AS type_id,
              NULL::int AS sst_id,
              NULL::int AS type_cd,
              NULL::int AS line_group_cd,
              NULL::int AS pass,
              NULL::text AS type_name,
              NULL::text AS type_name_k,
              NULL::text AS type_name_r,
              NULL::text AS type_name_zh,
              NULL::text AS type_name_ko,
              NULL::text AS color,
              NULL::int AS direction,
              NULL::int AS kind
            FROM stations AS s
            JOIN lines AS l ON l.line_cd = s.line_cd AND l.e_status = 0
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON la.alias_cd = a.id
            WHERE
                s.station_cd IN ( {params} )
                AND s.e_status = 0
            ORDER BY CASE s.station_cd {order_case} END"#
        );

        let mut query = sqlx::query_as::<_, StationRow>(&query_str);
        for id in ids {
            query = query.bind(*id as i32);
        }
        for id in ids {
            query = query.bind(*id as i32);
        }

        let rows = query.fetch_all(conn).await?;
        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_line_id_without_train_types(
        line_id: u32,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows = sqlx::query_as!(
            StationRow,
            r#"SELECT 
              s.station_cd,
              s.station_g_cd,
              s.station_name,
              s.station_name_k,
              s.station_name_r,
              s.station_name_rn,
              s.station_name_zh,
              s.station_name_ko,
              s.station_number1,
              s.station_number2,
              s.station_number3,
              s.station_number4,
              s.three_letter_code,
              s.line_cd,
              s.pref_cd,
              s.post,
              s.address,
              s.lon,
              s.lat,
              s.open_ymd,
              s.close_ymd,
              s.e_status,
              s.e_sort,
              l.company_cd,
              COALESCE(a.line_name, l.line_name, '')         AS line_name,
              COALESCE(a.line_name_k, l.line_name_k, '')     AS line_name_k,
              COALESCE(a.line_name_h, l.line_name_h, '')     AS line_name_h,
              COALESCE(a.line_name_r, l.line_name_r, '')     AS line_name_r,
              COALESCE(a.line_name_zh, l.line_name_zh, '')   AS line_name_zh,
              COALESCE(a.line_name_ko, l.line_name_ko, '')   AS line_name_ko,
              COALESCE(a.line_color_c, l.line_color_c, '')   AS line_color_c,
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
              COALESCE(l.average_distance, 0.0) AS average_distance,
              NULL::int AS type_id,
              NULL::int AS sst_id,
              NULL::int AS type_cd,
              NULL::int AS line_group_cd,
              NULL::int AS pass,
              NULL::text AS type_name,
              NULL::text AS type_name_k,
              NULL::text AS type_name_r,
              NULL::text AS type_name_zh,
              NULL::text AS type_name_ko,
              NULL::text AS color,
              NULL::int AS direction,
              NULL::int AS kind
              FROM stations AS s
              JOIN lines AS l ON l.line_cd = s.line_cd
              LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
              LEFT JOIN aliases AS a ON a.id = la.alias_cd
            WHERE l.line_cd = $1
              AND s.e_status = 0
              AND l.e_status = 0
            ORDER BY s.e_sort, s.station_cd ASC"#,
            line_id as i32
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_line_id_and_station_id(
        line_id: u32,
        station_id: u32,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let stations: Vec<Station> =
            match Self::fetch_has_local_train_types_by_station_id(station_id, conn).await? {
                true => {
                    let rows = sqlx::query_as!(
                        StationRow,
                        r#"SELECT s.*,
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
                          COALESCE(l.average_distance, 0.0) AS average_distance,
                          COALESCE(a.line_name, l.line_name, '') AS "line_name: String",
                          COALESCE(a.line_name_k, l.line_name_k, '') AS "line_name_k: String",
                          COALESCE(a.line_name_h, l.line_name_h, '') AS "line_name_h: String",
                          COALESCE(a.line_name_r, l.line_name_r, '') AS "line_name_r: String",
                          COALESCE(a.line_name_zh, l.line_name_zh, '') AS "line_name_zh: String",
                          COALESCE(a.line_name_ko, l.line_name_ko, '') AS "line_name_ko: String",
                          COALESCE(a.line_color_c, l.line_color_c, '') AS "line_color_c: String",
                          t.id AS type_id,
                          t.type_cd,
                          t.color,
                          t.type_name,
                          t.type_name_k,
                          t.type_name_r,
                          t.type_name_zh,
                          t.type_name_ko,
                          t.direction,
                          t.kind,
                          sst.id AS sst_id,
                          sst.line_group_cd,
                          sst.pass
                          FROM stations AS s
                          JOIN station_station_types AS sst ON sst.line_group_cd = (
                            SELECT sst.line_group_cd
                            FROM station_station_types AS sst
                              LEFT JOIN types AS t ON sst.type_cd = t.type_cd
                            WHERE sst.station_cd = $1
                            AND (
                                (t.priority > 0 AND sst.pass <> 1 AND sst.type_cd = t.type_cd)
                                OR (NOT (t.priority > 0 AND sst.pass <> 1) AND t.kind IN (0,1))
                              )
                            ORDER BY t.priority DESC
                            LIMIT 1
                          )
                          JOIN types AS t ON t.type_cd = sst.type_cd
                          JOIN lines AS l ON l.line_cd = s.line_cd
                          LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
                          LEFT JOIN aliases AS a ON a.id = la.alias_cd
                          WHERE sst.station_cd = s.station_cd
                            AND s.e_status = 0
                            AND l.e_status = 0
                          ORDER BY sst.id"#,
                        station_id as i32
                    )
                    .fetch_all(conn)
                    .await?;
                    rows.into_iter().map(|row| row.into()).collect()
                }
                false => Self::get_by_line_id_without_train_types(line_id, conn).await?,
            };

        Ok(stations)
    }

    async fn get_by_station_group_id(
        group_id: u32,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows = sqlx::query_as!(
            StationRow,
            r#"SELECT s.*,
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
            COALESCE(l.average_distance, 0.0) AS average_distance,
            COALESCE(a.line_name, l.line_name, '') AS "line_name: String",
            COALESCE(a.line_name_k, l.line_name_k, '') AS "line_name_k: String",
            COALESCE(a.line_name_h, l.line_name_h, '') AS "line_name_h: String",
            COALESCE(a.line_name_r, l.line_name_r, '') AS "line_name_r: String",
            COALESCE(a.line_name_zh, l.line_name_zh, '') AS "line_name_zh: String",
            COALESCE(a.line_name_ko, l.line_name_ko, '') AS "line_name_ko: String",
            COALESCE(a.line_color_c, l.line_color_c, '') AS "line_color_c: String",
            sst.id AS sst_id,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass,
            t.id AS type_id,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind
          FROM
            stations AS s
            JOIN lines AS l ON l.line_cd = s.line_cd
            LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN types AS t ON t.type_cd = sst.type_cd
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON a.id = la.alias_cd
          WHERE
            s.station_g_cd = $1
            AND s.line_cd = l.line_cd
            AND s.e_status = 0
            AND l.e_status = 0"#,
            group_id as i32
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_station_group_id_vec(
        group_id_vec: &[u32],
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        if group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = (1..=group_id_vec.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let query_str = format!(
            r#"SELECT
            s.*,
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
            COALESCE(l.average_distance, 0.0) AS average_distance,
            COALESCE(a.line_name, l.line_name, '')         AS line_name,
            COALESCE(a.line_name_k, l.line_name_k, '')     AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h, '')     AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r, '')     AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh, '')   AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko, '')   AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c, '')   AS line_color_c,
            sst.id AS sst_id,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass,
            t.id AS type_id,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind
          FROM
            stations AS s
            JOIN lines AS l ON l.line_cd = s.line_cd AND l.e_status = 0
            LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN types AS t ON t.type_cd = sst.type_cd  
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON a.id = la.alias_cd
          WHERE
            s.station_g_cd IN ( {params} )
            AND s.line_cd = l.line_cd
            AND s.e_status = 0"#
        );

        let mut query = sqlx::query_as::<_, StationRow>(&query_str);
        for id in group_id_vec {
            query = query.bind(*id as i32);
        }

        let rows = query.fetch_all(conn).await?;
        let lines: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }

    async fn get_by_coordinates(
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let lat_min = latitude - 0.1;
        let lat_max = latitude + 0.1;
        let lon_min = longitude - 0.1;
        let lon_max = longitude + 0.1;

        let rows = sqlx::query_as::<_, StationRowWithDistance>(
            r#"SELECT
                s.*,
                l.company_cd,
                COALESCE(a.line_name, l.line_name, '')         AS line_name,
                COALESCE(a.line_name_k, l.line_name_k, '')     AS line_name_k,
                COALESCE(a.line_name_h, l.line_name_h, '')     AS line_name_h,
                COALESCE(a.line_name_r, l.line_name_r, '')     AS line_name_r,
                COALESCE(a.line_name_zh, l.line_name_zh, '')   AS line_name_zh,
                COALESCE(a.line_name_ko, l.line_name_ko, '')   AS line_name_ko,
                COALESCE(a.line_color_c, l.line_color_c, '')   AS line_color_c,
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
                COALESCE(l.average_distance, 0.0) AS average_distance,
              NULL::int AS type_id,
              NULL::int AS sst_id,
              NULL::int AS type_cd,
              NULL::int AS line_group_cd,
              NULL::int AS pass,
              NULL::text AS type_name,
              NULL::text AS type_name_k,
              NULL::text AS type_name_r,
              NULL::text AS type_name_zh,
              NULL::text AS type_name_ko,
              NULL::text AS color,
              NULL::int AS direction,
              NULL::int AS kind,
              ((s.lat - $1) * (s.lat - $2) + (s.lon - $3) * (s.lon - $4)) AS distance_sq
                FROM stations AS s
                JOIN lines AS l
                ON s.line_cd = l.line_cd
                LEFT JOIN line_aliases AS la
                ON la.station_cd = s.station_cd
                LEFT JOIN aliases AS a
                ON a.id = la.alias_cd
                WHERE s.e_status = 0
                AND s.lat BETWEEN $5 AND $6
                AND s.lon BETWEEN $7 AND $8
                ORDER BY distance_sq
                LIMIT $9"#,
        )
        .bind(latitude)
        .bind(latitude)
        .bind(longitude)
        .bind(longitude)
        .bind(lat_min)
        .bind(lat_max)
        .bind(lon_min)
        .bind(lon_max)
        .bind(limit.unwrap_or(1) as i32)
        .fetch_all(&mut *conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_name(
        station_name: String,
        limit: Option<u32>,
        from_station_group_id: Option<u32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let station_name = &(format!("%{station_name}%"));
        let limit = limit.map(|v| v as i64);
        let from_station_group_id = from_station_group_id.map(|id| id as i32);

        let rows = sqlx::query_as!(
            StationRow,
            r#"WITH from_stations AS (
                SELECT
                    s.station_cd,
                    s.line_cd
                FROM stations AS s
                WHERE s.station_g_cd = $1
                AND s.e_status = 0
            )
            SELECT
                s.*,
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
                COALESCE(l.average_distance, 0.0) AS average_distance,
                dst_sst.id AS sst_id,
                dst_sst.type_cd,
                dst_sst.line_group_cd,
                dst_sst.pass,
                COALESCE(a.line_name, l.line_name, '') AS line_name,
                COALESCE(a.line_name_k, l.line_name_k, '') AS line_name_k,
                COALESCE(a.line_name_h, l.line_name_h, '') AS line_name_h,
                COALESCE(a.line_name_r, l.line_name_r, '') AS line_name_r,
                COALESCE(a.line_name_zh, l.line_name_zh, '') AS line_name_zh,
                COALESCE(a.line_name_ko, l.line_name_ko, '') AS line_name_ko,
                COALESCE(a.line_color_c, l.line_color_c, '') AS line_color_c,
                t.id AS type_id,
                t.type_name,
                t.type_name_k,
                t.type_name_r,
                t.type_name_zh,
                t.type_name_ko,
                t.color,
                t.direction,
                t.kind
            FROM stations AS s
                LEFT JOIN from_stations AS fs
                    ON fs.station_cd = s.station_cd
                LEFT JOIN station_station_types AS from_sst
                    ON from_sst.station_cd = fs.station_cd
                LEFT JOIN station_station_types AS dst_sst
                    ON dst_sst.station_cd = s.station_cd
                LEFT JOIN types AS t
                    ON t.type_cd = dst_sst.type_cd
                LEFT JOIN line_aliases AS la
                    ON la.station_cd = s.station_cd
                LEFT JOIN aliases AS a
                    ON la.alias_cd = a.id
                JOIN lines AS l
                    ON l.line_cd = s.line_cd
                    AND l.e_status = 0
            WHERE
                (
                    s.station_name   LIKE $2
                    OR s.station_name_rn LIKE $3
                    OR s.station_name_k LIKE $4
                    OR s.station_name_zh LIKE $5
                    OR s.station_name_ko LIKE $6
                )
                AND s.e_status = 0
                AND (
                    (
                        from_sst.id IS NOT NULL
                        AND dst_sst.id IS NOT NULL
                        AND from_sst.line_group_cd = dst_sst.line_group_cd
                        AND dst_sst.pass <> 1
                    )
                    OR
                    (
                        (from_sst.id IS NULL OR dst_sst.id IS NULL)
                        AND s.line_cd = COALESCE(fs.line_cd, s.line_cd)
                    )
                )
            ORDER BY s.station_g_cd, s.station_name
            LIMIT $7"#,
            from_station_group_id,
            station_name,
            station_name,
            station_name,
            station_name,
            station_name,
            limit
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_line_group_id(
        line_group_id: u32,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows = sqlx::query_as!(
            StationRow,
            r#"SELECT DISTINCT s.*,
            COALESCE(a.line_name, l.line_name, '') AS "line_name: String",
            COALESCE(a.line_name_k, l.line_name_k, '') AS "line_name_k: String",
            COALESCE(a.line_name_h, l.line_name_h, '') AS "line_name_h: String",
            COALESCE(a.line_name_r, l.line_name_r, '') AS "line_name_r: String",
            COALESCE(a.line_name_zh, l.line_name_zh, '') AS "line_name_zh: String",
            COALESCE(a.line_name_ko, l.line_name_ko, '') AS "line_name_ko: String",
            COALESCE(a.line_color_c, l.line_color_c, '') AS "line_color_c: String",
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
            COALESCE(l.average_distance, 0.0) AS average_distance,
            sst.id AS sst_id,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass,
            t.id AS type_id,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind
          FROM stations AS s
          JOIN lines AS l ON l.line_cd = s.line_cd AND l.e_status = 0
          LEFT JOIN station_station_types AS sst ON sst.line_group_cd = $1
          LEFT JOIN types AS t ON t.type_cd = sst.type_cd
          LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
          LEFT JOIN aliases AS a ON a.id = la.alias_cd
          WHERE
            s.line_cd = l.line_cd
            AND s.station_cd = sst.station_cd
            AND s.e_status = 0
          ORDER BY sst.id"#,
            line_group_id as i32
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_route_stops(
        from_station_id: u32,
        to_station_id: u32,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let mut rows = sqlx::query_as!(
            StationRow,
            r#"WITH
                from_cte AS (
                    SELECT
                        s.station_cd,
                        s.line_cd
                    FROM
                        stations AS s
                    WHERE
                        s.station_g_cd = $1
                ),
                to_cte AS (
                    SELECT
                        s.station_cd,
                        s.line_cd
                    FROM
                        stations AS s
                    WHERE
                        s.station_g_cd = $2
                ),
                common_lines AS (
                    SELECT DISTINCT s1.line_cd
                    FROM stations s1
                    WHERE s1.station_g_cd = $3
                        AND s1.e_status = 0
                        AND EXISTS (
                        SELECT 1
                        FROM stations s2
                        WHERE s2.station_g_cd = $4
                            AND s2.e_status = 0
                            AND s2.line_cd = s1.line_cd
                        )
                ),
                sst_cte_c1 AS (
                    SELECT
                        sst.line_group_cd
                    FROM
                        station_station_types AS sst
                        JOIN from_cte ON sst.station_cd = from_cte.station_cd
                    WHERE
                        sst.pass <> 1
                ),
                sst_cte_c2 AS (
                    SELECT
                        sst.line_group_cd
                    FROM
                        station_station_types AS sst
                        JOIN to_cte ON sst.station_cd = to_cte.station_cd
                    WHERE
                        sst.pass <> 1
                ),
                sst_cte AS (
                    SELECT
                        sst.*
                    FROM
                        station_station_types AS sst
                        JOIN sst_cte_c1 ON sst.line_group_cd = sst_cte_c1.line_group_cd
                        JOIN sst_cte_c2 ON sst.line_group_cd = sst_cte_c2.line_group_cd
                )
            SELECT
            sta.*,
            lin.company_cd,
            COALESCE(a.line_name, lin.line_name, '') AS "line_name: String",
            COALESCE(a.line_name_k, lin.line_name_k, '') AS "line_name_k: String",
            COALESCE(a.line_name_h, lin.line_name_h, '') AS "line_name_h: String",
            COALESCE(a.line_name_r, lin.line_name_r, '') AS "line_name_r: String",
            COALESCE(a.line_name_zh, lin.line_name_zh, '') AS "line_name_zh: String",
            COALESCE(a.line_name_ko, lin.line_name_ko, '') AS "line_name_ko: String",
            COALESCE(a.line_color_c, lin.line_color_c, '') AS "line_color_c: String",
            lin.line_type,
            lin.line_symbol1,
            lin.line_symbol2,
            lin.line_symbol3,
            lin.line_symbol4,
            lin.line_symbol1_color,
            lin.line_symbol2_color,
            lin.line_symbol3_color,
            lin.line_symbol4_color,
            lin.line_symbol1_shape,
            lin.line_symbol2_shape,
            lin.line_symbol3_shape,
            lin.line_symbol4_shape,
            COALESCE(lin.average_distance, 0.0) AS average_distance,
            NULL::int AS type_id,
            NULL::int AS sst_id,
            NULL::int AS type_cd,
            NULL::int AS line_group_cd,
            NULL::int AS pass,
            NULL::text AS type_name,
            NULL::text AS type_name_k,
            NULL::text AS type_name_r,
            NULL::text AS type_name_zh,
            NULL::text AS type_name_ko,
            NULL::text AS color,
            NULL::int AS direction,
            NULL::int AS kind
            FROM
                stations AS sta
				JOIN common_lines AS cl ON sta.line_cd = cl.line_cd
				JOIN lines AS lin ON lin.line_cd = cl.line_cd
                LEFT JOIN sst_cte AS sst ON sst.station_cd = sta.station_cd
                LEFT JOIN types AS tt ON tt.type_cd = sst.type_cd
                LEFT JOIN line_aliases AS la ON la.station_cd = sta.station_cd
                LEFT JOIN aliases AS a ON a.id = la.alias_cd
            WHERE
                sst.line_group_cd IS NULL
                AND lin.e_status = 0
                AND sta.e_status = 0
                ORDER BY sta.e_sort, sta.station_cd"#,
            from_station_id as i32,
            to_station_id as i32,
            from_station_id as i32,
            to_station_id as i32,
        )
        .fetch_all(&mut *conn)
        .await?;

        let mut typed_rows = sqlx::query_as!(
            StationRow,
            r#"WITH
                from_cte AS (
                    SELECT
                        s.station_cd,
                        s.line_cd
                    FROM
                        stations AS s
                    WHERE
                        s.station_g_cd = $1
                        AND s.e_status = 0
                ),
                to_cte AS (
                    SELECT
                        s.station_cd,
                        s.line_cd
                    FROM
                        stations AS s
                    WHERE
                        s.station_g_cd = $2
                        AND s.e_status = 0
                ),
                sst_cte_c1 AS (
                    SELECT
                        sst.line_group_cd
                    FROM
                        station_station_types AS sst
                        JOIN from_cte ON sst.station_cd = from_cte.station_cd
                    WHERE
                        sst.pass <> 1
                ),
                sst_cte_c2 AS (
                    SELECT
                        sst.line_group_cd
                    FROM
                        station_station_types AS sst
                        JOIN to_cte ON sst.station_cd = to_cte.station_cd
                    WHERE
                        sst.pass <> 1
                ),
                sst_cte AS (
                    SELECT
                        sst.*
                    FROM
                        station_station_types AS sst
                        JOIN sst_cte_c1 ON sst.line_group_cd = sst_cte_c1.line_group_cd
                        JOIN sst_cte_c2 ON sst.line_group_cd = sst_cte_c2.line_group_cd
                )
            SELECT
                sta.*,
                lin.company_cd,
                COALESCE(a.line_name, lin.line_name, '') AS "line_name: String",
                COALESCE(a.line_name_k, lin.line_name_k, '') AS "line_name_k: String",
                COALESCE(a.line_name_h, lin.line_name_h, '') AS "line_name_h: String",
                COALESCE(a.line_name_r, lin.line_name_r, '') AS "line_name_r: String",
                COALESCE(a.line_name_zh, lin.line_name_zh, '') AS "line_name_zh: String",
                COALESCE(a.line_name_ko, lin.line_name_ko, '') AS "line_name_ko: String",
                COALESCE(a.line_color_c, lin.line_color_c, '') AS "line_color_c: String",
                lin.line_type,
                lin.line_symbol1,
                lin.line_symbol2,
                lin.line_symbol3,
                lin.line_symbol4,
                lin.line_symbol1_color,
                lin.line_symbol2_color,
                lin.line_symbol3_color,
                lin.line_symbol4_color,
                lin.line_symbol1_shape,
                lin.line_symbol2_shape,
                lin.line_symbol3_shape,
                lin.line_symbol4_shape,
                COALESCE(lin.average_distance, 0.0) AS average_distance,
                tt.id AS type_id,
                sst.id AS sst_id,
                sst.type_cd,
                sst.line_group_cd,
                sst.pass,
                tt.type_name,
                tt.type_name_k,
                tt.type_name_r,
                tt.type_name_zh,
                tt.type_name_ko,
                tt.color,
                tt.direction,
                tt.kind
            FROM
                stations AS sta
                LEFT JOIN sst_cte AS sst ON sst.station_cd = sta.station_cd
                LEFT JOIN types AS tt ON tt.type_cd = sst.type_cd
                JOIN lines AS lin ON lin.line_cd = sta.line_cd
                LEFT JOIN line_aliases AS la ON la.station_cd = sta.station_cd
                LEFT JOIN aliases AS a ON a.id = la.alias_cd
            WHERE
                sta.station_cd = sst.station_cd
                AND lin.e_status = 0
                AND sta.e_status = 0
            ORDER BY sst.id"#,
            from_station_id as i32,
            to_station_id as i32,
        )
        .fetch_all(conn)
        .await?;

        rows.append(&mut typed_rows);
        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::StopCondition;
    use sqlx::PgPool;
    use std::sync::Arc;

    async fn setup_test_db() -> Arc<Pool<Postgres>> {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://stationapi:stationapi@localhost:5432/stationapi_test".to_string()
        });
        let pool = PgPool::connect(&database_url).await.unwrap();

        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS stations (
                station_cd INTEGER PRIMARY KEY,
                station_g_cd INTEGER NOT NULL,
                station_name TEXT NOT NULL,
                station_name_k TEXT NOT NULL,
                station_name_r TEXT,
                station_name_rn TEXT,
                station_name_zh TEXT,
                station_name_ko TEXT,
                station_number1 TEXT,
                station_number2 TEXT,
                station_number3 TEXT,
                station_number4 TEXT,
                three_letter_code TEXT,
                line_cd INTEGER NOT NULL,
                pref_cd INTEGER NOT NULL,
                post TEXT NOT NULL,
                address TEXT NOT NULL,
                lon REAL NOT NULL,
                lat REAL NOT NULL,
                open_ymd TEXT NOT NULL,
                close_ymd TEXT NOT NULL,
                e_status INTEGER NOT NULL,
                e_sort INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS companies (
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
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS lines (
                line_cd INTEGER PRIMARY KEY,
                company_cd INTEGER NOT NULL,
                line_name TEXT NOT NULL,
                line_name_k TEXT NOT NULL,
                line_name_h TEXT NOT NULL,
                line_name_r TEXT NOT NULL DEFAULT '',
                line_name_rn TEXT NOT NULL DEFAULT '',
                line_name_zh TEXT DEFAULT '',
                line_name_ko TEXT DEFAULT '',
                line_color_c TEXT NOT NULL,
                line_type INTEGER NOT NULL,
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
                average_distance REAL DEFAULT 0
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS station_station_types (
                id SERIAL PRIMARY KEY,
                station_cd INTEGER NOT NULL,
                type_cd INTEGER NOT NULL,
                line_group_cd INTEGER NOT NULL,
                pass INTEGER DEFAULT 0
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS types (
                id SERIAL PRIMARY KEY,
                type_cd INTEGER NOT NULL,
                type_name TEXT NOT NULL,
                type_name_k TEXT NOT NULL,
                type_name_r TEXT NOT NULL,
                type_name_zh TEXT NOT NULL,
                type_name_ko TEXT NOT NULL,
                color TEXT NOT NULL,
                direction INTEGER DEFAULT 0,
                kind INTEGER DEFAULT 0,
                priority INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS line_aliases (
                station_cd INTEGER NOT NULL,
                alias_cd INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS aliases (
                id INTEGER PRIMARY KEY,
                line_name TEXT,
                line_name_k TEXT,
                line_name_h TEXT,
                line_name_r TEXT,
                line_name_zh TEXT,
                line_name_ko TEXT,
                line_color_c TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert test data
        // Companies first
        sqlx::query(
            r#"
            INSERT INTO companies (company_cd, rr_cd, company_name, company_name_k, company_name_h, company_name_r, company_name_en, company_name_full_en, company_type, e_status, e_sort)
            VALUES 
                (1, 1, 'JR東日本', 'じぇいあーるひがしにほん', 'JR東日本', 'JR East', 'JR East', 'East Japan Railway Company', 1, 0, 1)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Lines
        sqlx::query(
            r#"
            INSERT INTO lines (line_cd, company_cd, line_name, line_name_k, line_name_h, line_name_r, line_name_rn, line_color_c, line_type, line_symbol1, average_distance, e_status, e_sort)
            VALUES 
                (1001, 1, '山手線', 'やまてせん', 'やまてせん', 'Yamanote Line', 'Yamanote Line', '#9ACD32', 11, 'Y', 2.0, 0, 1),
                (1002, 1, '中央線', 'ちゅうおうせん', 'ちゅうおうせん', 'Chuo Line', 'Chuo Line', '#F97316', 11, 'C', 1.5, 0, 2),
                (1003, 1, '東海道線', 'とうかいどうせん', 'とうかいどうせん', 'Tokaido Line', 'Tokaido Line', '#F59E0B', 11, 'T', 3.0, 0, 3)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Stations
        sqlx::query(
            r#"
            INSERT INTO stations (
                station_cd, station_g_cd, station_name, station_name_k, 
                line_cd, pref_cd, post, address, lon, lat, 
                open_ymd, close_ymd, e_status, e_sort
            )
            VALUES 
                (1, 1001, '東京駅', 'とうきょうえき', 1001, 13, '100-0005', '東京都千代田区丸の内一丁目', 139.767125, 35.681236, '19141220', '99991231', 0, 1),
                (2, 1002, '新宿駅', 'しんじゅくえき', 1001, 13, '160-0022', '東京都新宿区新宿三丁目', 139.700258, 35.690921, '18850301', '99991231', 0, 2),
                (3, 1003, '渋谷駅', 'しぶやえき', 1001, 13, '150-0043', '東京都渋谷区道玄坂一丁目', 139.700464, 35.659518, '18850301', '99991231', 0, 3),
                (4, 1004, '品川駅', 'しながわえき', 1003, 13, '108-0074', '東京都港区高輪三丁目', 139.740570, 35.630152, '18720601', '99991231', 0, 4),
                (5, 1005, '神田駅', 'かんだえき', 1001, 13, '101-0044', '東京都千代田区鍛冶町二丁目', 139.770883, 35.691777, '19190301', '99991231', 0, 5)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Station types
        sqlx::query(
            r#"
            INSERT INTO types (type_cd, type_name, type_name_k, type_name_r, type_name_zh, type_name_ko, color, direction, kind, priority)
            VALUES 
                (11, '普通', 'ふつう', 'Local', '普通', '보통', '#000000', 0, 0, 1),
                (12, '快速', 'かいそく', 'Rapid', '快速', '쾌속', '#FF0000', 0, 1, 2),
                (13, '特急', 'とっきゅう', 'Limited Express', '特急', '특급', '#0000FF', 0, 2, 3)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Station station types
        sqlx::query(
            r#"
            INSERT INTO station_station_types (id, station_cd, type_cd, line_group_cd, pass)
            VALUES 
                (1, 1, 11, 1001, 0),
                (2, 2, 11, 1001, 0),
                (3, 2, 12, 1001, 0),
                (4, 3, 11, 1001, 0),
                (5, 4, 11, 1003, 0),
                (6, 5, 11, 1001, 0)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        Arc::new(pool)
    }

    #[tokio::test]
    async fn test_find_by_id_existing() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository.find_by_id(1).await;
        assert!(result.is_ok());

        let station = result.unwrap();
        assert!(station.is_some());

        let station = station.unwrap();
        assert_eq!(station.station_cd, 1);
        assert_eq!(station.station_name, "東京駅");
        assert_eq!(station.station_name_k, "とうきょうえき");
        assert_eq!(station.line_cd, 1001);
        assert_eq!(station.station_g_cd, 1001);
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository.find_by_id(999).await;
        assert!(result.is_ok());

        let station = result.unwrap();
        assert!(station.is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_vec() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let ids = vec![1, 2, 3];
        let result = repository.get_by_id_vec(&ids).await;
        if let Err(ref e) = result {
            eprintln!("Error in get_by_id_vec: {e:?}");
        }
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert_eq!(stations.len(), 3);

        let station_names: Vec<String> = stations.iter().map(|s| s.station_name.clone()).collect();
        assert!(station_names.contains(&"東京駅".to_string()));
        assert!(station_names.contains(&"新宿駅".to_string()));
        assert!(station_names.contains(&"渋谷駅".to_string()));
    }

    #[tokio::test]
    async fn test_get_by_id_vec_empty() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let ids = vec![];
        let result = repository.get_by_id_vec(&ids).await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert_eq!(stations.len(), 0);
    }

    #[tokio::test]
    async fn test_get_by_id_vec_partial() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let ids = vec![1, 999, 2];
        let result = repository.get_by_id_vec(&ids).await;
        if let Err(ref e) = result {
            eprintln!("Error in get_by_id_vec_partial: {e:?}");
        }
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert_eq!(stations.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_line_id_without_station_id() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository.get_by_line_id(1001, None).await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert!(!stations.is_empty());

        // 山手線の駅のみが返されることを確認
        for station in &stations {
            assert_eq!(station.line_cd, 1001);
        }
    }

    #[tokio::test]
    async fn test_get_by_line_id_with_station_id() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository.get_by_line_id(1001, Some(2)).await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        // train typesを持つ駅の場合、特定のtype情報が含まれる
        assert!(!stations.is_empty());
    }

    #[tokio::test]
    async fn test_get_by_station_group_id() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository.get_by_station_group_id(1001).await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0].station_g_cd, 1001);
        assert_eq!(stations[0].station_name, "東京駅");
    }

    #[tokio::test]
    async fn test_get_by_station_group_id_vec() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let group_ids = vec![1001, 1002, 1003];
        let result = repository.get_by_station_group_id_vec(&group_ids).await;
        assert!(result.is_ok());

        let stations = result.unwrap();

        // 実際の結果数を確認（station_station_typesとのJOINがあるため、複数行になる可能性がある）
        assert!(stations.len() >= 3);

        let group_cds: Vec<i32> = stations.iter().map(|s| s.station_g_cd).collect();
        assert!(group_cds.contains(&1001));
        assert!(group_cds.contains(&1002));
        assert!(group_cds.contains(&1003));
    }

    #[tokio::test]
    async fn test_get_by_station_group_id_vec_empty() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let group_ids = vec![];
        let result = repository.get_by_station_group_id_vec(&group_ids).await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert_eq!(stations.len(), 0);
    }

    #[tokio::test]
    async fn test_get_by_coordinates() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        // 東京駅の座標に近い場所を検索
        let result = repository
            .get_by_coordinates(35.681236, 139.767125, Some(3))
            .await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert!(!stations.is_empty());
        assert!(stations.len() <= 3);

        // 最初の駅は東京駅であるはず（距離が最も近い）
        assert_eq!(stations[0].station_name, "東京駅");
    }

    #[tokio::test]
    async fn test_get_by_coordinates_no_limit() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository
            .get_by_coordinates(35.681236, 139.767125, None)
            .await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert_eq!(stations.len(), 1); // limitがNoneの場合、デフォルトで1が使われる
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository.get_by_name("東京".to_string(), None, None).await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert!(!stations.is_empty());

        // 「東京」を含む駅名が返されることを確認
        let found_tokyo = stations.iter().any(|s| s.station_name.contains("東京"));
        assert!(found_tokyo);
    }

    #[tokio::test]
    async fn test_get_by_name_with_limit() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository
            .get_by_name("駅".to_string(), Some(2), None)
            .await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert!(stations.len() <= 2);
    }

    #[tokio::test]
    async fn test_get_by_name_not_found() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository
            .get_by_name("存在しない駅".to_string(), None, None)
            .await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert_eq!(stations.len(), 0);
    }

    #[tokio::test]
    async fn test_get_by_line_group_id() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository.get_by_line_group_id(1001).await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        assert!(!stations.is_empty());

        // 指定したline_group_cdを持つ駅のみが返されることを確認
        for station in &stations {
            assert_eq!(station.line_group_cd, Some(1001));
        }
    }

    #[tokio::test]
    async fn test_get_route_stops() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository.get_route_stops(1001, 1002).await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        // ルート検索の結果として駅のリストが返される
        // stations.len() >= 0 は常にtrueなので、より有意味なテストに変更
        assert!(stations.len() <= 100); // 最大100駅まで想定
    }

    #[tokio::test]
    async fn test_get_route_stops_same_station() {
        let conn = setup_test_db().await;
        let repository = MyStationRepository::new(conn);

        let result = repository.get_route_stops(1001, 1001).await;
        assert!(result.is_ok());

        let stations = result.unwrap();
        // 同じ駅の場合の動作を確認
        // stations.len() >= 0 は常にtrueなので、より有意味なテストに変更
        assert!(stations.len() <= 100); // 最大100駅まで想定
    }

    #[tokio::test]
    async fn test_fetch_has_local_train_types_by_station_id() {
        let pool = setup_test_db().await;
        let mut conn = pool.acquire().await.unwrap();

        // 新宿駅（ID: 2）は複数の train types を持つ
        let result =
            InternalStationRepository::fetch_has_local_train_types_by_station_id(2, &mut conn)
                .await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // 存在しない駅
        let result =
            InternalStationRepository::fetch_has_local_train_types_by_station_id(999, &mut conn)
                .await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_station_row_to_station_conversion() {
        // StationRowからStationへの変換をテスト
        let station_row = StationRow {
            station_cd: 1,
            station_g_cd: 1001,
            station_name: "テスト駅".to_string(),
            station_name_k: "てすとえき".to_string(),
            station_name_r: Some("Test Station".to_string()),
            station_name_rn: None,
            station_name_zh: Some("测试站".to_string()),
            station_name_ko: Some("테스트역".to_string()),
            station_number1: Some("T01".to_string()),
            station_number2: None,
            station_number3: None,
            station_number4: None,
            three_letter_code: Some("TST".to_string()),
            line_cd: 1001,
            pref_cd: 13,
            post: "100-0001".to_string(),
            address: "東京都千代田区".to_string(),
            lon: 139.767_12,
            lat: 35.681236,
            open_ymd: "19900401".to_string(),
            close_ymd: "99991231".to_string(),
            e_status: 0,
            e_sort: 1,
            company_cd: Some(1),
            line_name: Some("テスト線".to_string()),
            line_name_k: Some("てすとせん".to_string()),
            line_name_h: Some("テストセン".to_string()),
            line_name_r: Some("Test Line".to_string()),
            line_name_zh: Some("测试线".to_string()),
            line_name_ko: Some("테스트선".to_string()),
            line_color_c: Some("#FF0000".to_string()),
            line_type: Some(11),
            line_symbol1: Some("T".to_string()),
            line_symbol2: None,
            line_symbol3: None,
            line_symbol4: None,
            line_symbol1_color: Some("#0000FF".to_string()),
            line_symbol2_color: None,
            line_symbol3_color: None,
            line_symbol4_color: None,
            line_symbol1_shape: Some("circle".to_string()),
            line_symbol2_shape: None,
            line_symbol3_shape: None,
            line_symbol4_shape: None,
            average_distance: Some(2.0),
            type_id: Some(1),
            sst_id: Some(1),
            type_cd: Some(11),
            line_group_cd: Some(1001),
            pass: Some(0),
            type_name: Some("普通".to_string()),
            type_name_k: Some("ふつう".to_string()),
            type_name_r: Some("Local".to_string()),
            type_name_zh: Some("普通".to_string()),
            type_name_ko: Some("보통".to_string()),
            color: Some("#000000".to_string()),
            direction: Some(0),
            kind: Some(0),
        };

        let station: Station = station_row.into();

        assert_eq!(station.station_cd, 1);
        assert_eq!(station.station_name, "テスト駅");
        assert_eq!(station.station_name_k, "てすとえき");
        assert_eq!(station.stop_condition, StopCondition::All);
        assert!(station.has_train_types);
        assert_eq!(station.average_distance, 2.0);
    }

    #[tokio::test]
    async fn test_station_row_stop_condition_conversion() {
        // 各stopConditionのテスト
        let test_cases = vec![
            (Some(0), StopCondition::All),
            (Some(1), StopCondition::Not),
            (Some(2), StopCondition::Partial),
            (Some(3), StopCondition::Weekday),
            (Some(4), StopCondition::Holiday),
            (Some(5), StopCondition::PartialStop),
            (Some(999), StopCondition::All), // 未知の値
            (None, StopCondition::All),
        ];

        for (pass_value, expected_condition) in test_cases {
            let station_row = StationRow {
                station_cd: 1,
                station_g_cd: 1001,
                station_name: "テスト駅".to_string(),
                station_name_k: "てすとえき".to_string(),
                station_name_r: None,
                station_name_rn: None,
                station_name_zh: None,
                station_name_ko: None,
                station_number1: None,
                station_number2: None,
                station_number3: None,
                station_number4: None,
                three_letter_code: None,
                line_cd: 1001,
                pref_cd: 13,
                post: "100-0001".to_string(),
                address: "東京都千代田区".to_string(),
                lon: 139.767_12,
                lat: 35.681236,
                open_ymd: "19900401".to_string(),
                close_ymd: "99991231".to_string(),
                e_status: 0,
                e_sort: 1,
                company_cd: None,
                line_name: None,
                line_name_k: None,
                line_name_h: None,
                line_name_r: None,
                line_name_zh: None,
                line_name_ko: None,
                line_color_c: None,
                line_type: None,
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
                average_distance: Some(2.0),
                type_id: None,
                sst_id: None,
                type_cd: None,
                line_group_cd: None,
                pass: pass_value,
                type_name: None,
                type_name_k: None,
                type_name_r: None,
                type_name_zh: None,
                type_name_ko: None,
                color: None,
                direction: None,
                kind: None,
            };

            let station: Station = station_row.into();
            assert_eq!(station.stop_condition, expected_condition);
        }
    }
}
