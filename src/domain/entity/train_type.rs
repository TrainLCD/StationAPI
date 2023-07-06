use fake::Dummy;

use super::{line::Line, station::Station};

#[derive(Dummy, Clone, Debug)]
pub enum TrainDirection {
    Both,
    Inbound,
    Outbound,
}

#[derive(Dummy, Clone, Debug)]
pub struct TrainType {
    pub id: u32,
    pub type_cd: u32,
    pub group_cd: u32,
    pub name: String,
    pub name_k: String,
    pub name_r: String,
    pub name_zh: String,
    pub name_ko: String,
    pub color: String,
    pub stations: Vec<Station>,
    pub lines: Vec<Line>,
    pub all_train_types: Vec<TrainType>,
    pub direction: TrainDirection,
}
