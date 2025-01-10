use serde::{Deserialize, Serialize};

use super::line::Line;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrainType {
    pub id: u32,
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
    pub kind: u32,
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
        type_name_r: Option<String>,
        type_name_zh: Option<String>,
        type_name_ko: Option<String>,
        color: String,
        direction: u32,
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

#[cfg(test)]
mod tests {
    use super::TrainType;

    #[test]
    fn new() {
        let train_type = TrainType::new(
            1,
            100201,
            1,
            1,
            0,
            "のぞみ".to_string(),
            "ノゾミ".to_string(),
            Some("Nozomi".to_string()),
            Some("希望".to_string()),
            Some("노조미".to_string()),
            "#FFD400".to_string(),
            0,
            4,
        );

        assert_eq!(train_type.id, 1);
        assert_eq!(train_type.station_cd, 100201);
        assert_eq!(train_type.type_cd, 1);
        assert_eq!(train_type.line_group_cd, 1);
        assert_eq!(train_type.pass, 0);
        assert_eq!(train_type.type_name, "のぞみ");
        assert_eq!(train_type.type_name_k, "ノゾミ");
        assert_eq!(train_type.type_name_r, Some("Nozomi".to_string()));
        assert_eq!(train_type.type_name_zh, Some("希望".to_string()));
        assert_eq!(train_type.type_name_ko, Some("노조미".to_string()));
        assert_eq!(train_type.color, "#FFD400");
        assert_eq!(train_type.direction, 0);
        assert_eq!(train_type.kind, 4);
        assert_eq!(train_type.line, None);
        assert!(train_type.lines.is_empty());
    }
}
