use crate::{
    domain::{
        entity::{misc::StationIdWithDistance, station::Station},
        error::DomainError,
        repository::station_repository::StationRepository,
    },
    station_api::StopCondition,
};
use async_trait::async_trait;
use sqlx::{query_as, PgConnection, PgPool};

#[derive(sqlx::FromRow, Clone)]
struct StationRow {
    pub station_cd: Option<i32>,
    pub station_g_cd: Option<i32>,
    pub station_name: Option<String>,
    pub station_name_k: Option<String>,
    pub station_name_r: Option<String>,
    pub station_name_zh: Option<String>,
    pub station_name_ko: Option<String>,
    pub primary_station_number: Option<String>,
    pub secondary_station_number: Option<String>,
    pub extra_station_number: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: Option<i32>,
    pub pref_cd: Option<i32>,
    pub post: Option<String>,
    pub address: Option<String>,
    pub lon: Option<f64>,
    pub lat: Option<f64>,
    pub open_ymd: Option<String>,
    pub close_ymd: Option<String>,
    pub e_status: Option<i32>,
    pub e_sort: Option<i32>,
    pub has_train_types: Option<bool>,
    pub distance: Option<f64>,
    // linesからJOIN
    pub company_cd: Option<i32>,
    pub line_name: Option<String>,
    pub line_name_k: Option<String>,
    pub line_name_h: Option<String>,
    pub line_name_r: Option<String>,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: Option<String>,
    pub line_type: Option<i32>,
    pub line_symbol_primary: Option<String>,
    pub line_symbol_secondary: Option<String>,
    pub line_symbol_extra: Option<String>,
    pub line_symbol_primary_color: Option<String>,
    pub line_symbol_secondary_color: Option<String>,
    pub line_symbol_extra_color: Option<String>,
    pub line_symbol_primary_shape: Option<String>,
    pub line_symbol_secondary_shape: Option<String>,
    pub line_symbol_extra_shape: Option<String>,
    average_distance: Option<f64>,
    // station_station_typesからJOIN
    pub type_cd: Option<i32>,
    pub line_group_cd: Option<i32>,
    pub pass: Option<i32>,
    // typesからJOIN
    #[sqlx(default)]
    #[allow(dead_code)]
    sst_id: Option<i32>,
    pub type_name: Option<String>,
    pub type_name_k: Option<String>,
    pub type_name_r: Option<String>,
    pub type_name_zh: Option<String>,
    pub type_name_ko: Option<String>,
    pub kind: Option<i32>,
    pub color: Option<String>,
    pub direction: Option<i32>,
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
            station_cd: row.station_cd.unwrap(),
            station_g_cd: row.station_g_cd.unwrap(),
            station_name: row.station_name.unwrap(),
            station_name_k: row.station_name_k.unwrap(),
            station_name_r: row.station_name_r.unwrap(),
            station_name_zh: row.station_name_zh,
            station_name_ko: row.station_name_ko,
            station_numbers: vec![],
            primary_station_number: row.primary_station_number,
            secondary_station_number: row.secondary_station_number,
            extra_station_number: row.extra_station_number,
            three_letter_code: row.three_letter_code,
            line_cd: row.line_cd.unwrap(),
            line: None,
            lines: vec![],
            pref_cd: row.pref_cd.unwrap(),
            post: row.post.unwrap(),
            address: row.address.unwrap(),
            lon: row.lon.unwrap(),
            lat: row.lat.unwrap(),
            open_ymd: row.open_ymd.unwrap(),
            close_ymd: row.close_ymd.unwrap(),
            e_status: row.e_status.unwrap(),
            e_sort: row.e_sort.unwrap(),
            stop_condition,
            distance: row.distance,
            train_type: None,
            has_train_types: row.has_train_types.unwrap_or(false),
            company_cd: row.company_cd.unwrap(),
            line_name: row.line_name.unwrap(),
            line_name_k: row.line_name_k.unwrap(),
            line_name_h: row.line_name_h.unwrap(),
            line_name_r: row.line_name_r,
            line_name_zh: row.line_name_zh,
            line_name_ko: row.line_name_ko,
            line_color_c: row.line_color_c.unwrap(),
            line_type: row.line_type.unwrap(),
            line_symbol_primary: row.line_symbol_primary,
            line_symbol_secondary: row.line_symbol_secondary,
            line_symbol_extra: row.line_symbol_extra,
            line_symbol_primary_color: row.line_symbol_primary_color,
            line_symbol_secondary_color: row.line_symbol_secondary_color,
            line_symbol_extra_color: row.line_symbol_extra_color,
            line_symbol_primary_shape: row.line_symbol_primary_shape,
            line_symbol_secondary_shape: row.line_symbol_secondary_shape,
            line_symbol_extra_shape: row.line_symbol_extra_shape,
            average_distance: row.average_distance.unwrap(),
            type_cd: row.type_cd,
            line_group_cd: row.line_group_cd,
            pass: row.pass,
            type_name: row.type_name.unwrap_or_default(),
            type_name_k: row.type_name_k.unwrap_or_default(),
            type_name_r: row.type_name_r.unwrap_or_default(),
            type_name_zh: row.type_name_zh,
            type_name_ko: row.type_name_ko,
            color: row.color.unwrap_or_default(),
            direction: row.direction.unwrap_or_default(),
            kind: row.kind.unwrap_or_default(),
        }
    }
}

#[derive(sqlx::FromRow, Clone)]
struct DistanceWithIdRow {
    station_cd: i32,
    distance: Option<f64>,
    average_distance: f64,
}

#[derive(Debug, Clone)]
pub struct MyStationRepository {
    pool: PgPool,
}

impl MyStationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StationRepository for MyStationRepository {
    async fn find_by_id(&self, id: i32) -> Result<Option<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::find_by_id(id, &mut conn).await
    }
    async fn get_by_id_vec(&self, ids: Vec<i32>) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_id_vec(ids, &mut conn).await
    }
    async fn get_by_line_id(
        &self,
        line_id: i32,
        station_id: Option<i32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_line_id(line_id, station_id, &mut conn).await
    }
    async fn get_by_station_group_id(
        &self,
        station_group_id: i32,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_station_group_id(station_group_id, &mut conn).await
    }
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: Vec<i32>,
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
        limit: Option<i32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_coordinates(latitude, longitude, limit, &mut conn).await
    }

    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<i32>,
    ) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_name(station_name, limit, &mut conn).await
    }

    async fn get_by_line_group_id(&self, line_group_id: i32) -> Result<Vec<Station>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalStationRepository::get_by_line_group_id(line_group_id, &mut conn).await
    }
    async fn get_station_id_and_distance_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        line_id: Option<i32>,
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
    async fn find_by_id(id: i32, conn: &mut PgConnection) -> Result<Option<Station>, DomainError> {
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
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            COALESCE(s.station_cd = sst.station_cd, FALSE) AS has_train_types,
            sst.type_cd AS "type_cd?",
            sst.line_group_cd AS "line_group_cd?",
            sst.pass AS "pass?",
            sst.id AS "sst_id?",
            t.type_name AS "type_name?",
            t.type_name_k AS "type_name_k?",
            t.type_name_r AS "type_name_r?",
            t.type_name_zh AS "type_name_zh?",
            t.type_name_ko AS "type_name_ko?",
            t.color AS "color?",
            t.kind AS "kind?",
            t.direction AS "direction?",
            0.0::double precision AS distance
          FROM stations AS s
            JOIN lines AS l ON l.line_cd = s.line_cd
            AND l.e_status = 0
            LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN types AS t ON t.type_cd = sst.type_cd
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON la.alias_cd = a.id
          WHERE s.station_cd = $1
            AND s.e_status = 0
          ORDER BY s.e_sort,
            s.station_cd"#,
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
        ids: Vec<i32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let rows = query_as!(
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
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            COALESCE(s.station_cd = sst.station_cd, FALSE) AS has_train_types,
            sst.type_cd AS "type_cd?",
            sst.line_group_cd AS "line_group_cd?",
            sst.pass AS "pass?",
            sst.id AS "sst_id?",
            t.type_name AS "type_name?",
            t.type_name_k AS "type_name_k?",
            t.type_name_r AS "type_name_r?",
            t.type_name_zh AS "type_name_zh?",
            t.type_name_ko AS "type_name_ko?",
            t.color AS "color?",
            t.kind AS "kind?",
            t.direction AS "direction?",
            0.0::double precision AS distance
            FROM stations AS s
            JOIN lines AS l ON l.line_cd = s.line_cd AND l.e_status = 0
            LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN types AS t ON t.type_cd = sst.type_cd
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON la.alias_cd = a.id
            WHERE
              s.station_cd IN (SELECT UNNEST($1::integer[]))
              AND s.line_cd = l.line_cd
              AND s.e_status = 0
              ORDER BY s.e_sort, s.station_cd"#,
            &ids
        )
        .fetch_all(conn)
        .await?;
        let lines: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }

    async fn get_by_line_id(
        line_id: i32,
        station_id: Option<i32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let station_row: Vec<StationRow> = sqlx::query_as!(
            StationRow,
            r#"(
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
                t.kind,
                sst.pass,
                COALESCE(a.line_name, l.line_name) AS line_name,
                COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
                COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
                COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
                COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
                COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
                COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
                sst.line_group_cd,
                COALESCE(s.station_cd = sst.station_cd, FALSE) AS has_train_types,
                sst.id AS sst_id,
                0.0::double precision AS distance
            FROM stations AS s
                JOIN lines AS l ON l.line_cd = $1
                AND l.e_status = 0
                LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
                LEFT JOIN aliases AS a ON la.alias_cd = a.id
                LEFT JOIN station_station_types AS sst ON sst.line_group_cd = (
                SELECT sst.line_group_cd
                FROM station_station_types AS sst
                    LEFT JOIN types AS t ON sst.type_cd = t.type_cd
                WHERE CASE
                    WHEN $2::integer IS NOT NULL THEN sst.station_cd = $2
                    END
                    AND CASE
                    WHEN t.top_priority = 1 THEN sst.type_cd = t.type_cd
                    ELSE t.kind IN (0, 1)
                    END
                ORDER BY sst.id
                LIMIT 1
                )
                LEFT JOIN types AS t ON t.type_cd = sst.type_cd
            WHERE sst.station_cd IS NULL
                AND s.line_cd = l.line_cd
                AND s.e_status = 0
            )
            UNION
            (
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
                t.kind,
                sst.pass,
                COALESCE(a.line_name, l.line_name) AS line_name,
                COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
                COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
                COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
                COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
                COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
                COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
                sst.line_group_cd,
                COALESCE(s.station_cd = sst.station_cd, FALSE) AS has_train_types,
                sst.id AS sst_id,
                0.0::double precision AS distance
            FROM stations AS s
                JOIN lines AS l ON l.line_cd = s.line_cd
                AND l.e_status = 0
                LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
                LEFT JOIN aliases AS a ON la.alias_cd = a.id
                LEFT JOIN station_station_types AS sst ON sst.line_group_cd = (
                SELECT sst.line_group_cd
                FROM station_station_types AS sst
                    LEFT JOIN types AS t ON sst.type_cd = t.type_cd
                WHERE CASE
                    WHEN $2::integer IS NOT NULL THEN sst.station_cd = $2
                    END
                    AND CASE
                    WHEN t.top_priority = 1 THEN sst.type_cd = t.type_cd
                    ELSE t.kind IN (0, 1)
                    END
                ORDER BY sst.id
                LIMIT 1
                )
                LEFT JOIN types AS t ON t.type_cd = sst.type_cd
            WHERE sst.station_cd IS NOT NULL
                AND s.station_cd = sst.station_cd
                AND s.line_cd = l.line_cd
                AND s.e_status = 0
            )
            ORDER BY sst_id, e_status, station_cd"#,
            line_id,
            station_id
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = station_row.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_station_group_id(
        group_id: i32,
        conn: &mut PgConnection,
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
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            COALESCE(s.station_cd = sst.station_cd, FALSE) AS has_train_types,
            sst.type_cd AS "type_cd?",
            sst.line_group_cd AS "line_group_cd?",
            sst.pass AS "pass?",
            sst.id AS "sst_id?",
            t.type_name AS "type_name?",
            t.type_name_k AS "type_name_k?",
            t.type_name_r AS "type_name_r?",
            t.type_name_zh AS "type_name_zh?",
            t.type_name_ko AS "type_name_ko?",
            t.color AS "color?",
            t.kind AS "kind?",
            t.direction AS "direction?",
            0.0::double precision AS distance
          FROM
            stations AS s
            JOIN lines AS l ON l.line_cd = s.line_cd AND l.e_status = 0
            LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN types AS t ON t.type_cd = sst.type_cd
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON a.id = la.alias_cd
          WHERE
            s.station_g_cd = $1
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
        group_id_vec: Vec<i32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        if group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let rows = query_as!(
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
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            COALESCE(s.station_cd = sst.station_cd, FALSE) AS has_train_types,
            sst.type_cd AS "type_cd?",
            sst.line_group_cd AS "line_group_cd?",
            sst.pass AS "pass?",
            sst.id AS "sst_id?",
            t.type_name AS "type_name?",
            t.type_name_k AS "type_name_k?",
            t.type_name_r AS "type_name_r?",
            t.type_name_zh AS "type_name_zh?",
            t.type_name_ko AS "type_name_ko?",
            t.color AS "color?",
            t.kind AS "kind?",
            t.direction AS "direction?",
            0.0::double precision AS distance
          FROM
            stations AS s
            JOIN lines AS l ON l.line_cd = s.line_cd AND l.e_status = 0
            LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN types AS t ON t.type_cd = sst.type_cd
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON a.id = la.alias_cd
          WHERE
            s.station_g_cd IN (SELECT UNNEST($1::integer[]))
            AND s.line_cd = l.line_cd
            AND s.e_status = 0"#,
            &group_id_vec
        )
        .fetch_all(conn)
        .await?;
        let lines: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(lines)
    }

    async fn get_by_coordinates(
        latitude: f64,
        longitude: f64,
        limit: Option<i32>,
        conn: &mut PgConnection,
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
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            COALESCE(s.station_cd = sst.station_cd, FALSE) AS has_train_types,
            sst.type_cd AS "type_cd?",
            sst.line_group_cd AS "line_group_cd?",
            sst.pass AS "pass?",
            sst.id AS "sst_id?",
            t.type_name AS "type_name?",
            t.type_name_k AS "type_name_k?",
            t.type_name_r AS "type_name_r?",
            t.type_name_zh AS "type_name_zh?",
            t.type_name_ko AS "type_name_ko?",
            t.color AS "color?",
            t.kind AS "kind?",
            t.direction AS "direction?",
            POINT(s.lat, s.lon) <-> POINT($1, $2) AS distance
          FROM stations AS s
            JOIN lines AS l ON l.line_cd = s.line_cd
            AND l.e_status = 0
            LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd
            LEFT JOIN types AS t ON t.type_cd = sst.type_cd
            LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
            LEFT JOIN aliases AS a ON la.alias_cd = a.id
          WHERE s.e_status = 0
              ORDER BY 
                distance 
              LIMIT
                $3"#,
            latitude,
            longitude,
            limit.unwrap_or(1) as i32
        )
        .fetch_all(conn)
        .await?;

        let stations = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_station_id_and_distance_by_coordinates_and_line_id(
        latitude: f64,
        longitude: f64,
        line_id: i32,
        conn: &mut PgConnection,
    ) -> Result<StationIdWithDistance, DomainError> {
        let row = sqlx::query_as!(
            DistanceWithIdRow,
            "SELECT
            s.station_cd,
            l.average_distance,
            POINT(s.lat, s.lon) <-> POINT($1, $2) AS distance
          FROM stations AS s
          JOIN lines AS l ON l.line_cd = $3
          WHERE
            s.line_cd = $3
            AND s.e_status = 0
          ORDER BY 
            distance
          LIMIT 
            1",
            latitude,
            longitude,
            line_id
        )
        .fetch_one(conn)
        .await?;
        let id_with_distance = StationIdWithDistance {
            station_id: row.station_cd,

            distance: row.distance.unwrap(),
            average_distance: row.average_distance,
        };

        Ok(id_with_distance)
    }

    async fn get_station_id_and_distance_by_coordinates(
        latitude: f64,
        longitude: f64,
        conn: &mut PgConnection,
    ) -> Result<StationIdWithDistance, DomainError> {
        let row = sqlx::query_as!(
            DistanceWithIdRow,
            "SELECT
          s.station_cd,
          l.average_distance,
            POINT(s.lat, s.lon) <-> POINT($1, $2) AS distance
        FROM stations AS s
        JOIN lines AS l ON l.line_cd = s.line_cd
        WHERE
          s.e_status = 0
        ORDER BY 
          distance
        LIMIT 
          1",
            latitude,
            longitude
        )
        .fetch_one(conn)
        .await?;
        let id_with_distance = StationIdWithDistance {
            station_id: row.station_cd,
            distance: row.distance.unwrap(),
            average_distance: row.average_distance,
        };

        Ok(id_with_distance)
    }

    async fn get_by_name(
        station_name: String,
        limit: Option<i32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<Station>, DomainError> {
        let station_name = format!("%{}%", station_name);

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
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            COALESCE(s.station_cd = sst.station_cd, FALSE) AS has_train_types,
            sst.type_cd AS "type_cd?",
            sst.line_group_cd AS "line_group_cd?",
            sst.pass AS "pass?",
            sst.id AS "sst_id?",
            t.type_name AS "type_name?",
            t.type_name_k AS "type_name_k?",
            t.type_name_r AS "type_name_r?",
            t.type_name_zh AS "type_name_zh?",
            t.type_name_ko AS "type_name_ko?",
            t.color AS "color?",
            t.kind AS "kind?",
            t.direction AS "direction?",
            0.0::double precision AS distance
                  FROM stations AS s
                  JOIN lines AS l ON l.line_cd = s.line_cd AND l.e_status = 0
                  LEFT JOIN station_station_types AS sst ON sst.station_cd = s.station_cd
                  LEFT JOIN types AS t ON t.type_cd = sst.type_cd
                  LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
                  LEFT JOIN aliases AS a ON la.alias_cd = a.id
                  WHERE
                    (
                      station_name ILIKE $1
                      OR station_name_r ILIKE $1
                      OR station_name_k ILIKE $1
                      OR station_name_zh ILIKE $1
                      OR station_name_ko ILIKE $1
                    )
                    AND s.e_status = 0
                  LIMIT
                    $2"#,
            station_name,
            limit.unwrap_or(1) as i32,
        )
        .fetch_all(conn)
        .await?;

        let stations: Vec<Station> = rows.into_iter().map(|row| row.into()).collect();

        Ok(stations)
    }

    async fn get_by_line_group_id(
        line_group_id: i32,
        conn: &mut PgConnection,
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
            COALESCE(a.line_name, l.line_name) AS line_name,
            COALESCE(a.line_name_k, l.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, l.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, l.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, l.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, l.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, l.line_color_c) AS line_color_c,
            COALESCE(s.station_cd = sst.station_cd, FALSE) AS has_train_types,
            sst.type_cd AS "type_cd?",
            sst.line_group_cd AS "line_group_cd?",
            sst.pass AS "pass?",
            sst.id AS "sst_id?",
            t.type_name AS "type_name?",
            t.type_name_k AS "type_name_k?",
            t.type_name_r AS "type_name_r?",
            t.type_name_zh AS "type_name_zh?",
            t.type_name_ko AS "type_name_ko?",
            t.color AS "color?",
            t.kind AS "kind?",
            t.direction AS "direction?",
            0.0::double precision AS distance
          FROM stations AS s
          JOIN lines AS l ON l.line_cd = s.line_cd AND l.e_status = 0
          LEFT JOIN station_station_types AS sst ON sst.line_group_cd = $1
          LEFT JOIN types AS t ON t.type_cd = sst.type_cd
          LEFT JOIN line_aliases AS la ON la.station_cd = s.station_cd
          LEFT JOIN aliases AS a ON la.alias_cd = a.id
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
}
