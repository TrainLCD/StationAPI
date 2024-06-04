use async_trait::async_trait;
use sqlx::{MySql, MySqlConnection, Pool};

use crate::{
    domain::{error::DomainError, repository::routes_repository::RoutesRepository},
    station_api::{Station, TrainType},
};

#[derive(sqlx::FromRow, Clone)]
pub struct RouteRow {
    // stations
    pub station_cd: u32,
    pub station_g_cd: u32,
    pub station_name: String,
    pub station_name_k: String,
    pub station_name_r: Option<String>,
    pub station_name_zh: Option<String>,
    pub station_name_ko: Option<String>,
    pub primary_station_number: Option<String>,
    pub secondary_station_number: Option<String>,
    pub extra_station_number: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: u32,
    pub pref_cd: u32,
    pub post: String,
    pub address: String,
    pub lon: f64,
    pub lat: f64,
    pub open_ymd: String,
    pub close_ymd: String,
    pub e_status: u32,
    pub e_sort: u32,
    // lines
    pub company_cd: u32,
    pub line_name: String,
    pub line_name_k: String,
    pub line_name_h: String,
    pub line_name_r: Option<String>,
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
    pub average_distance: f64,
    // types
    pub type_id: Option<u32>,
    pub type_cd: Option<u32>,
    pub line_group_cd: Option<u32>,
    pub pass: Option<u32>,
    pub type_name: Option<String>,
    pub type_name_k: Option<String>,
    pub type_name_r: Option<String>,
    pub type_name_zh: Option<String>,
    pub type_name_ko: Option<String>,
    pub color: Option<String>,
    pub direction: Option<u32>,
    pub kind: Option<u32>,
    pub has_train_types: i64,
}

impl From<RouteRow> for TrainType {
    fn from(row: RouteRow) -> Self {
        Self {
            id: row.type_id.unwrap_or_default(),
            type_id: row.type_cd.unwrap_or_default(),
            group_id: row.line_group_cd.unwrap_or_default(),
            name: row.type_name.unwrap_or_default(),
            name_katakana: row.type_name_k.unwrap_or_default(),
            name_roman: row.type_name_r,
            name_chinese: row.type_name_zh,
            name_korean: row.type_name_ko,
            color: row.color.unwrap_or_default(),
            lines: vec![],
            line: None,
            direction: row.direction.unwrap_or_default() as i32,
            kind: row.kind.unwrap_or_default() as i32,
        }
    }
}

impl From<RouteRow> for Station {
    fn from(row: RouteRow) -> Self {
        Self {
            id: row.station_cd,
            group_id: row.line_group_cd.unwrap_or_default(),
            name: row.station_name,
            name_katakana: row.station_name_k,
            name_roman: row.station_name_r,
            name_chinese: row.station_name_zh,
            name_korean: row.station_name_ko,
            three_letter_code: row.three_letter_code,
            lines: vec![],
            line: None,
            prefecture_id: row.pref_cd,
            postal_code: row.post,
            address: row.address,
            latitude: row.lat,
            longitude: row.lon,
            opened_at: row.open_ymd,
            closed_at: row.close_ymd,
            status: row.e_status as i32,
            station_numbers: vec![],
            stop_condition: row.pass.unwrap_or(0) as i32,
            distance: Some(0.0),
            has_train_types: Some(row.has_train_types != 0),
            train_type: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MyRoutesRepository {
    pool: Pool<MySql>,
}

impl MyRoutesRepository {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoutesRepository for MyRoutesRepository {
    async fn get_routes(
        &self,
        from_station_id: u32,
        to_station_id: u32,
    ) -> Result<Vec<RouteRow>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalRoutesRepository::get_routes(from_station_id, to_station_id, &mut conn).await
    }
}

pub struct InternalRoutesRepository {}

impl InternalRoutesRepository {
    async fn get_routes(
        from_station_id: u32,
        to_station_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<RouteRow>, DomainError> {
        let rows = sqlx::query_as!(
            RouteRow,
            "SELECT sta.*,
            via_lines.company_cd,
            via_lines.line_name,
            via_lines.line_name_k,
            via_lines.line_name_h,
            via_lines.line_name_r,
            via_lines.line_name_zh,
            via_lines.line_name_ko,
            via_lines.line_color_c,
            via_lines.line_type,
            line_symbol_primary,
            line_symbol_secondary,
            line_symbol_extra,
            line_symbol_primary_color,
            line_symbol_secondary_color,
            line_symbol_extra_color,
            line_symbol_primary_shape,
            line_symbol_secondary_shape,
            line_symbol_extra_shape,
            average_distance,
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
            types.direction,
            types.kind,
            IFNULL(sta.station_cd = sst.station_cd, 0) AS has_train_types
        FROM stations AS sta
            LEFT JOIN station_station_types AS sst ON sta.station_cd = sst.station_cd
            AND sst.pass <> 1
            LEFT JOIN types ON sst.type_cd = types.type_cd
            JOIN `lines` AS via_lines ON sta.line_cd = via_lines.line_cd
        WHERE (
                (sst.station_cd IS NOT NULL)
                AND sst.line_group_cd IN (
                    SELECT _sst.line_group_cd
                    FROM station_station_types AS _sst
                    WHERE _sst.station_cd = ?
                )
                AND sst.line_group_cd IN (
                    SELECT _sst.line_group_cd
                    FROM station_station_types AS _sst
                    WHERE _sst.station_cd = ?
                )
                AND sta.line_cd = (
                    SELECT s.line_cd
                    FROM stations AS s
                    WHERE s.station_cd = sst.station_cd
                )
                AND sta.station_cd = sst.station_cd
                AND types.type_cd = sst.type_cd
            )
            OR (
                (sst.station_cd IS NULL)
                AND (
                    sta.line_cd = (
                        SELECT s.line_cd
                        FROM stations AS s
                        WHERE s.station_cd = ?
                    )
                    AND (
                        sta.line_cd = (
                            SELECT s.line_cd
                            FROM stations AS s
                            WHERE s.station_cd = ?
                        )
                    )
                )
            )
        ORDER BY CASE
                WHEN sst.station_cd IS NULL THEN sta.e_sort -- NOTE: sta.station_cdが指定できていないので順序がおかしい場合ここを疑う
                ELSE sst.id
            END",
            from_station_id,
            to_station_id,
            from_station_id,
            to_station_id,
        )
        .fetch_all(conn)
        .await?;

        Ok(rows)
    }
}
