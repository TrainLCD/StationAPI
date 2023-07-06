use crate::domain::{
    entity::train_type::{TrainDirection, TrainType},
    repository::train_type_repository::TrainTypeRepository,
};
use async_trait::async_trait;
use sqlx::{MySql, Pool};

#[derive(sqlx::FromRow, Clone)]
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
        let direction: TrainDirection = match row.direction {
            0 => TrainDirection::Both,
            1 => TrainDirection::Inbound,
            2 => TrainDirection::Outbound,
            _ => TrainDirection::Both,
        };

        Self {
            id: row.id,
            type_cd: row.type_cd,
            group_cd: row.line_group_cd,
            name: row.type_name,
            name_k: row.type_name_k,
            name_r: row.type_name_r,
            name_zh: row.type_name_zh,
            name_ko: row.type_name_ko,
            color: row.color,
            stations: vec![],
            lines: vec![],
            all_train_types: vec![],
            direction,
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
impl TrainTypeRepository for MyTrainTypeRepository {}

pub struct InternalTrainTypeRepository {}

impl InternalTrainTypeRepository {}
