use async_trait::async_trait;

use bigdecimal::{BigDecimal, ToPrimitive};
use fake::Dummy;
use sqlx::{MySql, MySqlConnection, Pool};

use crate::domain::{
    entity::line::Line, error::DomainError, repository::line_repository::LineRepository,
};

#[derive(sqlx::FromRow, Clone, Dummy)]
pub struct LineRow {
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

impl From<LineRow> for Line {
    fn from(row: LineRow) -> Self {
        Self {
            line_cd: row.line_cd,
            company_cd: row.company_cd,
            company: None,
            line_name: row.line_name,
            line_name_k: row.line_name_k,
            line_name_h: row.line_name_h,
            line_name_r: row.line_name_r,
            line_name_zh: row.line_name_zh,
            line_name_ko: row.line_name_ko,
            line_color_c: row.line_color_c,
            line_color_t: row.line_color_t,
            line_type: row.line_type,
            line_symbols: vec![],
            line_symbol_primary: row.line_symbol_primary,
            line_symbol_secondary: row.line_symbol_secondary,
            line_symbol_extra: row.line_symbol_extra,
            line_symbol_primary_color: row.line_symbol_primary_color,
            line_symbol_secondary_color: row.line_symbol_secondary_color,
            line_symbol_extra_color: row.line_symbol_extra_color,
            line_symbol_primary_shape: row.line_symbol_primary_shape,
            line_symbol_secondary_shape: row.line_symbol_secondary_shape,
            line_symbol_extra_shape: row.line_symbol_extra_shape,
            lon: row.lon.to_f64().unwrap_or(0.0),
            lat: row.lat.to_f64().unwrap_or(0.0),
            zoom: row.zoom,
            e_status: row.e_status,
            e_sort: row.e_sort,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MyLineRepository {
    pool: Pool<MySql>,
}

impl MyLineRepository {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LineRepository for MyLineRepository {
    async fn find_by_id(&self, id: u32) -> Result<Option<Line>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::find_by_id(id, &mut conn).await
    }
    async fn get_by_ids(&self, ids: Vec<u32>) -> Result<Vec<Line>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_ids(ids, &mut conn).await
    }
    async fn get_by_station_group_id(&self, id: u32) -> Result<Vec<Line>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalLineRepository::get_by_station_group_id(id, &mut conn).await
    }
}

pub struct InternalLineRepository {}

impl InternalLineRepository {
    async fn find_by_id(id: u32, conn: &mut MySqlConnection) -> Result<Option<Line>, DomainError> {
        let rows: Option<LineRow> =
            sqlx::query_as("SELECT * FROM `lines` WHERE line_cd = ? AND e_status = 0")
                .bind(id)
                .fetch_optional(conn)
                .await?;
        Ok(rows.map(|row| row.into()))
    }

    async fn get_by_ids(
        ids: Vec<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Line>, DomainError> {
        let params = format!("?{}", ", ?".repeat(ids.len() - 1));
        let query_str = format!(
            "SELECT * FROM `lines` WHERE line_cd IN ( {} ) AND e_status = 0",
            params
        );

        let mut query = sqlx::query_as::<_, LineRow>(&query_str);
        for id in ids {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        Ok(rows.into_iter().map(|row| row.into()).collect())
    }

    async fn get_by_station_group_id(
        station_group_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Line>, DomainError> {
        let rows: Vec<LineRow> =
            sqlx::query_as("SELECT * FROM `lines` WHERE line_cd IN (SELECT line_cd FROM stations WHERE station_g_cd = ? AND e_status = 0) AND e_status = 0")
                .bind(station_group_id)
                .fetch_all(conn)
                .await?;
        let lines = rows.into_iter().map(|row| row.into()).collect();
        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use bigdecimal::ToPrimitive;
    use fake::{Fake, Faker};

    use crate::domain::entity::line::Line;

    use super::LineRow;

    #[test]
    fn from_line_row() {
        let row: LineRow = Faker.fake();
        let LineRow {
            line_cd,
            company_cd,
            line_name,
            line_name_k,
            line_name_h,
            line_name_r,
            line_name_zh,
            line_name_ko,
            line_color_c,
            line_color_t,
            line_type,
            line_symbol_primary,
            line_symbol_secondary,
            line_symbol_extra,
            line_symbol_primary_color,
            line_symbol_secondary_color,
            line_symbol_extra_color,
            line_symbol_primary_shape,
            line_symbol_secondary_shape,
            line_symbol_extra_shape,
            lon,
            lat,
            zoom,
            e_status,
            e_sort,
        } = row.clone();
        let actual = Line::from(row);

        assert_eq!(actual.line_cd, line_cd);
        assert_eq!(actual.company_cd, company_cd);
        assert_eq!(actual.line_name, line_name);
        assert_eq!(actual.line_name_k, line_name_k);
        assert_eq!(actual.line_name_h, line_name_h);
        assert_eq!(actual.line_name_r, line_name_r);
        assert_eq!(actual.line_name_zh, line_name_zh);
        assert_eq!(actual.line_name_ko, line_name_ko);
        assert_eq!(actual.line_color_c, line_color_c);
        assert_eq!(actual.line_color_t, line_color_t);
        assert_eq!(actual.line_type, line_type);
        assert_eq!(actual.line_symbol_primary, line_symbol_primary);
        assert_eq!(actual.line_symbol_secondary, line_symbol_secondary);
        assert_eq!(actual.line_symbol_extra, line_symbol_extra);
        assert_eq!(actual.line_symbol_primary_color, line_symbol_primary_color);
        assert_eq!(
            actual.line_symbol_secondary_color,
            line_symbol_secondary_color
        );
        assert_eq!(actual.line_symbol_extra_color, line_symbol_extra_color);
        assert_eq!(actual.line_symbol_primary_shape, line_symbol_primary_shape);
        assert_eq!(
            actual.line_symbol_secondary_shape,
            line_symbol_secondary_shape
        );
        assert_eq!(actual.line_symbol_extra_shape, line_symbol_extra_shape);
        assert_eq!(Some(actual.lon), lon.to_f64());
        assert_eq!(Some(actual.lat), lat.to_f64());
        assert_eq!(actual.zoom, zoom);
        assert_eq!(actual.e_status, e_status);
        assert_eq!(actual.e_sort, e_sort);
    }
}
