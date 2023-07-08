use fake::Dummy;

use super::line::Line;

#[derive(Dummy, Clone, Debug)]
pub struct TrainType {
    pub id: u32,
    pub station_cd: u32,
    pub type_cd: u32,
    pub line_group_cd: u32,
    pub pass: u32,
    pub type_name: String,
    pub type_name_k: String,
    pub type_name_r: String,
    pub type_name_zh: String,
    pub type_name_ko: String,
    pub color: String,
    pub direction: u32,
    pub lines: Vec<Line>,
}

impl TrainType {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
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
    ) -> Self {
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
