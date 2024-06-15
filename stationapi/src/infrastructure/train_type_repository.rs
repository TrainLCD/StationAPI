use crate::domain::{
    entity::train_type::TrainType, error::DomainError,
    repository::train_type_repository::TrainTypeRepository,
};
use async_trait::async_trait;
use sqlx::{query_as, PgConnection, Pool, Postgres};

#[derive(sqlx::FromRow)]
pub struct TrainTypeRow {
    id: i32,
    station_cd: i32,
    type_cd: i32,
    line_cd: i32,
    line_group_cd: i32,
    pass: i32,
    type_name: String,
    type_name_k: String,
    type_name_r: String,
    type_name_zh: String,
    type_name_ko: String,
    color: String,
    direction: Option<i32>,
    kind: Option<i32>,
}

impl From<TrainTypeRow> for TrainType {
    fn from(row: TrainTypeRow) -> Self {
        let TrainTypeRow {
            id,
            station_cd,
            type_cd,
            line_cd,
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
            line_cd,
            line_group_cd,
            pass,
            type_name,
            type_name_k,
            type_name_r,
            type_name_zh,
            type_name_ko,
            color,
            direction: direction.unwrap_or(0),
            kind: kind.unwrap_or(0),
            line: None,
            lines: vec![],
        }
    }
}

#[derive(Debug)]
pub struct MyTrainTypeRepository {
    pool: Pool<Postgres>,
}

impl MyTrainTypeRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TrainTypeRepository for MyTrainTypeRepository {
    async fn get_by_line_group_id_vec(
        &self,
        line_group_ids: Vec<i32>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id_vec(line_group_ids, &mut conn).await
    }

    async fn get_by_station_id(&self, station_id: i32) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_station_id(station_id, &mut conn).await
    }

    async fn find_by_line_group_id_and_line_id(
        &self,
        line_group_id: i32,
        line_id: i32,
    ) -> Result<Option<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id_and_line_id(
            line_group_id.try_into().unwrap(),
            line_id.try_into().unwrap(),
            &mut conn,
        )
        .await
    }

    async fn get_types_by_station_id_vec(
        &self,
        station_id_vec: Vec<i32>,
        line_group_id: Option<i32>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_types_by_station_id_vec(
            station_id_vec,
            line_group_id,
            &mut conn,
        )
        .await
    }
}

pub struct InternalTrainTypeRepository {}

impl InternalTrainTypeRepository {
    async fn get_by_line_group_id_vec(
        line_group_ids: Vec<i32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        let rows = sqlx::query_as!(
            TrainTypeRow,
            r#"SELECT
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.kind AS "kind?",
            t.direction AS "direction?",
            sst.*,
            s.line_cd
            FROM types as t
            JOIN station_station_types AS sst ON sst.line_group_cd IN (SELECT(UNNEST($1::integer[])))
            JOIN stations AS s ON s.station_cd = sst.station_cd
            WHERE 
                t.type_cd = sst.type_cd"#,
            &line_group_ids
        )
        .fetch_all(conn)
        .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
    async fn get_by_station_id(
        station_id: i32,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        let rows = sqlx::query_as!(TrainTypeRow,
            r#"SELECT
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.kind AS "kind?",
            t.direction AS "direction?",
            sst.*,
            s.line_cd
            FROM types AS t
            JOIN stations AS s ON s.station_cd = $1 AND s.e_status = 0
            JOIN station_station_types AS sst ON sst.station_cd = s.station_cd AND sst.type_cd = t.type_cd AND sst.pass <> 1
            ORDER BY t.kind, sst.id"#,
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
        conn: &mut PgConnection,
    ) -> Result<Option<TrainType>, DomainError> {
        let rows = sqlx::query_as!(
            TrainTypeRow,
            r#"SELECT
            sst.type_cd,
            s.line_cd,
            sst.station_cd,
            sst.line_group_cd,
            sst.pass,
            t.id,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.kind AS "kind?",
            t.direction AS "direction?"
            FROM types as t
            JOIN station_station_types AS sst ON sst.line_group_cd = $1
            JOIN stations AS s ON s.station_cd = sst.station_cd
            WHERE 
            sst.station_cd IN (
                SELECT 
                station_cd 
                FROM 
                stations as s 
                WHERE 
                line_cd = $2
                AND s.e_status = 0
            )
            AND t.type_cd = sst.type_cd
            ORDER BY t.kind, sst.id"#,
            line_group_id as i32,
            line_id as i32,
        )
        .fetch_optional(conn)
        .await?;

        let train_type: Option<TrainType> = rows.map(|row| row.into());

        let Some(train_type) = train_type else {
            return Ok(None);
        };

        Ok(Some(train_type))
    }

    async fn get_types_by_station_id_vec(
        station_id_vec: Vec<i32>,
        line_group_id: Option<i32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        if station_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let rows = query_as!(TrainTypeRow,
            r#"SELECT 
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.kind AS "kind?",
            t.direction AS "direction?",
            sst.*,
            s.line_cd
            FROM 
            types AS t
            JOIN stations AS s ON s.station_cd IN (SELECT UNNEST($1::integer[])) AND s.e_status = 0
            LEFT JOIN station_station_types AS sst ON sst.line_group_cd = $2 AND sst.pass <> 1 AND sst.type_cd = t.type_cd
            WHERE 
            CASE WHEN t.top_priority = 1
            THEN
                sst.type_cd = t.type_cd
            ELSE
                sst.pass <> 1
                AND sst.type_cd = t.type_cd
            END
            ORDER BY t.kind, sst.id"#,
            &station_id_vec,
            line_group_id
        ).fetch_all(conn).await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
}
