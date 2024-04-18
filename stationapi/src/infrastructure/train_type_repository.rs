use crate::domain::{
    entity::train_type::TrainType, error::DomainError,
    repository::train_type_repository::TrainTypeRepository,
};
use async_trait::async_trait;
use sqlx::{MySql, MySqlConnection, Pool};

#[derive(sqlx::FromRow, Clone)]
pub struct TrainTypeRow {
    id: u32,
    station_cd: u32,
    type_cd: u32,
    line_group_cd: u32,
    pass: u32,
    type_name: String,
    type_name_k: String,
    type_name_r: Option<String>,
    type_name_zh: Option<String>,
    type_name_ko: Option<String>,
    color: String,
    direction: u32,
    kind: u32,
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

#[derive(Debug, Clone)]
pub struct MyTrainTypeRepository {
    pool: Pool<MySql>,
}

impl MyTrainTypeRepository {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TrainTypeRepository for MyTrainTypeRepository {
    async fn get_by_line_group_id(
        &self,
        line_group_id: u32,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id(line_group_id, &mut conn).await
    }

    async fn get_by_station_id(&self, station_id: u32) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_station_id(station_id, &mut conn).await
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
        )
        .await
    }

    async fn get_types_by_station_id_vec(
        &self,
        station_id_vec: Vec<u32>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_types_by_station_id_vec(station_id_vec, &mut conn).await
    }
}

pub struct InternalTrainTypeRepository {}

impl InternalTrainTypeRepository {
    async fn get_by_line_group_id(
        line_group_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        let rows: Vec<TrainTypeRow> = sqlx::query_as(
            "SELECT 
            t.*, 
            sst.*
          FROM 
            types as t, 
            station_station_types as sst 
          WHERE 
            sst.line_group_cd = ? 
            AND t.type_cd = sst.type_cd",
        )
        .bind(line_group_id)
        .fetch_all(conn)
        .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
    async fn get_by_station_id(
        station_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        let rows: Vec<TrainTypeRow> = sqlx::query_as(
            "SELECT 
            t.*, 
            sst.*
          FROM 
            station_station_types as sst, 
            stations as s, 
            types as t 
          WHERE 
            s.station_cd = ? 
            AND s.station_cd = sst.station_cd 
            AND sst.type_cd = t.type_cd 
            AND s.e_status = 0 
            AND sst.pass <> 1",
        )
        .bind(station_id)
        .fetch_all(conn)
        .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
    async fn get_by_line_group_id_and_line_id(
        line_group_id: u32,
        line_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Option<TrainType>, DomainError> {
        let rows: Option<TrainTypeRow> = sqlx::query_as(
            "SELECT 
            t.*, 
            sst.*
          FROM 
            types as t, 
            station_station_types as sst 
          WHERE 
            sst.line_group_cd = ? 
            AND sst.station_cd IN (
              SELECT 
                station_cd 
              FROM 
                stations as s 
              WHERE 
                line_cd = ?
                AND s.e_status = 0
            ) 
            AND t.type_cd = sst.type_cd",
        )
        .bind(line_group_id)
        .bind(line_id)
        .fetch_optional(conn)
        .await?;

        let train_type: Option<TrainType> = rows.map(|row| row.into());

        let Some(train_type) = train_type else {
            return Ok(None);
        };

        Ok(Some(train_type))
    }

    async fn get_types_by_station_id_vec(
        station_id_vec: Vec<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        if station_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(station_id_vec.len() - 1));
        let query_str = format!(
            "SELECT 
            t.*, 
            sst.*
          FROM 
            station_station_types as sst, 
            stations as s, 
            types as t 
          WHERE 
            s.station_cd IN ( {} ) 
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
            AND sst.pass <> 1
            ORDER BY sst.id",
            params
        );

        let mut query = sqlx::query_as::<_, TrainTypeRow>(&query_str);
        for id in station_id_vec {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
}
