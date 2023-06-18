use anyhow::Result;
use bigdecimal::{BigDecimal, ToPrimitive};
use sqlx::{MySql, Pool};

use crate::domain::models::line::{line_model::Line, line_repository::LineRepository};
#[derive(sqlx::FromRow, Clone)]
pub struct LineEntity {
    pub line_cd: u32,
    pub company_cd: u32,
    pub line_name: String,
    pub line_name_k: String,
    pub line_name_h: String,
    pub line_name_r: String,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: String,
    pub line_color_t: String,
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
    pub lon: BigDecimal,
    pub lat: BigDecimal,
    pub zoom: u32,
    pub e_status: u32,
    pub e_sort: u32,
}

impl From<LineEntity> for Line {
    fn from(entity: LineEntity) -> Line {
        Line {
            line_cd: entity.line_cd,
            company_cd: entity.company_cd,
            line_name: entity.line_name,
            line_name_k: entity.line_name_k,
            line_name_h: entity.line_name_h,
            line_name_r: entity.line_name_r,
            line_name_zh: entity.line_name_zh,
            line_name_ko: entity.line_name_ko,
            line_color_c: entity.line_color_c,
            line_color_t: entity.line_color_t,
            line_type: entity.line_type,
            line_symbol_primary: entity.line_symbol_primary,
            line_symbol_secondary: entity.line_symbol_secondary,
            line_symbol_extra: entity.line_symbol_extra,
            line_symbol_primary_color: entity.line_symbol_primary_color,
            line_symbol_secondary_color: entity.line_symbol_secondary_color,
            line_symbol_extra_color: entity.line_symbol_extra_color,
            line_symbol_primary_shape: entity.line_symbol_primary_shape,
            line_symbol_secondary_shape: entity.line_symbol_secondary_shape,
            line_symbol_extra_shape: entity.line_symbol_extra_shape,
            lon: entity.lon.to_f64().unwrap_or(0.0),
            lat: entity.lat.to_f64().unwrap_or(0.0),
            zoom: entity.zoom,
            e_status: entity.e_status,
            e_sort: entity.e_sort,
        }
    }
}

pub struct LineRepositoryImpl {
    pub pool: Box<Pool<MySql>>,
}

#[async_trait::async_trait]
impl LineRepository for LineRepositoryImpl {
    async fn find_by_id(&self, id: u32) -> Result<Line> {
        let result = sqlx::query_as!(
            LineEntity,
            "SELECT * FROM `lines` WHERE line_cd = ? AND e_status = 0",
            id
        )
        .fetch_one(self.pool.as_ref())
        .await;
        match result.map(|entity| entity.into()) {
            Ok(line) => Ok(line),
            Err(err) => Err(err.into()),
        }
    }
    async fn find_by_station_id(&self, station_id: u32) -> Result<Line> {
        let result = sqlx::query_as!(
            LineEntity,
            "SELECT l.* FROM `lines` AS l WHERE EXISTS
            (
            	SELECT line_cd FROM stations WHERE station_cd = ?
            	AND l.line_cd = line_cd
            	AND e_status = 0
            )
            ORDER BY l.e_sort, l.line_cd",
            station_id
        )
        .fetch_one(self.pool.as_ref())
        .await;

        match result {
            Ok(line) => Ok(line.into()),
            Err(err) => Err(err.into()),
        }
    }
    async fn get_by_station_group_id(&self, station_group_id: u32) -> Result<Vec<Line>> {
        let result = sqlx::query_as!(
            LineEntity,
            "SELECT l.* FROM `lines` AS l WHERE EXISTS
            (SELECT * FROM stations AS s1 WHERE s1.station_g_cd IN
            (SELECT station_g_cd FROM stations WHERE station_g_cd = ?)
            AND l.line_cd = s1.line_cd AND e_status = 0)
            ORDER BY l.e_sort, l.line_cd",
            station_group_id
        )
        .fetch_all(self.pool.as_ref())
        .await;

        match result {
            Ok(lines) => Ok(lines.into_iter().map(|line| line.into()).collect()),
            Err(err) => Err(err.into()),
        }
    }
}
