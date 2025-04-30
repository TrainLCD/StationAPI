use crate::domain::{
    entity::train_type::TrainType, error::DomainError,
    repository::train_type_repository::TrainTypeRepository,
};
use async_trait::async_trait;
use sqlx::{PgConnection, Pool, Postgres};
use std::sync::Arc;

#[derive(sqlx::FromRow, Clone)]
pub struct TrainTypeRow {
    id: Option<i64>,
    station_cd: Option<i64>,
    type_cd: Option<i64>,
    line_group_cd: Option<i64>,
    pass: Option<i64>,
    type_name: String,
    type_name_k: String,
    type_name_r: Option<String>,
    type_name_zh: Option<String>,
    type_name_ko: Option<String>,
    color: String,
    direction: Option<i64>,
    kind: Option<i64>,
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
            kind,
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
            kind,
        }
    }
}

pub struct MyTrainTypeRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyTrainTypeRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TrainTypeRepository for MyTrainTypeRepository {
    async fn get_by_line_group_id(
        &self,
        line_group_id: i64,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id(line_group_id, &mut conn).await
    }

    async fn get_by_station_id(&self, station_id: i64) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_station_id(station_id, &mut conn).await
    }

    async fn find_by_line_group_id_and_line_id(
        &self,
        line_group_id: i64,
        line_id: i64,
    ) -> Result<Option<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id_and_line_id(
            line_group_id,
            line_id,
            &mut conn,
        )
        .await
    }

    async fn get_by_station_id_vec(
        &self,
        station_id_vec: Vec<i64>,
        line_group_id: Option<i64>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_station_id_vec(station_id_vec, line_group_id, &mut conn)
            .await
    }

    async fn get_types_by_station_id_vec(
        &self,
        station_id_vec: Vec<i64>,
        line_group_id: Option<i64>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_types_by_station_id_vec(
            station_id_vec,
            line_group_id,
            &mut conn,
        )
        .await
    }

    async fn get_by_line_group_id_vec(
        &self,
        line_group_id_vec: Vec<i64>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id_vec(line_group_id_vec, &mut conn).await
    }
}

pub struct InternalTrainTypeRepository {}

impl InternalTrainTypeRepository {
    async fn get_by_line_group_id(
        line_group_id: i64,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        let rows = sqlx::query_as!(
            TrainTypeRow,
            "SELECT
            t.id,
            t.type_cd,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            sst.station_cd,
            sst.line_group_cd,
            sst.pass
            FROM types as t
            JOIN station_station_types AS sst ON sst.line_group_cd = $1
            WHERE 
                t.type_cd = sst.type_cd
            ORDER BY t.kind, sst.id",
            line_group_id
        )
        .fetch_all(conn)
        .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
    async fn get_by_station_id(
        station_id: i64,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        let rows = sqlx::query_as!(TrainTypeRow,
            "SELECT
            t.id,
            t.type_cd,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            s.station_cd,
            sst.line_group_cd,
            sst.pass
            FROM  types AS t
            JOIN stations AS s ON s.station_cd = $1 AND s.e_status = 0
            JOIN station_station_types AS sst ON sst.station_cd = s.station_cd AND sst.type_cd = t.type_cd AND sst.pass <> 1
            ORDER BY sst.id",
            station_id
        )
        .fetch_all(conn)
        .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
    async fn get_by_line_group_id_and_line_id(
        line_group_id: i64,
        line_id: i64,
        conn: &mut PgConnection,
    ) -> Result<Option<TrainType>, DomainError> {
        let rows = sqlx::query_as!(
            TrainTypeRow,
            "SELECT 
            t.id,
            t.type_cd,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            sst.station_cd,
            sst.line_group_cd,
            sst.pass
            FROM types as t
            JOIN station_station_types AS sst ON sst.line_group_cd = $1
            WHERE 
            sst.station_cd = ANY(
                SELECT 
                station_cd 
                FROM 
                stations
                WHERE 
                line_cd = $2
                AND e_status = 0
            )
            AND t.type_cd = sst.type_cd
            ORDER BY sst.id",
            line_group_id,
            line_id,
        )
        .fetch_optional(conn)
        .await?;

        let train_type: Option<TrainType> = rows.map(|row| row.into());

        let Some(train_type) = train_type else {
            return Ok(None);
        };

        Ok(Some(train_type))
    }

    async fn get_by_station_id_vec(
        station_id_vec: Vec<i64>,
        line_group_id: Option<i64>,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        if station_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let rows = sqlx::query_as!(TrainTypeRow,
            "SELECT 
            t.id,
            t.type_cd,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            s.station_cd,
            sst.line_group_cd,
            sst.pass
            FROM 
            types as t
            JOIN stations AS s ON s.station_cd IN (SELECT unnest($1::bigint[])) AND s.e_status = 0
            JOIN station_station_types AS sst ON sst.line_group_cd = $2 AND sst.pass <> 1 AND sst.type_cd = t.type_cd
            WHERE sst.pass <> 1 AND sst.type_cd = t.type_cd
            ORDER BY sst.id",
            &station_id_vec,
            line_group_id
        ).fetch_all(conn).await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }

    async fn get_types_by_station_id_vec(
        station_id_vec: Vec<i64>,
        line_group_id: Option<i64>,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        if station_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let rows = sqlx::query_as!(
            TrainTypeRow,
            "SELECT 
            t.id,
            t.type_cd,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            s.station_cd,
            sst.line_group_cd,
            sst.pass
            FROM 
            station_station_types as sst, 
            stations as s, 
            types as t 
            WHERE 
            s.station_cd IN (SELECT unnest($1::bigint[]))
            AND CASE WHEN t.top_priority = 1
            THEN
                sst.type_cd = t.type_cd
            ELSE
                sst.pass <> 1
                AND sst.type_cd = t.type_cd
            END
            AND s.station_cd = sst.station_cd
            AND sst.type_cd = t.type_cd 
            AND s.e_status = 0
            AND sst.line_group_cd = $2
            AND sst.pass <> 1
            ORDER BY t.kind, sst.id",
            &station_id_vec,
            line_group_id
        )
        .fetch_all(conn)
        .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }

    async fn get_by_line_group_id_vec(
        line_group_id_vec: Vec<i64>,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        if line_group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let rows = sqlx::query_as!(
            TrainTypeRow,
            "SELECT 
            t.id,
            t.type_cd,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            s.station_cd,
            sst.line_group_cd,
            sst.pass
            FROM 
            types as t
            JOIN station_station_types AS sst ON sst.line_group_cd IN (SELECT unnest($1::bigint[])) AND sst.pass <> 1 AND sst.type_cd = t.type_cd
            JOIN stations AS s ON s.station_cd = sst.station_cd
            WHERE sst.pass <> 1 AND sst.type_cd = t.type_cd
            ORDER BY sst.id",
            &line_group_id_vec)
            .fetch_all(conn).await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
}
