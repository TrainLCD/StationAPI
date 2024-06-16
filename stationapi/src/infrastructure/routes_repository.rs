use std::rc::Rc;

use crate::{
    domain::{error::DomainError, repository::routes_repository::RoutesRepository},
    station_api::{Station, TrainType},
};
use async_trait::async_trait;
use sqlx::{PgConnection, PgPool};

#[derive(Default, sqlx::FromRow)]
#[sqlx(default)]
pub struct RouteRow {
    // stations
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
    pub distance: Option<f64>,
    // lines
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
    pub average_distance: Option<f64>,
    // types
    sst_id: Option<i32>,
    pub type_id: Option<i32>,
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
    pub has_train_types: Option<bool>,
}

impl From<RouteRow> for TrainType {
    fn from(row: RouteRow) -> Self {
        Self {
            id: *row.type_id.as_ref().unwrap(),
            type_id: *row.type_cd.as_ref().unwrap(),
            group_id: *row.line_group_cd.as_ref().unwrap(),
            name: row.type_name.as_ref().unwrap().to_string(),
            name_katakana: row.type_name_k.as_ref().unwrap().to_string(),
            name_roman: row.type_name_r,
            name_chinese: row.type_name_zh,
            name_korean: row.type_name_ko,
            color: row.color.as_ref().unwrap().to_string(),
            lines: vec![],
            line: None,
            direction: row.direction.unwrap_or(0),
            kind: row.kind.unwrap_or(0),
        }
    }
}

impl From<Rc<&RouteRow>> for Station {
    fn from(row: Rc<&RouteRow>) -> Self {
        Self {
            id: Rc::clone(&row).station_cd.unwrap(),
            group_id: Rc::clone(&row).station_g_cd.unwrap(),
            name: Rc::clone(&row).station_name.as_ref().unwrap().to_string(),
            name_katakana: Rc::clone(&row).station_name_k.as_ref().unwrap().to_string(),
            name_roman: Some(Rc::clone(&row).station_name_r.as_ref().unwrap().to_string()),
            name_chinese: Some(
                Rc::clone(&row)
                    .station_name_zh
                    .as_ref()
                    .unwrap_or(&"".to_string())
                    .to_string(),
            ),
            name_korean: Some(
                Rc::clone(&row)
                    .station_name_ko
                    .as_ref()
                    .unwrap_or(&"".to_string())
                    .to_string(),
            ),
            three_letter_code: Some(
                Rc::clone(&row)
                    .three_letter_code
                    .as_ref()
                    .unwrap_or(&"".to_string())
                    .to_string(),
            ),
            lines: vec![],
            line: None,
            prefecture_id: Rc::clone(&row).pref_cd.unwrap(),
            postal_code: Rc::clone(&row).post.as_ref().unwrap().to_string(),
            address: Rc::clone(&row).address.as_ref().unwrap().to_string(),
            latitude: Rc::clone(&row).lat.unwrap(),
            longitude: Rc::clone(&row).lon.unwrap(),
            opened_at: Rc::clone(&row).open_ymd.as_ref().unwrap().to_string(),
            closed_at: Rc::clone(&row).close_ymd.as_ref().unwrap().to_string(),
            status: Rc::clone(&row).e_status.unwrap(),
            station_numbers: vec![],
            stop_condition: Rc::clone(&row).pass.unwrap_or(0),
            distance: Rc::clone(&row).distance.unwrap_or_default().into(),
            has_train_types: Rc::clone(&row).has_train_types,
            train_type: None,
        }
    }
}

#[derive(Debug)]
pub struct MyRoutesRepository {
    pool: PgPool,
}

impl MyRoutesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoutesRepository for MyRoutesRepository {
    async fn get_routes(
        &self,
        from_station_id: i32,
        to_station_id: i32,
    ) -> Result<Vec<RouteRow>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalRoutesRepository::get_routes(from_station_id, to_station_id, &mut conn).await
    }
}

pub struct InternalRoutesRepository {}

impl InternalRoutesRepository {
    async fn get_routes(
        from_station_id: i32,
        to_station_id: i32,
        conn: &mut PgConnection,
    ) -> Result<Vec<RouteRow>, DomainError> {
        let rows = sqlx::query_as!(
            RouteRow,
            r#"SELECT
            sta.*,
            via_lines.company_cd,
            via_lines.line_type,
            via_lines.line_symbol_primary,
            via_lines.line_symbol_secondary,
            via_lines.line_symbol_extra,
            via_lines.line_symbol_primary_color,
            via_lines.line_symbol_secondary_color,
            via_lines.line_symbol_extra_color,
            via_lines.line_symbol_primary_shape,
            via_lines.line_symbol_secondary_shape,
            via_lines.line_symbol_extra_shape,
            via_lines.average_distance,
            sst.id AS sst_id,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass,
            types.id AS type_id,
            types.type_name,
            types.type_name_k,
            types.type_name_r,
            types.type_name_zh,
            types.type_name_ko,
            types.color,
            types.kind AS "kind?",
            types.direction AS "direction?",
            COALESCE(a.line_name, via_lines.line_name) AS line_name,
            COALESCE(a.line_name_k, via_lines.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, via_lines.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, via_lines.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, via_lines.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, via_lines.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, via_lines.line_color_c) AS line_color_c,
            COALESCE(sta.station_cd = sst.station_cd,FALSE) AS has_train_types,
            0.0::double precision AS distance
        FROM stations AS sta
            JOIN station_station_types AS sst ON sta.station_cd = sst.station_cd
            AND sst.pass <> 1
            AND sst.line_group_cd IN (
                SELECT _sst.line_group_cd
                FROM station_station_types AS _sst
                WHERE _sst.station_cd IN (
                        SELECT s.station_cd
                        FROM stations AS s
                        WHERE s.station_g_cd = $1
                    )
                    AND _sst.pass <> 1
            )
            AND sst.line_group_cd IN (
                SELECT _sst.line_group_cd
                FROM station_station_types AS _sst
                WHERE _sst.station_cd IN (
                        SELECT s.station_cd
                        FROM stations AS s
                        WHERE s.station_g_cd = $2
                    )
                    AND _sst.pass <> 1
            )
            AND sta.station_cd = sst.station_cd
            JOIN types ON sst.type_cd = types.type_cd
            JOIN lines AS via_lines ON sta.line_cd = via_lines.line_cd
            LEFT JOIN line_aliases AS la ON la.station_cd = sta.station_cd
            LEFT JOIN aliases AS a ON a.id = la.alias_cd
        UNION
        SELECT
        sta.*,
            via_lines.company_cd,
            via_lines.line_type,
            via_lines.line_symbol_primary,
            via_lines.line_symbol_secondary,
            via_lines.line_symbol_extra,
            via_lines.line_symbol_primary_color,
            via_lines.line_symbol_secondary_color,
            via_lines.line_symbol_extra_color,
            via_lines.line_symbol_primary_shape,
            via_lines.line_symbol_secondary_shape,
            via_lines.line_symbol_extra_shape,
            via_lines.average_distance,
            sst.id AS sst_id,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass,
            types.id AS type_id,
            types.type_name,
            types.type_name_k,
            types.type_name_r,
            types.type_name_zh,
            types.type_name_ko,
            types.color,
            types.kind AS "kind?",
            types.direction AS "direction?",
            COALESCE(a.line_name, via_lines.line_name) AS line_name,
            COALESCE(a.line_name_k, via_lines.line_name_k) AS line_name_k,
            COALESCE(a.line_name_h, via_lines.line_name_h) AS line_name_h,
            COALESCE(a.line_name_r, via_lines.line_name_r) AS line_name_r,
            COALESCE(a.line_name_zh, via_lines.line_name_zh) AS line_name_zh,
            COALESCE(a.line_name_ko, via_lines.line_name_ko) AS line_name_ko,
            COALESCE(a.line_color_c, via_lines.line_color_c) AS line_color_c,
            COALESCE(sta.station_cd = sst.station_cd,FALSE) AS has_train_types,
            0.0 AS distance
        FROM stations AS sta
            LEFT JOIN station_station_types AS sst ON sst.station_cd = NULL
            LEFT JOIN types ON types.type_cd = NULL
            JOIN lines AS via_lines ON via_lines.line_cd IN (
                SELECT s.line_cd
                FROM stations AS s
                WHERE s.station_g_cd = $1
            )
            AND via_lines.line_cd IN (
                SELECT s.line_cd
                FROM stations AS s
                WHERE s.station_g_cd = $2
            )
            LEFT JOIN line_aliases AS la ON la.station_cd = sta.station_cd
            LEFT JOIN aliases AS a ON a.id = la.alias_cd
        WHERE sta.line_cd = via_lines.line_cd
        ORDER BY sst_id, e_sort, station_cd"#,
            from_station_id,
            to_station_id,
        )
        .fetch_all(conn)
        .await?;

        Ok(rows)
    }
}
