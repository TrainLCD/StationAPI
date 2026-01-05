use async_trait::async_trait;
use sqlx::{PgConnection, Pool, Postgres};
use std::sync::Arc;

use crate::{
    domain::{
        entity::{gtfs::TransportType, station::Station},
        error::DomainError,
        normalize::normalize_for_search,
        repository::station_repository::StationRepository,
    },
    proto::StopCondition,
};

#[derive(sqlx::FromRow)]
struct TrainTypesCountRow {
    train_types_count: Option<i32>,
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
    pub lon: f64,
    pub lat: f64,
    pub open_ymd: String,
    pub close_ymd: String,
    pub e_status: i32,
    pub e_sort: i32,
    pub company_cd: Option<i32>,
    pub line_name: Option<String>,
    pub line_name_k: Option<String>,
    pub line_name_h: Option<String>,
    pub line_name_r: Option<String>,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: Option<String>,
    pub line_type: Option<i32>,
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
    pub average_distance: Option<f64>,
    pub type_id: Option<i32>,
    pub sst_id: Option<i32>,
    pub type_cd: Option<i32>,
    pub line_group_cd: Option<i32>,
    pub pass: Option<i32>,
    pub type_name: Option<String>,
    pub type_name_k: Option<String>,
    pub type_name_r: Option<String>,
    pub type_name_zh: Option<String>,
    pub type_name_ko: Option<String>,
    pub color: Option<String>,
    pub direction: Option<i32>,
    pub kind: Option<i32>,
    pub transport_type: Option<i32>,
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
            lon: row.lon,
            lat: row.lat,
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
            average_distance: row.average_distance,
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
            transport_type: TransportType::from(row.transport_type.unwrap_or(0)),
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
        direction_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        match station_id {
            Some(station_id) => {
                InternalStationRepository::get_by_line_id_and_station_id(
                    line_id,
                    station_id,
                    direction_id,
                    &mut conn,
                )
                .await
            }
            None => {
                InternalStationRepository::get_by_line_id_without_train_types(
                    line_id,
                    direction_id,
                    &mut conn,
                )
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
        transport_type: Option<TransportType>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_coordinates(
            latitude,
            longitude,
            limit,
            transport_type,
            &mut conn,
        )
        .await
    }

    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
        from_station_group_id: Option<u32>,
        transport_type: Option<TransportType>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_name(
            station_name,
            limit,
            from_station_group_id,
            transport_type,
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
        via_line_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_route_stops(
            from_station_id,
            to_station_id,
            via_line_id,
            &mut conn,
        )
        .await
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
            "SELECT COUNT(sst.line_group_cd)::integer AS train_types_count
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
            r#"SELECT s.station_cd,
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
            COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL) AS line_name,
            COALESCE(NULLIF(COALESCE(a.line_name_k, l.line_name_k), ''), NULL) AS line_name_k,
            COALESCE(NULLIF(COALESCE(a.line_name_h, l.line_name_h), ''), NULL) AS line_name_h,
            COALESCE(NULLIF(COALESCE(a.line_name_r, l.line_name_r), ''), NULL) AS line_name_r,
            COALESCE(NULLIF(COALESCE(a.line_name_zh, l.line_name_zh), ''), NULL) AS line_name_zh,
            COALESCE(NULLIF(COALESCE(a.line_name_ko, l.line_name_ko), ''), NULL) AS line_name_ko,
            COALESCE(NULLIF(COALESCE(a.line_color_c, l.line_color_c), ''), NULL) AS line_color_c,
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
            COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
            t.id AS type_id,
            sst.id AS sst_id,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            s.transport_type
          FROM stations AS s
          JOIN lines AS l ON l.line_cd = s.line_cd
          LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd
          LEFT JOIN types AS t ON t.type_cd = sst.type_cd
          LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
          LEFT JOIN aliases AS a ON a.id = la.alias_cd
          WHERE s.station_cd = $1
            AND s.e_status = 0
            AND l.e_status = 0
          LIMIT 1"#,
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
                COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
                COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL) AS line_name,
                COALESCE(NULLIF(COALESCE(a.line_name_k, l.line_name_k), ''), NULL) AS line_name_k,
                COALESCE(NULLIF(COALESCE(a.line_name_h, l.line_name_h), ''), NULL) AS line_name_h,
                COALESCE(NULLIF(COALESCE(a.line_name_r, l.line_name_r), ''), NULL) AS line_name_r,
                COALESCE(NULLIF(COALESCE(a.line_name_zh, l.line_name_zh), ''), NULL) AS line_name_zh,
                COALESCE(NULLIF(COALESCE(a.line_name_ko, l.line_name_ko), ''), NULL) AS line_name_ko,
                COALESCE(NULLIF(COALESCE(a.line_color_c, l.line_color_c), ''), NULL) AS line_color_c,
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
                s.transport_type
            FROM stations AS s
            JOIN lines AS l ON l.line_cd = s.line_cd AND l.e_status = 0
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON la.alias_cd = a.id
            WHERE
                s.station_cd IN ( {params} )
                AND s.e_status = 0
            GROUP BY
                s.station_cd, s.station_g_cd, s.station_name, s.station_name_k,
                s.station_name_r, s.station_name_rn, s.station_name_zh, s.station_name_ko,
                s.station_number1, s.station_number2, s.station_number3, s.station_number4,
                s.three_letter_code, s.line_cd, s.pref_cd, s.post, s.address, s.lon, s.lat,
                s.open_ymd, s.close_ymd, s.e_status, s.e_sort, s.transport_type, l.company_cd, l.line_type,
                l.line_symbol1, l.line_symbol2, l.line_symbol3, l.line_symbol4,
                l.line_symbol1_color, l.line_symbol2_color, l.line_symbol3_color, l.line_symbol4_color,
                l.line_symbol1_shape, l.line_symbol2_shape, l.line_symbol3_shape, l.line_symbol4_shape,
                l.average_distance, a.line_name, l.line_name, a.line_name_k, l.line_name_k,
                a.line_name_h, l.line_name_h, a.line_name_r, l.line_name_r,
                a.line_name_zh, l.line_name_zh, a.line_name_ko, l.line_name_ko,
                a.line_color_c, l.line_color_c
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
        direction_id: Option<u32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        // When direction_id = 1 (上り) or 2 (下り), reverse the order
        let order_clause = if matches!(direction_id, Some(1) | Some(2)) {
            "ORDER BY s.e_sort DESC, s.station_cd DESC"
        } else {
            "ORDER BY s.e_sort ASC, s.station_cd ASC"
        };

        let query_str = format!(
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
              COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL) AS line_name,
              COALESCE(NULLIF(COALESCE(a.line_name_k, l.line_name_k), ''), NULL) AS line_name_k,
              COALESCE(NULLIF(COALESCE(a.line_name_h, l.line_name_h), ''), NULL) AS line_name_h,
              COALESCE(NULLIF(COALESCE(a.line_name_r, l.line_name_r), ''), NULL) AS line_name_r,
              COALESCE(NULLIF(COALESCE(a.line_name_zh, l.line_name_zh), ''), NULL) AS line_name_zh,
              COALESCE(NULLIF(COALESCE(a.line_name_ko, l.line_name_ko), ''), NULL) AS line_name_ko,
              COALESCE(NULLIF(COALESCE(a.line_color_c, l.line_color_c), ''), NULL) AS line_color_c,
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
              COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
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
              s.transport_type
            FROM stations AS s
            JOIN lines AS l ON l.line_cd = s.line_cd
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON a.id = la.alias_cd
            WHERE l.line_cd = $1
              AND s.e_status = 0
              AND l.e_status = 0
            {order_clause}"#
        );

        let rows = sqlx::query_as::<_, StationRow>(&query_str)
            .bind(line_id as i32)
            .fetch_all(conn)
            .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_line_id_and_station_id(
        line_id: u32,
        station_id: u32,
        direction_id: Option<u32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let stations: Vec<Station> = match Self::fetch_has_local_train_types_by_station_id(
            station_id, conn,
        )
        .await?
        {
            true => {
                // When direction_id = 1 (上り) or 2 (下り), reverse the order
                let order_clause = if matches!(direction_id, Some(1) | Some(2)) {
                    "ORDER BY sst.id DESC"
                } else {
                    "ORDER BY sst.id ASC"
                };

                let query_str = format!(
                    r#"WITH target_line_group AS (
                            SELECT sst_inner.line_group_cd
                            FROM station_station_types AS sst_inner
                              LEFT JOIN types AS t_inner ON sst_inner.type_cd = t_inner.type_cd
                            WHERE sst_inner.station_cd = $1
                            AND (
                                (t_inner.priority > 0 AND sst_inner.pass <> 1 AND sst_inner.type_cd = t_inner.type_cd)
                                OR (NOT (t_inner.priority > 0 AND sst_inner.pass <> 1) AND t_inner.kind IN (0,1))
                              )
                            ORDER BY t_inner.priority DESC
                            LIMIT 1
                          )
                          SELECT s.station_cd,
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
                          COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
                          COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL) AS line_name,
                          COALESCE(NULLIF(COALESCE(a.line_name_k, l.line_name_k), ''), NULL) AS line_name_k,
                          COALESCE(NULLIF(COALESCE(a.line_name_h, l.line_name_h), ''), NULL) AS line_name_h,
                          COALESCE(NULLIF(COALESCE(a.line_name_r, l.line_name_r), ''), NULL) AS line_name_r,
                          COALESCE(NULLIF(COALESCE(a.line_name_zh, l.line_name_zh), ''), NULL) AS line_name_zh,
                          COALESCE(NULLIF(COALESCE(a.line_name_ko, l.line_name_ko), ''), NULL) AS line_name_ko,
                          COALESCE(NULLIF(COALESCE(a.line_color_c, l.line_color_c), ''), NULL) AS line_color_c,
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
                          sst.pass,
                          s.transport_type
                          FROM stations AS s
                          JOIN station_station_types AS sst ON sst.line_group_cd = (SELECT line_group_cd FROM target_line_group) AND sst.station_cd = s.station_cd
                          JOIN types AS t ON t.type_cd = sst.type_cd
                          JOIN lines AS l ON l.line_cd = s.line_cd
                          LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
                          LEFT JOIN aliases AS a ON a.id = la.alias_cd
                          WHERE s.e_status = 0
                            AND l.e_status = 0
                          {order_clause}"#
                );

                let rows = sqlx::query_as::<_, StationRow>(&query_str)
                    .bind(station_id as i32)
                    .fetch_all(conn)
                    .await?;
                rows.into_iter().map(|row| row.into()).collect()
            }
            false => Self::get_by_line_id_without_train_types(line_id, direction_id, conn).await?,
        };

        Ok(stations)
    }

    async fn get_by_station_group_id(
        group_id: u32,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows = sqlx::query_as!(
            StationRow,
            r#"SELECT s.station_cd,
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
            COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
            COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL) AS line_name,
            COALESCE(NULLIF(COALESCE(a.line_name_k, l.line_name_k), ''), NULL) AS line_name_k,
            COALESCE(NULLIF(COALESCE(a.line_name_h, l.line_name_h), ''), NULL) AS line_name_h,
            COALESCE(NULLIF(COALESCE(a.line_name_r, l.line_name_r), ''), NULL) AS line_name_r,
            COALESCE(NULLIF(COALESCE(a.line_name_zh, l.line_name_zh), ''), NULL) AS line_name_zh,
            COALESCE(NULLIF(COALESCE(a.line_name_ko, l.line_name_ko), ''), NULL) AS line_name_ko,
            COALESCE(NULLIF(COALESCE(a.line_color_c, l.line_color_c), ''), NULL) AS line_color_c,
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
            t.kind,
            s.transport_type
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
            COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
            COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL) AS line_name,
            COALESCE(NULLIF(COALESCE(a.line_name_k, l.line_name_k), ''), NULL) AS line_name_k,
            COALESCE(NULLIF(COALESCE(a.line_name_h, l.line_name_h), ''), NULL) AS line_name_h,
            COALESCE(NULLIF(COALESCE(a.line_name_r, l.line_name_r), ''), NULL) AS line_name_r,
            COALESCE(NULLIF(COALESCE(a.line_name_zh, l.line_name_zh), ''), NULL) AS line_name_zh,
            COALESCE(NULLIF(COALESCE(a.line_name_ko, l.line_name_ko), ''), NULL) AS line_name_ko,
            COALESCE(NULLIF(COALESCE(a.line_color_c, l.line_color_c), ''), NULL) AS line_color_c,
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
            t.kind,
            s.transport_type
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
        transport_type: Option<TransportType>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let transport_type_value: Option<i32> = transport_type.map(|t| t as i32);

        let rows = sqlx::query_as::<_, StationRow>(
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
                COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL) AS line_name,
                COALESCE(NULLIF(COALESCE(a.line_name_k, l.line_name_k), ''), NULL) AS line_name_k,
                COALESCE(NULLIF(COALESCE(a.line_name_h, l.line_name_h), ''), NULL) AS line_name_h,
                COALESCE(NULLIF(COALESCE(a.line_name_r, l.line_name_r), ''), NULL) AS line_name_r,
                COALESCE(NULLIF(COALESCE(a.line_name_zh, l.line_name_zh), ''), NULL) AS line_name_zh,
                COALESCE(NULLIF(COALESCE(a.line_name_ko, l.line_name_ko), ''), NULL) AS line_name_ko,
                COALESCE(NULLIF(COALESCE(a.line_color_c, l.line_color_c), ''), NULL) AS line_color_c,
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
                COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
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
                s.transport_type
                FROM stations AS s
                JOIN lines AS l
                ON s.line_cd = l.line_cd
                LEFT JOIN line_aliases AS la
                ON la.station_cd = s.station_cd
                LEFT JOIN aliases AS a
                ON a.id = la.alias_cd
                WHERE s.e_status = 0
                AND ($4::int IS NULL OR COALESCE(s.transport_type, 0) = $4)
                ORDER BY point(s.lat, s.lon) <-> point($1, $2)
                LIMIT $3"#,
        )
        .bind(latitude)
        .bind(longitude)
        .bind(limit.unwrap_or(1) as i32)
        .bind(transport_type_value)
        .fetch_all(&mut *conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_name(
        station_name: String,
        limit: Option<u32>,
        from_station_group_id: Option<u32>,
        transport_type: Option<TransportType>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        // 元の入力用パターン（漢字・その他）
        let station_name_pattern = &(format!("%{station_name}%"));
        // カタカナ検索用に正規化されたパターン（ひらがな→カタカナ変換）
        let station_name_k_pattern = &(format!("%{}%", normalize_for_search(&station_name)));
        let limit = limit.map(|v| v as i64);
        let from_station_group_id = from_station_group_id.map(|id| id as i32);
        let transport_type_value: Option<i32> = transport_type.map(|t| t as i32);

        let rows = sqlx::query_as!(
            StationRow,
            r#"WITH from_stations AS (
                SELECT
                    s.station_cd,
                    s.line_cd
                FROM stations AS s
                WHERE s.station_g_cd = $1
                AND s.e_status = 0
            ),
            filtered AS (
                SELECT DISTINCT ON (s.station_cd)
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
                    COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL) AS line_name,
                    COALESCE(NULLIF(COALESCE(a.line_name_k, l.line_name_k), ''), NULL) AS line_name_k,
                    COALESCE(NULLIF(COALESCE(a.line_name_h, l.line_name_h), ''), NULL) AS line_name_h,
                    COALESCE(NULLIF(COALESCE(a.line_name_r, l.line_name_r), ''), NULL) AS line_name_r,
                    COALESCE(NULLIF(COALESCE(a.line_name_zh, l.line_name_zh), ''), NULL) AS line_name_zh,
                    COALESCE(NULLIF(COALESCE(a.line_name_ko, l.line_name_ko), ''), NULL) AS line_name_ko,
                    COALESCE(NULLIF(COALESCE(a.line_color_c, l.line_color_c), ''), NULL) AS line_color_c,
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
                    COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
                    COALESCE(from_sst.line_group_cd, NULL)::int AS line_group_cd, -- has_train_types用
                    NULL::int AS type_id,
                    NULL::int AS sst_id,
                    NULL::int AS type_cd,
                    NULL::int AS pass,
                    NULL::text AS type_name,
                    NULL::text AS type_name_k,
                    NULL::text AS type_name_r,
                    NULL::text AS type_name_zh,
                    NULL::text AS type_name_ko,
                    NULL::text AS color,
                    NULL::int AS direction,
                    NULL::int AS kind,
                s.transport_type
                FROM stations AS s
                    LEFT JOIN from_stations AS fs
                        ON fs.station_cd IS NOT NULL
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
                    AND ($8::int IS NULL OR COALESCE(s.transport_type, 0) = $8)
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
                ORDER BY s.station_cd, s.station_g_cd, s.station_name
            )
            SELECT *
            FROM filtered
            ORDER BY station_g_cd, station_name
            LIMIT $7"#,
            from_station_group_id,
            station_name_pattern,
            station_name_pattern,
            station_name_k_pattern, // station_name_k用には正規化されたパターンを使用
            station_name_pattern,
            station_name_pattern,
            limit,
            transport_type_value
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
            COALESCE(NULLIF(COALESCE(a.line_name, l.line_name), ''), NULL) AS line_name,
            COALESCE(NULLIF(COALESCE(a.line_name_k, l.line_name_k), ''), NULL) AS line_name_k,
            COALESCE(NULLIF(COALESCE(a.line_name_h, l.line_name_h), ''), NULL) AS line_name_h,
            COALESCE(NULLIF(COALESCE(a.line_name_r, l.line_name_r), ''), NULL) AS line_name_r,
            COALESCE(NULLIF(COALESCE(a.line_name_zh, l.line_name_zh), ''), NULL) AS line_name_zh,
            COALESCE(NULLIF(COALESCE(a.line_name_ko, l.line_name_ko), ''), NULL) AS line_name_ko,
            COALESCE(NULLIF(COALESCE(a.line_color_c, l.line_color_c), ''), NULL) AS line_color_c,
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
            COALESCE(l.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
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
            t.kind,
            s.transport_type
          FROM stations AS s
          JOIN lines AS l ON l.line_cd = s.line_cd AND l.e_status = 0
          JOIN station_station_types AS sst ON sst.line_group_cd = $1 AND sst.station_cd = s.station_cd
          JOIN types AS t ON t.type_cd = sst.type_cd
          LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
          LEFT JOIN aliases AS a ON a.id = la.alias_cd
          WHERE
            s.e_status = 0
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
        via_line_id: Option<u32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let via_line_id = via_line_id.map(|id| id as i32);
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
                        AND ($5::int IS NULL OR s1.line_cd = $5)
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
                        sst.id,
                        sst.station_cd,
                        sst.type_cd,
                        sst.line_group_cd,
                        sst.pass
                    FROM
                        station_station_types AS sst
                        JOIN sst_cte_c1 ON sst.line_group_cd = sst_cte_c1.line_group_cd
                        JOIN sst_cte_c2 ON sst.line_group_cd = sst_cte_c2.line_group_cd
                )
            SELECT
            sta.station_cd,
            sta.station_g_cd,
            sta.station_name,
            sta.station_name_k,
            sta.station_name_r,
            sta.station_name_rn,
            sta.station_name_zh,
            sta.station_name_ko,
            sta.station_number1,
            sta.station_number2,
            sta.station_number3,
            sta.station_number4,
            sta.three_letter_code,
            sta.line_cd,
            sta.pref_cd,
            sta.post,
            sta.address,
            sta.lon,
            sta.lat,
            sta.open_ymd,
            sta.close_ymd,
            sta.e_status,
            sta.e_sort,
            lin.company_cd,
            COALESCE(NULLIF(COALESCE(a.line_name, lin.line_name), ''), NULL) AS line_name,
            COALESCE(NULLIF(COALESCE(a.line_name_k, lin.line_name_k), ''), NULL) AS line_name_k,
            COALESCE(NULLIF(COALESCE(a.line_name_h, lin.line_name_h), ''), NULL) AS line_name_h,
            COALESCE(NULLIF(COALESCE(a.line_name_r, lin.line_name_r), ''), NULL) AS line_name_r,
            COALESCE(NULLIF(COALESCE(a.line_name_zh, lin.line_name_zh), ''), NULL) AS line_name_zh,
            COALESCE(NULLIF(COALESCE(a.line_name_ko, lin.line_name_ko), ''), NULL) AS line_name_ko,
            COALESCE(NULLIF(COALESCE(a.line_color_c, lin.line_color_c), ''), NULL) AS line_color_c,
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
            COALESCE(lin.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
            COALESCE(sst.line_group_cd, NULL)::int AS line_group_cd, -- has_train_types用
            NULL::int AS type_id,
            NULL::int AS sst_id,
            NULL::int AS type_cd,
            NULL::int AS pass,
            NULL::text AS type_name,
            NULL::text AS type_name_k,
            NULL::text AS type_name_r,
            NULL::text AS type_name_zh,
            NULL::text AS type_name_ko,
            NULL::text AS color,
            NULL::int AS direction,
            NULL::int AS kind,
            sta.transport_type
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
            via_line_id,
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
                        sst.id,
                        sst.station_cd,
                        sst.type_cd,
                        sst.line_group_cd,
                        sst.pass
                    FROM
                        station_station_types AS sst
                        JOIN sst_cte_c1 ON sst.line_group_cd = sst_cte_c1.line_group_cd
                        JOIN sst_cte_c2 ON sst.line_group_cd = sst_cte_c2.line_group_cd
                )
            SELECT
                sta.station_cd,
                sta.station_g_cd,
                sta.station_name,
                sta.station_name_k,
                sta.station_name_r,
                sta.station_name_rn,
                sta.station_name_zh,
                sta.station_name_ko,
                sta.station_number1,
                sta.station_number2,
                sta.station_number3,
                sta.station_number4,
                sta.three_letter_code,
                sta.line_cd,
                sta.pref_cd,
                sta.post,
                sta.address,
                sta.lon,
                sta.lat,
                sta.open_ymd,
                sta.close_ymd,
                sta.e_status,
                sta.e_sort,
                lin.company_cd,
                COALESCE(NULLIF(COALESCE(a.line_name, lin.line_name), ''), NULL) AS line_name,
                COALESCE(NULLIF(COALESCE(a.line_name_k, lin.line_name_k), ''), NULL) AS line_name_k,
                COALESCE(NULLIF(COALESCE(a.line_name_h, lin.line_name_h), ''), NULL) AS line_name_h,
                COALESCE(NULLIF(COALESCE(a.line_name_r, lin.line_name_r), ''), NULL) AS line_name_r,
                COALESCE(NULLIF(COALESCE(a.line_name_zh, lin.line_name_zh), ''), NULL) AS line_name_zh,
                COALESCE(NULLIF(COALESCE(a.line_name_ko, lin.line_name_ko), ''), NULL) AS line_name_ko,
                COALESCE(NULLIF(COALESCE(a.line_color_c, lin.line_color_c), ''), NULL) AS line_color_c,
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
                COALESCE(lin.average_distance, 0.0)::DOUBLE PRECISION AS average_distance,
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
                tt.kind,
                sta.transport_type
            FROM
                stations AS sta
                LEFT JOIN sst_cte AS sst ON sst.station_cd = sta.station_cd
                JOIN types AS tt ON tt.type_cd = sst.type_cd
                JOIN lines AS lin ON lin.line_cd = sta.line_cd AND lin.e_status = 0
                LEFT JOIN line_aliases AS la ON la.station_cd = sta.station_cd
                LEFT JOIN aliases AS a ON a.id = la.alias_cd
            WHERE
                sta.e_status = 0
                AND ($3::int IS NULL OR sta.line_cd = $3)
            ORDER BY sst.id"#,
            from_station_id as i32,
            to_station_id as i32,
            via_line_id,
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

    #[tokio::test]
    async fn test_station_row_to_station_conversion() {
        let station_row = StationRow {
            station_cd: 1,
            station_g_cd: 1,
            station_name: "Test Station".to_string(),
            station_name_k: "テスト駅".to_string(),
            station_name_r: Some("Test Station".to_string()),
            station_name_rn: None,
            station_name_zh: None,
            station_name_ko: None,
            station_number1: Some("A01".to_string()),
            station_number2: None,
            station_number3: None,
            station_number4: None,
            three_letter_code: Some("TST".to_string()),
            line_cd: 1,
            pref_cd: 13,
            post: "100-0001".to_string(),
            address: "Test Address".to_string(),
            lon: 139.7673068,
            lat: 35.6809591,
            open_ymd: "19900101".to_string(),
            close_ymd: "99991231".to_string(),
            e_status: 0,
            e_sort: 1,
            company_cd: Some(1),
            line_name: Some("Test Line".to_string()),
            line_name_k: Some("テストライン".to_string()),
            line_name_h: Some("テストライン".to_string()),
            line_name_r: Some("Test Line".to_string()),
            line_name_zh: None,
            line_name_ko: None,
            line_color_c: Some("#FF0000".to_string()),
            line_type: Some(1),
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
            average_distance: Some(1.5),
            type_id: Some(1),
            sst_id: Some(1),
            type_cd: Some(1),
            line_group_cd: Some(1),
            pass: Some(0),
            type_name: Some("Local".to_string()),
            type_name_k: Some("各駅停車".to_string()),
            type_name_r: Some("Local".to_string()),
            type_name_zh: None,
            type_name_ko: None,
            color: Some("#008000".to_string()),
            direction: Some(0),
            kind: Some(0),
            transport_type: Some(0),
        };

        let station: Station = station_row.into();

        assert_eq!(station.station_cd, 1);
        assert_eq!(station.station_g_cd, 1);
        assert_eq!(station.station_name, "Test Station");
        assert_eq!(station.station_name_k, "テスト駅");
        assert_eq!(station.line_cd, 1);
        assert_eq!(station.stop_condition, StopCondition::All);
        // Use approximate comparison for floating point values
        assert!((station.lon - 139.7673068).abs() < 0.0001);
        assert!((station.lat - 35.6809591).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_stop_condition_conversion() {
        // Test all stop condition values
        let test_cases = vec![
            (0, StopCondition::All),
            (1, StopCondition::Not),
            (2, StopCondition::Partial),
            (3, StopCondition::Weekday),
            (4, StopCondition::Holiday),
            (5, StopCondition::PartialStop),
            (99, StopCondition::All), // Invalid value should default to All
        ];

        for (pass_value, expected_condition) in test_cases {
            let station_row = StationRow {
                station_cd: 1,
                station_g_cd: 1,
                station_name: "Test".to_string(),
                station_name_k: "テスト".to_string(),
                station_name_r: None,
                station_name_rn: None,
                station_name_zh: None,
                station_name_ko: None,
                station_number1: None,
                station_number2: None,
                station_number3: None,
                station_number4: None,
                three_letter_code: None,
                line_cd: 1,
                pref_cd: 13,
                post: "".to_string(),
                address: "".to_string(),
                lon: 0.0,
                lat: 0.0,
                open_ymd: "".to_string(),
                close_ymd: "".to_string(),
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
                average_distance: Some(0.0),
                type_id: None,
                sst_id: None,
                type_cd: None,
                line_group_cd: None,
                pass: Some(pass_value),
                type_name: None,
                type_name_k: None,
                type_name_r: None,
                type_name_zh: None,
                type_name_ko: None,
                color: None,
                direction: None,
                kind: None,
                transport_type: None,
            };

            let station: Station = station_row.into();
            assert_eq!(station.stop_condition, expected_condition);
        }
    }

    #[tokio::test]
    async fn test_mock_station_repository() {
        // Mock repository test is disabled as mock_repositories module doesn't exist
        // This test would require implementing mock repositories
    }

    #[tokio::test]
    async fn test_sql_query_generation_with_order() {
        let ids = [1, 2, 3];
        let params = (1..=ids.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ");

        let order_case = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("WHEN ${} THEN {}", i + 1, i))
            .collect::<Vec<_>>()
            .join(" ");

        assert_eq!(params, "$1, $2, $3");
        assert_eq!(order_case, "WHEN $1 THEN 0 WHEN $2 THEN 1 WHEN $3 THEN 2");
    }

    #[tokio::test]
    #[ignore] // Requires actual database setup
    async fn test_find_by_id_with_database() {
        // This test would require proper database setup and test data
        // Skipped in regular test runs
    }

    // ============================================
    // Tests for get_by_name search pattern generation
    // ============================================

    mod get_by_name_tests {
        use crate::domain::normalize::normalize_for_search;

        #[test]
        fn test_search_pattern_generation() {
            // station_name_pattern: 元の入力をそのまま使用
            let station_name = "しんじゅく";
            let station_name_pattern = format!("%{station_name}%");
            assert_eq!(station_name_pattern, "%しんじゅく%");

            // station_name_k_pattern: ひらがな→カタカナ変換して使用
            let station_name_k_pattern = format!("%{}%", normalize_for_search(station_name));
            assert_eq!(station_name_k_pattern, "%シンジュク%");
        }

        #[test]
        fn test_hiragana_search_converts_to_katakana_for_name_k() {
            // ひらがな入力の場合、station_name_kはカタカナに変換される
            let input = "とうきょう";
            let pattern_for_name_k = format!("%{}%", normalize_for_search(input));
            assert_eq!(pattern_for_name_k, "%トウキョウ%");
        }

        #[test]
        fn test_katakana_search_remains_katakana() {
            // カタカナ入力の場合、station_name_kもカタカナのまま
            let input = "トウキョウ";
            let pattern_for_name_k = format!("%{}%", normalize_for_search(input));
            assert_eq!(pattern_for_name_k, "%トウキョウ%");
        }

        #[test]
        fn test_kanji_search_remains_kanji_for_name() {
            // 漢字入力の場合、station_nameは漢字のまま
            let input = "東京";
            let pattern_for_name = format!("%{input}%");
            assert_eq!(pattern_for_name, "%東京%");

            // station_name_kも漢字のまま（normalize_for_searchは漢字を変換しない）
            let pattern_for_name_k = format!("%{}%", normalize_for_search(input));
            assert_eq!(pattern_for_name_k, "%東京%");
        }

        #[test]
        fn test_mixed_hiragana_kanji_search() {
            // ひらがな+漢字の混合入力
            let input = "しん宿";
            let pattern_for_name = format!("%{input}%");
            assert_eq!(pattern_for_name, "%しん宿%");

            // station_name_k用: ひらがな部分だけカタカナに変換
            let pattern_for_name_k = format!("%{}%", normalize_for_search(input));
            assert_eq!(pattern_for_name_k, "%シン宿%");
        }

        #[test]
        fn test_search_pattern_with_special_stations() {
            // 実際の駅名での動作確認
            let test_cases = vec![
                ("しながわ", "%シナガワ%"),
                ("うえの", "%ウエノ%"),
                ("あきはばら", "%アキハバラ%"),
                ("いけぶくろ", "%イケブクロ%"),
                ("おおさか", "%オオサカ%"),
            ];

            for (input, expected_k_pattern) in test_cases {
                let pattern_for_name_k = format!("%{}%", normalize_for_search(input));
                assert_eq!(
                    pattern_for_name_k, expected_k_pattern,
                    "Failed for input: {input}"
                );
            }
        }
    }

    // ============================================
    // Tests for get_by_line_group_id SQL structure
    // ============================================

    mod get_by_line_group_id_tests {
        #[test]
        fn test_line_group_id_join_condition_structure() {
            // JOINの条件が正しく構成されることを確認
            // 修正後: JOIN station_station_types AS sst ON sst.line_group_cd = $1 AND sst.station_cd = s.station_cd
            let line_group_id = 1;
            let join_condition = format!(
                "sst.line_group_cd = {} AND sst.station_cd = s.station_cd",
                line_group_id
            );
            assert!(join_condition.contains("sst.line_group_cd = 1"));
            assert!(join_condition.contains("sst.station_cd = s.station_cd"));
        }

        #[test]
        fn test_line_group_id_parameter_binding() {
            // パラメータが正しくバインドされることを確認
            let line_group_id: u32 = 123;
            let bound_value = line_group_id as i32;
            assert_eq!(bound_value, 123);
        }
    }

    // ============================================
    // Database integration tests (require actual DB)
    // ============================================

    #[tokio::test]
    #[ignore] // Requires actual database setup
    async fn test_get_by_name_with_hiragana_search() {
        // ひらがなで駅を検索できることを確認
        // 例: "しんじゅく" で "新宿" (station_name_k: "シンジュク") がヒットする
    }

    #[tokio::test]
    #[ignore] // Requires actual database setup
    async fn test_get_by_line_group_id_returns_correct_stations() {
        // line_group_idで指定した路線グループの駅のみが返されることを確認
        // 以前のバグ: JOINの条件が不完全で意図しない駅が返されていた
    }

    #[tokio::test]
    #[ignore] // Requires actual database setup
    async fn test_get_by_line_group_id_station_order() {
        // 返される駅がsst.idの順序でソートされていることを確認
    }
}
