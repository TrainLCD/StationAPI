use async_trait::async_trait;
use fake::Dummy;
use moka::sync::Cache;
use sqlx::{MySql, MySqlConnection, Pool};

use crate::domain::{
    entity::train_type::TrainType, error::DomainError,
    repository::train_type_repository::TrainTypeRepository,
};

#[derive(sqlx::FromRow, Clone, Dummy)]
pub struct TrainTypeRow {
    id: u32,
    station_cd: u32,
    type_cd: u32,
    line_group_cd: u32,
    pass: u32,
    type_name: String,
    type_name_k: String,
    type_name_r: String,
    type_name_zh: String,
    type_name_ko: String,
    color: String,
    direction: u32,
}

impl From<TrainTypeRow> for TrainType {
    fn from(row: TrainTypeRow) -> Self {
        let TrainTypeRow {
            id,
            station_cd,
            type_cd,
            line_group_cd,
            pass,
            type_name,
            type_name_k,
            type_name_r,
            type_name_zh,
            type_name_ko,
            color,
            direction,
        } = row;
        Self {
            id,
            station_cd,
            type_cd,
            line_group_cd,
            pass,
            type_name,
            type_name_k,
            type_name_r,
            type_name_zh,
            type_name_ko,
            color,
            direction,
            lines: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct MyTrainTypeRepository {
    pool: Pool<MySql>,
    cache: Cache<String, Vec<TrainType>>,
}

impl MyTrainTypeRepository {
    pub fn new(pool: Pool<MySql>, cache: Cache<String, Vec<TrainType>>) -> Self {
        Self { pool, cache }
    }
}

#[async_trait]
impl TrainTypeRepository for MyTrainTypeRepository {
    async fn find_by_line_group_id(
        &self,
        line_group_id: u32,
    ) -> Result<Option<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::find_by_line_group_id(line_group_id, &mut conn, &self.cache)
            .await
    }

    async fn get_by_station_id(&self, station_id: u32) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_station_id(station_id, &mut conn, &self.cache).await
    }
}

pub struct InternalTrainTypeRepository {}

impl InternalTrainTypeRepository {
    async fn find_by_line_group_id(
        line_group_id: u32,
        conn: &mut MySqlConnection,
        cache: &Cache<String, Vec<TrainType>>,
    ) -> Result<Option<TrainType>, DomainError> {
        let cache_key = format!(
            "train_type_repository:find_by_line_group_id:{}",
            line_group_id
        );
        if let Some(cache_data) = cache.get(&cache_key) {
            if let Some(cache_data) = cache_data.first() {
                return Ok(Some(cache_data.clone()));
            }
        };

        let rows: Option<TrainTypeRow> = sqlx::query_as(
            "SELECT t.*, sst.*
            FROM types as t, station_station_types as sst
            WHERE sst.line_group_cd = ?
              AND t.type_cd = sst.type_cd
            LIMIT 1",
        )
        .bind(line_group_id)
        .fetch_optional(conn)
        .await?;
        let train_type: Option<TrainType> = rows.map(|row| row.into());

        if let Some(train_type) = train_type.clone() {
            cache.insert(cache_key, vec![train_type]);
        }

        Ok(train_type)
    }
    async fn get_by_station_id(
        station_id: u32,
        conn: &mut MySqlConnection,
        cache: &Cache<String, Vec<TrainType>>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let cache_key = format!("train_type_repository:get_by_station_id:{}", station_id);
        if let Some(cache_data) = cache.get(&cache_key) {
            return Ok(cache_data);
        };

        let rows: Vec<TrainTypeRow> = sqlx::query_as(
            "SELECT t.*, sst.*
            FROM station_station_types as sst, stations as s, types as t
            WHERE s.station_cd = ?
            AND s.station_cd = sst.station_cd
            AND sst.type_cd = t.type_cd
            AND s.e_status = 0",
        )
        .bind(station_id)
        .fetch_all(conn)
        .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        cache.insert(cache_key, train_types.clone());

        Ok(train_types)
    }
}
