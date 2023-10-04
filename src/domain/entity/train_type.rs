use super::line::Line;

#[derive(Clone, Debug)]
pub struct TrainType {
    pub sst_cd: u32,
    pub station_cd: u32,
    pub type_cd: u32,
    pub line_group_cd: u32,
    pub pass: u32,
    pub type_name: String,
    pub type_name_k: String,
    pub type_name_r: Option<String>,
    pub type_name_zh: Option<String>,
    pub type_name_ko: Option<String>,
    pub color: String,
    pub direction: u32,
    pub line: Option<Box<Line>>,
    pub lines: Vec<Line>,
}

impl TrainType {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        sst_cd: u32,
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
    ) -> Self {
        Self {
            sst_cd,
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
