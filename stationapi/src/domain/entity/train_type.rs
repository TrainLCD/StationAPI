use serde::{Deserialize, Serialize};

use super::line::Line;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrainType {
    pub id: u32,
    pub station_cd: u32,
    pub type_cd: Option<u32>,
    pub line_group_cd: Option<u32>,
    pub pass: Option<u32>,
    pub type_name: Option<String>,
    pub type_name_k: Option<String>,
    pub type_name_r: Option<String>,
    pub type_name_zh: Option<String>,
    pub type_name_ko: Option<String>,
    pub color: Option<String>,
    pub direction: Option<u32>,
    pub line: Option<Box<Line>>,
    pub lines: Vec<Line>,
    pub kind: u32,
}

impl TrainType {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
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
            line: None,
            lines: vec![],
            kind,
        }
    }
}
