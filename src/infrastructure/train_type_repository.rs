use std::sync::Arc;

use crate::domain::{
    entity::train_type::TrainType, error::DomainError,
    repository::train_type_repository::TrainTypeRepository,
};
use async_trait::async_trait;
use fake::Dummy;
use moka::future::Cache;
use sqlx::{MySql, MySqlConnection, Pool};

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
            line: None,
            lines: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct MyTrainTypeRepository {
    pool: Pool<MySql>,
    cache: Cache<String, Arc<Vec<TrainType>>>,
}

impl MyTrainTypeRepository {
    pub fn new(pool: Pool<MySql>, cache: Cache<String, Arc<Vec<TrainType>>>) -> Self {
        Self { pool, cache }
    }
}

#[async_trait]
impl TrainTypeRepository for MyTrainTypeRepository {
    async fn get_by_line_group_id(
        &self,
        line_group_id: u32,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id(line_group_id, &mut conn, &self.cache)
            .await
    }

    async fn get_by_station_id(&self, station_id: u32) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_station_id(station_id, &mut conn, &self.cache).await
    }

    async fn find_by_line_group_id_and_line_id(
        &self,
        line_group_id: u32,
        line_id: u32,
    ) -> Result<Option<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id_and_line_id(
            line_group_id,
            line_id,
            &mut conn,
            &self.cache,
        )
        .await
    }
}

pub struct InternalTrainTypeRepository {}

impl InternalTrainTypeRepository {
    async fn get_by_line_group_id(
        line_group_id: u32,
        conn: &mut MySqlConnection,
        cache: &Cache<String, Arc<Vec<TrainType>>>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let cache_key = format!(
            "train_type_repository:get_by_line_group_id:{}",
            line_group_id
        );
        if let Some(cache_data) = cache.get(&cache_key) {
            return Ok(Arc::clone(&cache_data).to_vec());
        };

        let rows: Vec<TrainTypeRow> = sqlx::query_as(
            "SELECT t.*, sst.*
            FROM types as t, station_station_types as sst
            WHERE sst.line_group_cd = ?
              AND t.type_cd = sst.type_cd",
        )
        .bind(line_group_id)
        .fetch_all(conn)
        .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        let train_types_with_arc = Arc::new(train_types);
        cache.insert(cache_key.clone(), train_types_with_arc).await;

        cache
            .get(&cache_key)
            .map(|train_type| Arc::clone(&train_type).to_vec())
            .ok_or(DomainError::Unexpected(
                "Failed to caching for response.".to_string(),
            ))
    }
    async fn get_by_station_id(
        station_id: u32,
        conn: &mut MySqlConnection,
        cache: &Cache<String, Arc<Vec<TrainType>>>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let cache_key = format!("train_type_repository:get_by_station_id:{}", station_id);
        if let Some(cache_data) = cache.get(&cache_key) {
            return Ok(Arc::clone(&cache_data).to_vec());
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

        let train_types_with_arc = Arc::new(train_types);
        cache.insert(cache_key.clone(), train_types_with_arc).await;

        cache
            .get(&cache_key)
            .map(|train_type| Arc::clone(&train_type).to_vec())
            .ok_or(DomainError::Unexpected(
                "Failed to caching for response.".to_string(),
            ))
    }
    async fn get_by_line_group_id_and_line_id(
        line_group_id: u32,
        line_id: u32,
        conn: &mut MySqlConnection,
        cache: &Cache<String, Arc<Vec<TrainType>>>,
    ) -> Result<Option<TrainType>, DomainError> {
        let cache_key = format!(
            "train_type_repository:get_by_line_group_id_and_line_id:{}:{}",
            line_group_id, line_id
        );
        if let Some(cache_data) = cache.get(&cache_key) {
            return Ok(Arc::clone(&cache_data).first().cloned());
        };

        let rows: Option<TrainTypeRow> = sqlx::query_as(
            "SELECT t.*, sst.*
            FROM types as t, station_station_types as sst
            WHERE sst.line_group_cd = ?
            AND sst.station_cd IN (
                SELECT station_cd
                FROM stations as s
                WHERE line_cd =?
            )
            AND t.type_cd = sst.type_cd",
        )
        .bind(line_group_id)
        .bind(line_id)
        .fetch_optional(conn)
        .await?;

        let train_type = rows.map(|row| row.into());

        let Some(train_type) = train_type else {
            return Ok(None);
        };

        let train_type_with_arc = Arc::new(vec![train_type]);

        cache.insert(cache_key.clone(), train_type_with_arc).await;

        if let Some(cache_data) = cache.get(&cache_key) {
            if let Some(cache_data) = cache_data.first() {
                return Ok(Some(cache_data.clone()));
            }
        };
        Ok(None)
    }
}
