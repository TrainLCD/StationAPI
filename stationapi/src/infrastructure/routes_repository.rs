use std::collections::BTreeMap;

use async_trait::async_trait;
use sqlx::{MySql, MySqlConnection, Pool};

use crate::{
    domain::{error::DomainError, repository::routes_repository::RoutesRepository},
    station_api::{Route, Station, TrainType},
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
    // types
    pub id: u32,
    pub type_cd: u32,
    pub line_group_cd: u32,
    pub pass: u32,
    pub type_name: String,
    pub type_name_k: String,
    pub type_name_r: Option<String>,
    pub type_name_zh: Option<String>,
    pub type_name_ko: Option<String>,
    pub color: String,
    pub direction: u32,
    pub kind: u32,
}

impl From<RouteRow> for TrainType {
    fn from(row: RouteRow) -> Self {
        Self {
            id: row.id,
            type_id: row.type_cd,
            group_id: row.line_group_cd,
            name: row.type_name,
            name_katakana: row.type_name_k,
            name_roman: row.type_name_r,
            name_chinese: row.type_name_zh,
            name_korean: row.type_name_ko,
            color: row.color,
            lines: vec![],
            line: None,
            direction: row.direction as i32,
            kind: row.kind as i32,
        }
    }
}

impl From<RouteRow> for Station {
    fn from(row: RouteRow) -> Self {
        Self {
            id: row.station_cd,
            group_id: row.line_group_cd,
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
            stop_condition: row.pass as i32,
            distance: Some(0.0),
            has_train_types: Some(false),
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
    ) -> Result<Vec<Route>, DomainError> {
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
    ) -> Result<Vec<Route>, DomainError> {
        let rows: Vec<RouteRow> = sqlx::query_as(
            "SELECT via_stations.*,
            via_lines.*,
            sst.*,
            types.*
            FROM `stations`
            JOIN `station_station_types` AS sst
            ON sst.station_cd = ?
            JOIN `station_station_types` AS to_sst
            ON to_sst.station_cd = ?
            JOIN types
            ON sst.type_cd = types.type_cd
            JOIN `lines` AS via_lines
            ON via_lines.line_cd IN (
              SELECT stations.line_cd
              FROM stations
              JOIN station_station_types AS via_sst
              ON via_sst.line_group_cd = sst.line_group_cd
              WHERE stations.station_cd = via_sst.station_cd
            )
            JOIN `stations` AS via_stations
            ON via_stations.station_cd IN (
              SELECT station_cd
              FROM station_station_types AS via_sst
              WHERE via_stations.line_cd = via_lines.line_cd
              AND via_sst.type_cd = `types`.type_cd
              AND via_sst.pass <> 1
            )
            WHERE sst.line_group_cd = to_sst.line_group_cd
            AND stations.station_cd = sst.station_cd
            AND sst.pass <> 1
            AND to_sst.pass <> 1
            ORDER BY sst.line_group_cd",
        )
        .bind(from_station_id)
        .bind(to_station_id)
        .fetch_all(conn)
        .await?;

        let train_type_tree = rows.clone().into_iter().fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<u32, TrainType>, value| {
                acc.insert(value.line_group_cd, value.into());
                acc
            },
        );
        let stations_tree_map = rows.into_iter().fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<u32, Vec<Station>>, value| {
                acc.entry(value.line_group_cd)
                    .or_default()
                    .push(std::convert::Into::<Station>::into(value));
                acc
            },
        );

        let mut routes = vec![];

        for (line_group_cd, stops) in &stations_tree_map {
            routes.push(Route {
                train_type: train_type_tree.get(line_group_cd).cloned(),
                stops: stops.to_vec(),
            });
        }

        Ok(routes)
    }
}
