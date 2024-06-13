use serde::{Deserialize, Serialize};

use super::line::Line;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TrainType {
    pub id: i32,
    pub station_cd: i32,
    pub type_cd: i32,
    pub line_cd: i32,
    pub line_group_cd: i32,
    pub pass: i32,
    pub type_name: String,
    pub type_name_k: String,
    pub type_name_r: String,
    pub type_name_zh: String,
    pub type_name_ko: String,
    pub color: String,
    pub direction: i32,
    pub line: Option<Box<Line>>,
    pub lines: Vec<Line>,
    pub kind: i32,
}

impl TrainType {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
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
        direction: i32,
        kind: i32,
    ) -> Self {
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
            direction,
            kind,
            line: None,
            lines: vec![],
        }
    }
}
