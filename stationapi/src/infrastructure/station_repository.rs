use async_trait::async_trait;
use sqlx::SqliteConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    domain::{
        entity::{misc::StationIdWithDistance, station::Station},
        error::DomainError,
        repository::station_repository::StationRepository,
    },
    proto::StopCondition,
};

#[derive(sqlx::FromRow)]
struct TrainTypesCountRow {
    train_types_count: i64,
}

#[derive(sqlx::FromRow, Clone)]
struct StationRow {
    pub station_cd: i64,
    pub station_g_cd: i64,
    pub station_name: String,
    pub station_name_k: String,
    pub station_name_r: Option<String>,
    #[allow(dead_code)]
    pub station_name_rn: Option<String>,
    pub station_name_zh: Option<String>,
    pub station_name_ko: Option<String>,
    pub primary_station_number: Option<String>,
    pub secondary_station_number: Option<String>,
    pub extra_station_number: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: i64,
    pub pref_cd: i64,
    pub post: String,
    pub address: String,
    pub lon: f64,
    pub lat: f64,
    pub open_ymd: String,
    pub close_ymd: String,
    pub e_status: i64,
    pub e_sort: i64,
    pub company_cd: Option<i64>,
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
    pub line_type: Option<i64>,
    pub line_symbol_primary: Option<String>,
    pub line_symbol_secondary: Option<String>,
    pub line_symbol_extra: Option<String>,
    pub line_symbol_primary_color: Option<String>,
    pub line_symbol_secondary_color: Option<String>,
    pub line_symbol_extra_color: Option<String>,
    pub line_symbol_primary_shape: Option<String>,
    pub line_symbol_secondary_shape: Option<String>,
    pub line_symbol_extra_shape: Option<String>,
    pub average_distance: Option<f64>,
    #[sqlx(default)]
    pub type_id: Option<i64>,
    #[sqlx(default)]
    pub sst_id: Option<i64>,
    #[sqlx(default)]
    pub type_cd: Option<i64>,
    #[sqlx(default)]
    pub line_group_cd: Option<i64>,
    #[sqlx(default)]
    pub pass: Option<i64>,
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
    pub direction: Option<i64>,
    #[sqlx(default)]
    pub kind: Option<i64>,
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
            line_symbol_primary: row.line_symbol_primary,
            line_symbol_secondary: row.line_symbol_secondary,
            line_symbol_extra: row.line_symbol_extra,
            line_symbol_primary_color: row.line_symbol_primary_color,
            line_symbol_secondary_color: row.line_symbol_secondary_color,
            line_symbol_extra_color: row.line_symbol_extra_color,
            line_symbol_primary_shape: row.line_symbol_primary_shape,
            line_symbol_secondary_shape: row.line_symbol_secondary_shape,
            line_symbol_extra_shape: row.line_symbol_extra_shape,
            average_distance: row.average_distance.unwrap_or(0.0),
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
struct DistanceWithIdRow {
    station_cd: i64,
    distance: f64,
    average_distance: f64,
}

pub struct MyStationRepository {
    conn: Arc<Mutex<SqliteConnection>>,
}

impl MyStationRepository {
    pub fn new(conn: Arc<Mutex<SqliteConnection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl StationRepository for MyStationRepository {
    async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError> {
        let mut conn = self.conn.lock().await;
        InternalStationRepository::find_by_id(id, &mut conn).await
    }
    async fn get_by_id_vec(&self, ids: &[u32]) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.conn.lock().await;
        InternalStationRepository::get_by_id_vec(ids, &mut conn).await
    }
    async fn get_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.conn.lock().await;
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
        let mut conn = self.conn.lock().await;
        InternalStationRepository::get_by_station_group_id(station_group_id, &mut conn).await
    }
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.conn.lock().await;
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
        let mut conn = self.conn.lock().await;
        InternalStationRepository::get_by_coordinates(latitude, longitude, limit, &mut conn).await
    }

    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
        from_station_group_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.conn.lock().await;
        InternalStationRepository::get_by_name(
            station_name,
            limit,
            from_station_group_id,
            &mut conn,
        )
        .await
    }

    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.conn.lock().await;
        InternalStationRepository::get_by_line_group_id(line_group_id, &mut conn).await
    }
    async fn get_station_id_and_distance_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        line_id: Option<u32>,
    ) -> Result<StationIdWithDistance, DomainError> {
        let mut conn = self.conn.lock().await;
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

    async fn get_route_stops(
        &self,
        from_station_id: u32,
        to_station_id: u32,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.conn.lock().await;
        InternalStationRepository::get_route_stops(from_station_id, to_station_id, &mut conn).await
    }
}

struct InternalStationRepository {}

impl InternalStationRepository {
    async fn fetch_has_local_train_types_by_station_id(
        id: u32,
        conn: &mut SqliteConnection,
    ) -> Result<bool, DomainError> {
        let row: TrainTypesCountRow = sqlx::query_as!(
            TrainTypesCountRow,
            "SELECT COUNT(sst.line_group_cd) AS train_types_count
            FROM station_station_types AS sst
                JOIN `types` AS t ON t.type_cd = sst.type_cd
                AND (
                    t.kind IN (0, 1)
                    OR t.top_priority = 1
                )
            WHERE sst.station_cd = ?",
            id,
        )
        .fetch_one(conn)
        .await?;

        Ok(row.train_types_count > 0)
    }

    async fn find_by_id(
        id: u32,
        conn: &mut SqliteConnection,
    ) -> Result<Option<Station>, DomainError> {
        let rows: Option<StationRow> = sqlx::query_as!(
            StationRow,
            r#"SELECT DISTINCT s.*,
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
            COALESCE(CAST(a.line_name AS TEXT), l.line_name) AS "line_name: String",
            COALESCE(CAST(a.line_name_k AS TEXT), l.line_name_k) AS "line_name_k: String",
            COALESCE(CAST(a.line_name_h AS TEXT), l.line_name_h) AS "line_name_h: String",
            COALESCE(CAST(a.line_name_r AS TEXT), l.line_name_r) AS "line_name_r: String",
            COALESCE(CAST(a.line_name_zh AS TEXT), l.line_name_zh) AS "line_name_zh: String",
            COALESCE(CAST(a.line_name_ko AS TEXT), l.line_name_ko) AS "line_name_ko: String",
            COALESCE(CAST(a.line_color_c AS TEXT), l.line_color_c) AS "line_color_c: String",
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
          FROM `stations` AS s
          JOIN `lines` AS l ON l.line_cd = s.line_cd
          AND l.e_status = 0
          LEFT JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd
          LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd
          LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
          LEFT JOIN `aliases` AS a ON a.id = la.alias_cd
          WHERE s.station_cd = ?
            AND s.e_status = 0"#,
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
        ids: &[u32],
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Station>, DomainError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(ids.len() - 1));

        let query_str = format!(
            r#"SELECT DISTINCT
                s.*,
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
                COALESCE(CAST(a.line_name AS TEXT), l.line_name) AS "line_name: String",
                COALESCE(CAST(a.line_name_k AS TEXT), l.line_name_k) AS "line_name_k: String",
                COALESCE(CAST(a.line_name_h AS TEXT), l.line_name_h) AS "line_name_h: String",
                COALESCE(CAST(a.line_name_r AS TEXT), l.line_name_r) AS "line_name_r: String",
                COALESCE(CAST(a.line_name_zh AS TEXT), l.line_name_zh) AS "line_name_zh: String",
                COALESCE(CAST(a.line_name_ko AS TEXT), l.line_name_ko) AS "line_name_ko: String",
                COALESCE(CAST(a.line_color_c AS TEXT), l.line_color_c) AS "line_color_c: String",
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
            ORDER BY FIELD(s.station_cd, {})"#,
            params, params
        );

        let mut query = sqlx::query_as::<_, StationRow>(&query_str);
        for id in ids {
            query = query.bind(id);
        }
        for id in ids {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_line_id_without_train_types(
        line_id: u32,
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows = sqlx::query_as!(
            StationRow,
            r#"SELECT s.*,
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
              COALESCE(CAST(a.line_name AS TEXT), l.line_name) AS "line_name: String",
              COALESCE(CAST(a.line_name_k AS TEXT), l.line_name_k) AS "line_name_k: String",
              COALESCE(CAST(a.line_name_h AS TEXT), l.line_name_h) AS "line_name_h: String",
              COALESCE(CAST(a.line_name_r AS TEXT), l.line_name_r) AS "line_name_r: String",
              COALESCE(CAST(a.line_name_zh AS TEXT), l.line_name_zh) AS "line_name_zh: String",
              COALESCE(CAST(a.line_name_ko AS TEXT), l.line_name_ko) AS "line_name_ko: String",
              COALESCE(CAST(a.line_color_c AS TEXT), l.line_color_c) AS "line_color_c: String",
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
              sst.pass,
              sst.line_group_cd
              FROM `stations` AS s
              JOIN `lines` AS l ON l.line_cd = s.line_cd
                AND l.e_status = 0
              LEFT JOIN `station_station_types` AS sst ON 1 <> 1
              LEFT JOIN `types` AS t ON 1 <> 1
              LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
              LEFT JOIN `aliases` AS a ON a.id = la.alias_cd
            WHERE l.line_cd = ?
              AND s.e_status = 0
            ORDER BY s.e_sort, s.station_cd ASC"#,
            line_id
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_line_id_and_station_id(
        line_id: u32,
        station_id: u32,
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let stations: Vec<Station> = match Self::fetch_has_local_train_types_by_station_id(
            station_id, conn,
        )
        .await?
        {
            true => {
                let rows = sqlx::query_as!(
                        StationRow,
                        r#"SELECT s.*,
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
                          COALESCE(CAST(a.line_name AS TEXT), l.line_name) AS "line_name: String",
                          COALESCE(CAST(a.line_name_k AS TEXT), l.line_name_k) AS "line_name_k: String",
                          COALESCE(CAST(a.line_name_h AS TEXT), l.line_name_h) AS "line_name_h: String",
                          COALESCE(CAST(a.line_name_r AS TEXT), l.line_name_r) AS "line_name_r: String",
                          COALESCE(CAST(a.line_name_zh AS TEXT), l.line_name_zh) AS "line_name_zh: String",
                          COALESCE(CAST(a.line_name_ko AS TEXT), l.line_name_ko) AS "line_name_ko: String",
                          COALESCE(CAST(a.line_color_c AS TEXT), l.line_color_c) AS "line_color_c: String",
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
                          FROM `stations` AS s
                          JOIN `station_station_types` AS sst ON sst.line_group_cd = (
                            SELECT sst.line_group_cd
                            FROM `station_station_types` AS sst
                              LEFT JOIN `types` AS t ON sst.type_cd = t.type_cd
                            WHERE sst.station_cd = ?
                              AND CASE
                                WHEN t.top_priority = 1 AND sst.pass <> 1 THEN sst.type_cd = t.type_cd
                                ELSE t.kind IN (0, 1)
                              END
                            LIMIT 1
                          )
                          AND sst.station_cd = s.station_cd
                          AND s.e_status = 0
                          JOIN `types` AS t ON t.type_cd = sst.type_cd
                          JOIN `lines` AS l ON l.line_cd = s.line_cd
                            AND l.e_status = 0
                          LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
                          LEFT JOIN `aliases` AS a ON a.id = la.alias_cd
                          ORDER BY sst.id"#,
                          station_id
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
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows = sqlx::query_as!(
            StationRow,
            r#"SELECT s.*,
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
            COALESCE(CAST(a.line_name AS TEXT), l.line_name) AS "line_name: String",
            COALESCE(CAST(a.line_name_k AS TEXT), l.line_name_k) AS "line_name_k: String",
            COALESCE(CAST(a.line_name_h AS TEXT), l.line_name_h) AS "line_name_h: String",
            COALESCE(CAST(a.line_name_r AS TEXT), l.line_name_r) AS "line_name_r: String",
            COALESCE(CAST(a.line_name_zh AS TEXT), l.line_name_zh) AS "line_name_zh: String",
            COALESCE(CAST(a.line_name_ko AS TEXT), l.line_name_ko) AS "line_name_ko: String",
            COALESCE(CAST(a.line_color_c AS TEXT), l.line_color_c) AS "line_color_c: String",
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
            `stations` AS s
            JOIN `lines` AS l ON l.line_cd = s.line_cd AND l.e_status = 0
            LEFT JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd
            LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
            LEFT JOIN `aliases` AS a ON a.id = la.alias_cd
          WHERE
            s.station_g_cd = ?
            AND s.line_cd = l.line_cd
            AND s.e_status = 0"#,
            group_id
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_station_group_id_vec(
        group_id_vec: &[u32],
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Station>, DomainError> {
        if group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(group_id_vec.len() - 1));
        let query_str = format!(
            r#"SELECT
            s.*,
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
            COALESCE(CAST(a.line_name AS TEXT), l.line_name) AS "line_name: String",
            COALESCE(CAST(a.line_name_k AS TEXT), l.line_name_k) AS "line_name_k: String",
            COALESCE(CAST(a.line_name_h AS TEXT), l.line_name_h) AS "line_name_h: String",
            COALESCE(CAST(a.line_name_r AS TEXT), l.line_name_r) AS "line_name_r: String",
            COALESCE(CAST(a.line_name_zh AS TEXT), l.line_name_zh) AS "line_name_zh: String",
            COALESCE(CAST(a.line_name_ko AS TEXT), l.line_name_ko) AS "line_name_ko: String",
            COALESCE(CAST(a.line_color_c AS TEXT), l.line_color_c) AS "line_color_c: String"
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
            AND s.e_status = 0"#,
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
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let lat_min = latitude - 0.1;
        let lat_max = latitude + 0.1;
        let lon_min = longitude - 0.1;
        let lon_max = longitude + 0.1;

        let rows = sqlx::query_as::<_, StationRow>(
            r#"SELECT
                s.*,
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
                COALESCE(a.line_name, l.line_name)         AS line_name,
                COALESCE(a.line_name_k, l.line_name_k)     AS line_name_k,
                COALESCE(a.line_name_h, l.line_name_h)     AS line_name_h,
                COALESCE(a.line_name_r, l.line_name_r)     AS line_name_r,
                COALESCE(a.line_name_zh, l.line_name_zh)   AS line_name_zh,
                COALESCE(a.line_name_ko, l.line_name_ko)   AS line_name_ko,
                COALESCE(a.line_color_c, l.line_color_c)   AS line_color_c,
                ((s.lat - ?) * (s.lat - ?) + (s.lon - ?) * (s.lon - ?)) AS distance_sq
                FROM stations AS s
                JOIN station_rtree r ON s.station_cd = r.station_cd
                JOIN lines AS l
                ON s.line_cd = l.line_cd
                LEFT JOIN line_aliases AS la
                ON la.station_cd = s.station_cd
                LEFT JOIN aliases AS a
                ON a.id = la.alias_cd
                WHERE r.min_lat <= ? AND r.max_lat >= ?
                AND r.min_lon <= ? AND r.max_lon >= ?
                AND s.e_status = 0
                ORDER BY distance_sq
                LIMIT
                ?"#,
        )
        .bind(latitude)
        .bind(latitude)
        .bind(longitude)
        .bind(longitude)
        .bind(lat_max)
        .bind(lat_min)
        .bind(lon_max)
        .bind(lon_min)
        .bind(limit.unwrap_or(1))
        .fetch_all(&mut *conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        if stations.len() != 0 {
            return Ok(stations);
        }

        let rows = sqlx::query_as::<_, StationRow>(
            r#"SELECT
                s.*,
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
                COALESCE(a.line_name, l.line_name)         AS line_name,
                COALESCE(a.line_name_k, l.line_name_k)     AS line_name_k,
                COALESCE(a.line_name_h, l.line_name_h)     AS line_name_h,
                COALESCE(a.line_name_r, l.line_name_r)     AS line_name_r,
                COALESCE(a.line_name_zh, l.line_name_zh)   AS line_name_zh,
                COALESCE(a.line_name_ko, l.line_name_ko)   AS line_name_ko,
                COALESCE(a.line_color_c, l.line_color_c)   AS line_color_c,
                ((s.lat - ?) * (s.lat - ?) + (s.lon - ?) * (s.lon - ?)) AS distance_sq
                FROM stations AS s
                JOIN lines AS l
                ON s.line_cd = l.line_cd
                LEFT JOIN line_aliases AS la
                ON la.station_cd = s.station_cd
                LEFT JOIN aliases AS a
                ON a.id = la.alias_cd
                WHERE s.e_status = 0
                ORDER BY distance_sq
                LIMIT
                ?"#,
        )
        .bind(latitude)
        .bind(latitude)
        .bind(longitude)
        .bind(longitude)
        .bind(limit.unwrap_or(1))
        .fetch_all(&mut *conn)
        .await?;

        let stations = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_station_id_and_distance_by_coordinates_and_line_id(
        latitude: f64,
        longitude: f64,
        line_id: u32,
        conn: &mut SqliteConnection,
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
            station_id: row.station_cd as u32,
            distance: row.distance,
            average_distance: row.average_distance,
        };

        Ok(id_with_distance)
    }

    async fn get_station_id_and_distance_by_coordinates(
        latitude: f64,
        longitude: f64,
        conn: &mut SqliteConnection,
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
            station_id: row.station_cd as u32,
            distance: row.distance,
            average_distance: row.average_distance,
        };

        Ok(id_with_distance)
    }

    async fn get_by_name(
        station_name: String,
        limit: Option<u32>,
        from_station_group_id: Option<u32>,
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let station_name = &(format!("%{}%", station_name));
        let limit = &limit.unwrap_or(1);

        let rows = sqlx::query_as!(
            StationRow,
            r#"WITH from_stations AS (
                SELECT
                    s.station_cd,
                    s.line_cd
                FROM stations AS s
                WHERE s.station_g_cd = ?
                AND s.e_status = 0
            )
            SELECT
                s.*,
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
                dst_sst.id AS sst_id,
                dst_sst.type_cd,
                dst_sst.line_group_cd,
                dst_sst.pass,
                COALESCE(CAST(a.line_name AS TEXT), l.line_name) AS "line_name: String",
                COALESCE(CAST(a.line_name_k AS TEXT), l.line_name_k) AS "line_name_k: String",
                COALESCE(CAST(a.line_name_h AS TEXT), l.line_name_h) AS "line_name_h: String",
                COALESCE(CAST(a.line_name_r AS TEXT), l.line_name_r) AS "line_name_r: String",
                COALESCE(CAST(a.line_name_zh AS TEXT), l.line_name_zh) AS "line_name_zh: String",
                COALESCE(CAST(a.line_name_ko AS TEXT), l.line_name_ko) AS "line_name_ko: String",
                COALESCE(CAST(a.line_color_c AS TEXT), l.line_color_c) AS "line_color_c: String",
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
                    s.station_name   LIKE ?
                    OR s.station_name_rn LIKE ?
                    OR s.station_name_k LIKE ?
                    OR s.station_name_zh LIKE ?
                    OR s.station_name_ko LIKE ?
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
                        AND s.line_cd = IFNULL(fs.line_cd, s.line_cd)
                    )
                )
            GROUP BY
                s.station_g_cd
            LIMIT ?"#,
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
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let rows = sqlx::query_as!(
            StationRow,
            r#"SELECT DISTINCT s.*,
            COALESCE(CAST(a.line_name AS TEXT), l.line_name) AS "line_name: String",
            COALESCE(CAST(a.line_name_k AS TEXT), l.line_name_k) AS "line_name_k: String",
            COALESCE(CAST(a.line_name_h AS TEXT), l.line_name_h) AS "line_name_h: String",
            COALESCE(CAST(a.line_name_r AS TEXT), l.line_name_r) AS "line_name_r: String",
            COALESCE(CAST(a.line_name_zh AS TEXT), l.line_name_zh) AS "line_name_zh: String",
            COALESCE(CAST(a.line_name_ko AS TEXT), l.line_name_ko) AS "line_name_ko: String",
            COALESCE(CAST(a.line_color_c AS TEXT), l.line_color_c) AS "line_color_c: String",
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
          FROM `stations` AS s
          JOIN `lines` AS l ON l.line_cd = s.line_cd AND l.e_status = 0
          LEFT JOIN `station_station_types` AS sst ON sst.line_group_cd = ?
          LEFT JOIN `types` AS t ON t.type_cd = sst.type_cd
          LEFT JOIN `line_aliases` AS la ON la.station_cd = s.station_cd
          LEFT JOIN `aliases` AS a ON a.id = la.alias_cd
          WHERE
            s.line_cd = l.line_cd
            AND s.station_cd = sst.station_cd
            AND s.e_status = 0"#,
            line_group_id
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_route_stops(
        from_station_id: u32,
        to_station_id: u32,
        conn: &mut SqliteConnection,
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
                        s.station_g_cd = ?
                ),
                to_cte AS (
                    SELECT
                        s.station_cd,
                        s.line_cd
                    FROM
                        stations AS s
                    WHERE
                        s.station_g_cd = ?
                ),
                local_cte AS (
                    SELECT
                        s.*
                    FROM
                        `stations` AS s
                        JOIN `lines` AS l ON l.line_cd IN (
                            SELECT
                                line_cd
                            FROM
                                `stations`
                            WHERE
                                station_g_cd = ?
                                AND e_status = 0
                        )
                        AND l.line_cd IN (
                            (
                                SELECT
                                    line_cd
                                FROM
                                    `stations`
                                WHERE
                                    station_g_cd = ?
                                    AND e_status = 0
                            )
                        )
                    WHERE
                        s.line_cd = l.line_cd
                    ORDER BY
                        s.e_sort,
                        s.station_cd
                ),
                sst_cte_c1 AS (
                    SELECT
                        sst.line_group_cd
                    FROM
                        station_station_types AS sst
                        JOIN from_cte
                    WHERE
                        sst.station_cd = from_cte.station_cd
                        AND sst.pass <> 1
                ),
                sst_cte_c2 AS (
                    SELECT
                        sst.line_group_cd
                    FROM
                        station_station_types AS sst
                        JOIN to_cte
                    WHERE
                        sst.station_cd = to_cte.station_cd
                        AND sst.pass <> 1
                ),
                sst_cte AS (
                    SELECT
                        sst.*
                    FROM
                        `station_station_types` AS sst
                        JOIN sst_cte_c1
                        JOIN sst_cte_c2
                    WHERE
                        sst.line_group_cd = sst_cte_c1.line_group_cd
                        AND sst.line_group_cd = sst_cte_c2.line_group_cd
                )
            SELECT
            sta.*,
            COALESCE(CAST(a.line_name AS TEXT), lin.line_name) AS "line_name: String",
            COALESCE(CAST(a.line_name_k AS TEXT), lin.line_name_k) AS "line_name_k: String",
            COALESCE(CAST(a.line_name_h AS TEXT), lin.line_name_h) AS "line_name_h: String",
            COALESCE(CAST(a.line_name_r AS TEXT), lin.line_name_r) AS "line_name_r: String",
            COALESCE(CAST(a.line_name_zh AS TEXT), lin.line_name_zh) AS "line_name_zh: String",
            COALESCE(CAST(a.line_name_ko AS TEXT), lin.line_name_ko) AS "line_name_ko: String",
            COALESCE(CAST(a.line_color_c AS TEXT), lin.line_color_c) AS "line_color_c: String",
            lin.company_cd,
            lin.line_type,
            lin.line_symbol_primary,
            lin.line_symbol_secondary,
            lin.line_symbol_extra,
            lin.line_symbol_primary_color,
            lin.line_symbol_secondary_color,
            lin.line_symbol_extra_color,
            lin.line_symbol_primary_shape,
            lin.line_symbol_secondary_shape,
            lin.line_symbol_extra_shape,
            lin.average_distance,
            sst.id AS sst_id,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass,
            tt.id AS type_id,
            tt.type_name,
            tt.type_name_k,
            tt.type_name_r,
            tt.type_name_zh,
            tt.type_name_ko,
            tt.color,
            tt.direction,
            tt.kind
            FROM
                local_cte AS sta
                LEFT JOIN `sst_cte` AS sst ON sst.station_cd = sta.station_cd
                LEFT JOIN `types` AS tt ON tt.type_cd = sst.type_cd
                JOIN `lines` AS lin ON lin.line_cd = sta.line_cd
                LEFT JOIN `line_aliases` AS la ON la.station_cd = sta.station_cd
                LEFT JOIN `aliases` AS a ON a.id = la.alias_cd
            WHERE
                sst.line_group_cd IS NULL
                AND lin.e_status = 0
                AND sta.e_status = 0
                ORDER BY sta.e_sort, sta.station_cd"#,
            from_station_id,
            to_station_id,
            from_station_id,
            to_station_id,
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
                        s.station_g_cd = ?
                        AND s.e_status = 0
                ),
                to_cte AS (
                    SELECT
                        s.station_cd,
                        s.line_cd
                    FROM
                        stations AS s
                    WHERE
                        s.station_g_cd = ?
                        AND s.e_status = 0
                ),
                sst_cte_c1 AS (
                    SELECT
                        sst.line_group_cd
                    FROM
                        station_station_types AS sst
                        JOIN from_cte
                    WHERE
                        sst.station_cd = from_cte.station_cd
                        AND sst.pass <> 1
                ),
                sst_cte_c2 AS (
                    SELECT
                        sst.line_group_cd
                    FROM
                        station_station_types AS sst
                        JOIN to_cte
                    WHERE
                        sst.station_cd = to_cte.station_cd
                        AND sst.pass <> 1
                ),
                sst_cte AS (
                    SELECT
                        sst.*
                    FROM
                        `station_station_types` AS sst
                        JOIN sst_cte_c1
                        JOIN sst_cte_c2
                    WHERE
                        sst.line_group_cd = sst_cte_c1.line_group_cd
                        AND sst.line_group_cd = sst_cte_c2.line_group_cd
                )
            SELECT
                sta.*,
                lin.company_cd,
                lin.line_type,
                lin.line_symbol_primary,
                lin.line_symbol_secondary,
                lin.line_symbol_extra,
                lin.line_symbol_primary_color,
                lin.line_symbol_secondary_color,
                lin.line_symbol_extra_color,
                lin.line_symbol_primary_shape,
                lin.line_symbol_secondary_shape,
                lin.line_symbol_extra_shape,
                lin.average_distance,
                sst.id AS sst_id,
                sst.type_cd,
                sst.line_group_cd,
                sst.pass,
                COALESCE(CAST(a.line_name AS TEXT), lin.line_name) AS "line_name: String",
                COALESCE(CAST(a.line_name_k AS TEXT), lin.line_name_k) AS "line_name_k: String",
                COALESCE(CAST(a.line_name_h AS TEXT), lin.line_name_h) AS "line_name_h: String",
                COALESCE(CAST(a.line_name_r AS TEXT), lin.line_name_r) AS "line_name_r: String",
                COALESCE(CAST(a.line_name_zh AS TEXT), lin.line_name_zh) AS "line_name_zh: String",
                COALESCE(CAST(a.line_name_ko AS TEXT), lin.line_name_ko) AS "line_name_ko: String",
                COALESCE(CAST(a.line_color_c AS TEXT), lin.line_color_c) AS "line_color_c: String",
                tt.id AS type_id,
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
                LEFT JOIN `sst_cte` AS sst ON sst.station_cd = sta.station_cd
                LEFT JOIN `types` AS tt ON tt.type_cd = sst.type_cd
                JOIN `lines` AS lin ON lin.line_cd = sta.line_cd
                LEFT JOIN `line_aliases` AS la ON la.station_cd = sta.station_cd
                LEFT JOIN `aliases` AS a ON a.id = la.alias_cd
            WHERE
                sta.station_cd = sst.station_cd
                AND lin.e_status = 0
                AND sta.e_status = 0
            ORDER BY
                sst.id"#,
            from_station_id,
            to_station_id,
        )
        .fetch_all(conn)
        .await?;

        rows.append(&mut typed_rows);
        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }
}
