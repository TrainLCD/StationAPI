use serde::{Deserialize, Serialize};

use super::line::Line;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrainType {
    pub id: Option<i32>,
    pub station_cd: Option<i32>,
    pub type_cd: Option<i32>,
    pub line_group_cd: Option<i32>,
    pub pass: Option<i32>,
    pub type_name: String,
    pub type_name_k: String,
    pub type_name_r: Option<String>,
    pub type_name_zh: Option<String>,
    pub type_name_ko: Option<String>,
    pub color: String,
    pub direction: Option<i32>,
    pub line: Option<Box<Line>>,
    pub lines: Vec<Line>,
    pub kind: Option<i32>,
}

impl TrainType {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        id: Option<i32>,
        station_cd: Option<i32>,
        type_cd: Option<i32>,
        line_group_cd: Option<i32>,
        pass: Option<i32>,
        type_name: String,
        type_name_k: String,
        type_name_r: Option<String>,
        type_name_zh: Option<String>,
        type_name_ko: Option<String>,
        color: String,
        direction: Option<i32>,
        kind: Option<i32>,
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
    use super::*;

    fn create_test_line() -> Line {
        Line::new(
            11302,                             // line_cd
            1001,                              // company_cd
            None,                              // company
            "山手線".to_string(),              // line_name
            "ヤマノテセン".to_string(),        // line_name_k
            "やまのてせん".to_string(),        // line_name_h
            Some("Yamanote Line".to_string()), // line_name_r
            Some("山手线".to_string()),        // line_name_zh
            Some("야마노테선".to_string()),    // line_name_ko
            Some("#00B261".to_string()),       // line_color_c
            Some(0),                           // line_type
            vec![],                            // line_symbols
            Some("JY".to_string()),            // line_symbol1
            None,                              // line_symbol2
            None,                              // line_symbol3
            None,                              // line_symbol4
            Some("#00B261".to_string()),       // line_symbol1_color
            None,                              // line_symbol2_color
            None,                              // line_symbol3_color
            None,                              // line_symbol4_color
            Some("square".to_string()),        // line_symbol1_shape
            None,                              // line_symbol2_shape
            None,                              // line_symbol3_shape
            None,                              // line_symbol4_shape
            1,                                 // e_status
            1130201,                           // e_sort
            None,                              // station
            None,                              // train_type
            Some(1001),                        // line_group_cd
            Some(11302),                       // station_cd
            Some(1130201),                     // station_g_cd
            0.97,                              // average_distance
            None,                              // type_cd
        )
    }

    fn create_test_train_type() -> TrainType {
        TrainType::new(
            Some(1),                   // id
            Some(1130201),             // station_cd
            Some(1001),                // type_cd
            Some(1001),                // line_group_cd
            Some(1),                   // pass
            "快速".to_string(),        // type_name
            "カイソク".to_string(),    // type_name_k
            Some("Rapid".to_string()), // type_name_r
            Some("快速".to_string()),  // type_name_zh
            Some("쾌속".to_string()),  // type_name_ko
            "#ff6600".to_string(),     // color
            Some(0),                   // direction
            Some(1),                   // kind
        )
    }

    fn create_test_train_type_minimal() -> TrainType {
        TrainType::new(
            None,                  // id
            None,                  // station_cd
            None,                  // type_cd
            None,                  // line_group_cd
            None,                  // pass
            "普通".to_string(),    // type_name
            "フツウ".to_string(),  // type_name_k
            None,                  // type_name_r
            None,                  // type_name_zh
            None,                  // type_name_ko
            "#000000".to_string(), // color
            None,                  // direction
            None,                  // kind
        )
    }

    #[test]
    fn test_train_type_new() {
        let train_type = create_test_train_type();

        assert_eq!(train_type.id, Some(1));
        assert_eq!(train_type.station_cd, Some(1130201));
        assert_eq!(train_type.type_cd, Some(1001));
        assert_eq!(train_type.line_group_cd, Some(1001));
        assert_eq!(train_type.pass, Some(1));
        assert_eq!(train_type.type_name, "快速");
        assert_eq!(train_type.type_name_k, "カイソク");
        assert_eq!(train_type.type_name_r, Some("Rapid".to_string()));
        assert_eq!(train_type.type_name_zh, Some("快速".to_string()));
        assert_eq!(train_type.type_name_ko, Some("쾌속".to_string()));
        assert_eq!(train_type.color, "#ff6600");
        assert_eq!(train_type.direction, Some(0));
        assert_eq!(train_type.line, None);
        assert!(train_type.lines.is_empty());
        assert_eq!(train_type.kind, Some(1));
    }

    #[test]
    fn test_train_type_new_minimal() {
        let train_type = create_test_train_type_minimal();

        assert_eq!(train_type.id, None);
        assert_eq!(train_type.station_cd, None);
        assert_eq!(train_type.type_cd, None);
        assert_eq!(train_type.line_group_cd, None);
        assert_eq!(train_type.pass, None);
        assert_eq!(train_type.type_name, "普通");
        assert_eq!(train_type.type_name_k, "フツウ");
        assert_eq!(train_type.type_name_r, None);
        assert_eq!(train_type.type_name_zh, None);
        assert_eq!(train_type.type_name_ko, None);
        assert_eq!(train_type.color, "#000000");
        assert_eq!(train_type.direction, None);
        assert_eq!(train_type.line, None);
        assert!(train_type.lines.is_empty());
        assert_eq!(train_type.kind, None);
    }

    #[test]
    fn test_train_type_with_line() {
        let mut train_type = create_test_train_type();
        let line = create_test_line();

        train_type.line = Some(Box::new(line.clone()));
        train_type.lines = vec![line.clone()];

        assert!(train_type.line.is_some());
        assert_eq!(train_type.lines.len(), 1);
        assert_eq!(train_type.lines[0], line);
    }

    #[test]
    fn test_train_type_clone() {
        let train_type = create_test_train_type();
        let cloned = train_type.clone();

        assert_eq!(train_type, cloned);
    }

    #[test]
    fn test_train_type_partial_eq() {
        let train_type1 = create_test_train_type();
        let train_type2 = create_test_train_type();
        let train_type3 = create_test_train_type_minimal();

        assert_eq!(train_type1, train_type2);
        assert_ne!(train_type1, train_type3);
    }

    #[test]
    fn test_train_type_serialization() {
        let train_type = create_test_train_type();

        let serialized = serde_json::to_string(&train_type).unwrap();
        let deserialized: TrainType = serde_json::from_str(&serialized).unwrap();

        assert_eq!(train_type, deserialized);
    }

    #[test]
    fn test_train_type_serialization_minimal() {
        let train_type = create_test_train_type_minimal();

        let serialized = serde_json::to_string(&train_type).unwrap();
        let deserialized: TrainType = serde_json::from_str(&serialized).unwrap();

        assert_eq!(train_type, deserialized);
    }

    #[test]
    fn test_train_type_debug() {
        let train_type = create_test_train_type();
        let debug_string = format!("{train_type:?}");

        assert!(debug_string.contains("TrainType"));
        assert!(debug_string.contains("快速"));
    }

    #[test]
    fn test_train_type_with_multiple_lines() {
        let mut train_type = create_test_train_type();
        let line1 = create_test_line();
        let mut line2 = create_test_line();
        line2.line_cd = 11303;
        line2.line_name = "中央線".to_string();

        train_type.lines = vec![line1.clone(), line2.clone()];

        assert_eq!(train_type.lines.len(), 2);
        assert_eq!(train_type.lines[0], line1);
        assert_eq!(train_type.lines[1], line2);
    }

    #[test]
    fn test_train_type_field_types() {
        let train_type = create_test_train_type();

        // Option<i32>フィールドのテスト
        assert!(train_type.id.is_some());
        assert!(train_type.station_cd.is_some());
        assert!(train_type.type_cd.is_some());

        // Stringフィールドのテスト
        assert!(!train_type.type_name.is_empty());
        assert!(!train_type.type_name_k.is_empty());
        assert!(!train_type.color.is_empty());

        // Option<String>フィールドのテスト
        assert!(train_type.type_name_r.is_some());
        assert!(train_type.type_name_zh.is_some());
        assert!(train_type.type_name_ko.is_some());
    }

    #[test]
    fn test_train_type_with_unicode_characters() {
        let train_type = TrainType::new(
            Some(999),
            Some(1130201),
            Some(1001),
            Some(1001),
            Some(1),
            "特急「あずさ」".to_string(),
            "トッキュウアズサ".to_string(),
            Some("Limited Express \"Azusa\"".to_string()),
            Some("特急「梓」".to_string()),
            Some("특급 「아즈사」".to_string()),
            "#E233AA".to_string(),
            Some(0),
            Some(3),
        );

        assert_eq!(train_type.type_name, "特急「あずさ」");
        assert_eq!(train_type.type_name_k, "トッキュウアズサ");
        assert_eq!(
            train_type.type_name_r,
            Some("Limited Express \"Azusa\"".to_string())
        );
        assert_eq!(train_type.type_name_zh, Some("特急「梓」".to_string()));
        assert_eq!(train_type.type_name_ko, Some("특급 「아즈사」".to_string()));
    }

    #[test]
    fn test_train_type_with_empty_strings() {
        let train_type = TrainType::new(
            None,
            None,
            None,
            None,
            None,
            "".to_string(),
            "".to_string(),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            "".to_string(),
            None,
            None,
        );

        assert_eq!(train_type.type_name, "");
        assert_eq!(train_type.type_name_k, "");
        assert_eq!(train_type.color, "");
        assert_eq!(train_type.type_name_r, Some("".to_string()));
    }

    #[test]
    fn test_train_type_edge_cases() {
        // 極値のテスト
        let train_type = TrainType::new(
            Some(i32::MAX),
            Some(i32::MIN),
            Some(0),
            Some(-1),
            Some(999999),
            "Very Long Train Type Name That Exceeds Normal Length".to_string(),
            "ベリーロングトレインタイプネーム".to_string(),
            None,
            None,
            None,
            "#FFFFFF".to_string(),
            Some(i32::MAX),
            Some(i32::MIN),
        );

        assert_eq!(train_type.id, Some(i32::MAX));
        assert_eq!(train_type.station_cd, Some(i32::MIN));
        assert_eq!(train_type.direction, Some(i32::MAX));
        assert_eq!(train_type.kind, Some(i32::MIN));
    }
}
