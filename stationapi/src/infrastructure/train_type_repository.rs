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
    type_cd: Option<u32>,
    line_group_cd: Option<u32>,
    pass: Option<u32>,
    type_name: Option<String>,
    type_name_k: Option<String>,
    type_name_r: Option<String>,
    type_name_zh: Option<String>,
    type_name_ko: Option<String>,
    color: Option<String>,
    direction: Option<u32>,
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

    async fn get_by_station_id_vec(
        &self,
        station_id_vec: Vec<u32>,
        line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_station_id_vec(station_id_vec, line_group_id, &mut conn)
            .await
    }

    async fn get_types_by_station_id_vec(
        &self,
        station_id_vec: Vec<u32>,
        line_group_id: Option<u32>,
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
        line_group_id_vec: Vec<u32>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id_vec(line_group_id_vec, &mut conn).await
    }
}

pub struct InternalTrainTypeRepository {}

impl InternalTrainTypeRepository {
    async fn get_by_line_group_id(
        line_group_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        let rows: Vec<TrainTypeRow> = sqlx::query_as!(
            TrainTypeRow,
            "SELECT
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            sst.*
            FROM types as t
            JOIN `station_station_types` AS sst ON sst.line_group_cd = ?
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
        station_id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        let rows: Vec<TrainTypeRow> = sqlx::query_as!(TrainTypeRow,
            "SELECT 
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            sst.*
            FROM  `types` AS t
            JOIN `stations` AS s ON s.station_cd = ? AND s.e_status = 0
            JOIN `station_station_types` AS sst ON sst.station_cd = s.station_cd AND sst.type_cd = t.type_cd AND sst.pass <> 1
            ORDER BY sst.id",
            station_id
        )
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
            FROM `types` as t
            JOIN `station_station_types` AS sst ON sst.line_group_cd = ? 
            WHERE 
            sst.station_cd IN (
                SELECT 
                station_cd 
                FROM 
                stations as s 
                WHERE 
                line_cd = ?
                AND s.e_status = 0
            )
            AND t.type_cd = sst.type_cd
            ORDER BY sst.id",
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

    async fn get_by_station_id_vec(
        station_id_vec: Vec<u32>,
        line_group_id: Option<u32>,
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
            types as t
            JOIN `stations` AS s ON s.station_cd IN ( {} ) AND s.e_status = 0
            JOIN `station_station_types` AS sst ON sst.line_group_cd = ? AND sst.pass <> 1 AND sst.type_cd = t.type_cd
            WHERE sst.pass <> 1 AND sst.type_cd = t.type_cd
            ORDER BY sst.id",
            params
        );

        let mut query = sqlx::query_as::<_, TrainTypeRow>(&query_str);
        for id in station_id_vec {
            query = query.bind(id);
        }

        let rows = query.bind(line_group_id).fetch_all(conn).await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }

    async fn get_types_by_station_id_vec(
        station_id_vec: Vec<u32>,
        line_group_id: Option<u32>,
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
            AND sst.line_group_cd = ?
            AND sst.pass <> 1
            ORDER BY t.kind, sst.id",
            params
        );

        let mut query = sqlx::query_as::<_, TrainTypeRow>(&query_str);
        for id in station_id_vec {
            query = query.bind(id);
        }

        let rows = query.bind(line_group_id).fetch_all(conn).await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }

    async fn get_by_line_group_id_vec(
        line_group_id_vec: Vec<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        if line_group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(line_group_id_vec.len() - 1));
        let query_str = format!(
            "SELECT 
            t.*, 
            s.*,
            sst.*
            FROM 
            types as t
            JOIN `station_station_types` AS sst ON sst.line_group_cd IN ( {} ) AND sst.pass <> 1 AND sst.type_cd = t.type_cd
            JOIN `stations` AS s ON s.station_cd = sst.station_cd
            WHERE sst.pass <> 1 AND sst.type_cd = t.type_cd
            ORDER BY sst.id",
            params
        );

        let mut query = sqlx::query_as::<_, TrainTypeRow>(&query_str);
        for id in line_group_id_vec {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
}
