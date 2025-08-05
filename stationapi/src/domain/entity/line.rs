use serde::{Deserialize, Serialize};

use super::{company::Company, line_symbol::LineSymbol, station::Station, train_type::TrainType};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Line {
    pub line_cd: i32,
    pub company_cd: i32,
    pub company: Option<Company>,
    pub line_name: String,
    pub line_name_k: String,
    pub line_name_h: String,
    pub line_name_r: Option<String>,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: Option<String>,
    pub line_type: Option<i32>,
    pub line_symbols: Vec<LineSymbol>,
    pub line_symbol1: Option<String>,
    pub line_symbol2: Option<String>,
    pub line_symbol3: Option<String>,
    pub line_symbol4: Option<String>,
    pub line_symbol1_color: Option<String>,
    pub line_symbol2_color: Option<String>,
    pub line_symbol3_color: Option<String>,
    pub line_symbol4_color: Option<String>,
    pub line_symbol1_shape: Option<String>,
    pub line_symbol2_shape: Option<String>,
    pub line_symbol3_shape: Option<String>,
    pub line_symbol4_shape: Option<String>,
    pub e_status: i32,
    pub e_sort: i32,
    pub average_distance: f64,
    pub station: Option<Station>,
    pub train_type: Option<TrainType>,
    pub line_group_cd: Option<i32>,
    pub station_cd: Option<i32>,
    pub station_g_cd: Option<i32>,
    pub type_cd: Option<i32>,
}

impl Line {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        line_cd: i32,
        company_cd: i32,
        company: Option<Company>,
        line_name: String,
        line_name_k: String,
        line_name_h: String,
        line_name_r: Option<String>,
        line_name_zh: Option<String>,
        line_name_ko: Option<String>,
        line_color_c: Option<String>,
        line_type: Option<i32>,
        line_symbols: Vec<LineSymbol>,
        line_symbol1: Option<String>,
        line_symbol2: Option<String>,
        line_symbol3: Option<String>,
        line_symbol4: Option<String>,
        line_symbol1_color: Option<String>,
        line_symbol2_color: Option<String>,
        line_symbol3_color: Option<String>,
        line_symbol4_color: Option<String>,
        line_symbol1_shape: Option<String>,
        line_symbol2_shape: Option<String>,
        line_symbol3_shape: Option<String>,
        line_symbol4_shape: Option<String>,
        e_status: i32,
        e_sort: i32,
        station: Option<Station>,
        train_type: Option<TrainType>,
        line_group_cd: Option<i32>,
        station_cd: Option<i32>,
        station_g_cd: Option<i32>,
        average_distance: f64,
        type_cd: Option<i32>,
    ) -> Self {
        Self {
            line_cd,
            company_cd,
            company,
            line_name,
            line_name_k,
            line_name_h,
            line_name_r,
            line_name_zh,
            line_name_ko,
            line_color_c,
            line_type,
            line_symbols,
            line_symbol1,
            line_symbol2,
            line_symbol3,
            line_symbol4,
            line_symbol1_color,
            line_symbol2_color,
            line_symbol3_color,
            line_symbol4_color,
            line_symbol1_shape,
            line_symbol2_shape,
            line_symbol3_shape,
            line_symbol4_shape,
            e_status,
            e_sort,
            station,
            train_type,
            line_group_cd,
            station_cd,
            station_g_cd,
            average_distance,
            type_cd,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_company() -> Company {
        Company::new(
            1001,
            2001,
            "東日本旅客鉄道".to_string(),
            "ヒガシニホンリョカクテツドウ".to_string(),
            "ひがしにほんりょかくてつどう".to_string(),
            "Higashi-Nihon Ryokaku Tetsudou".to_string(),
            "JR East".to_string(),
            "East Japan Railway Company".to_string(),
            Some("https://www.jreast.co.jp".to_string()),
            1,
            1,
            1000,
        )
    }

    fn create_test_line_symbol() -> LineSymbol {
        LineSymbol::new(
            "JY".to_string(),
            "#00B261".to_string(),
            "square".to_string(),
        )
    }

    fn create_test_line() -> Line {
        Line::new(
            11302,                             // line_cd
            1001,                              // company_cd
            Some(create_test_company()),       // company
            "山手線".to_string(),              // line_name
            "ヤマノテセン".to_string(),        // line_name_k
            "やまのてせん".to_string(),        // line_name_h
            Some("Yamanote Line".to_string()), // line_name_r
            Some("山手线".to_string()),        // line_name_zh
            Some("야마노테선".to_string()),    // line_name_ko
            Some("#00B261".to_string()),       // line_color_c
            Some(0),                           // line_type
            vec![create_test_line_symbol()],   // line_symbols
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
            Some(1),                           // type_cd
        )
    }

    fn create_test_line_minimal() -> Line {
        Line::new(
            11302,                      // line_cd
            1001,                       // company_cd
            None,                       // company
            "山手線".to_string(),       // line_name
            "ヤマノテセン".to_string(), // line_name_k
            "やまのてせん".to_string(), // line_name_h
            None,                       // line_name_r
            None,                       // line_name_zh
            None,                       // line_name_ko
            None,                       // line_color_c
            None,                       // line_type
            vec![],                     // line_symbols
            None,                       // line_symbol1
            None,                       // line_symbol2
            None,                       // line_symbol3
            None,                       // line_symbol4
            None,                       // line_symbol1_color
            None,                       // line_symbol2_color
            None,                       // line_symbol3_color
            None,                       // line_symbol4_color
            None,                       // line_symbol1_shape
            None,                       // line_symbol2_shape
            None,                       // line_symbol3_shape
            None,                       // line_symbol4_shape
            1,                          // e_status
            1130201,                    // e_sort
            None,                       // station
            None,                       // train_type
            None,                       // line_group_cd
            None,                       // station_cd
            None,                       // station_g_cd
            0.0,                        // average_distance
            None,                       // type_cd
        )
    }

    #[test]
    fn test_line_new() {
        let line = create_test_line();

        assert_eq!(line.line_cd, 11302);
        assert_eq!(line.company_cd, 1001);
        assert!(line.company.is_some());
        assert_eq!(line.line_name, "山手線");
        assert_eq!(line.line_name_k, "ヤマノテセン");
        assert_eq!(line.line_name_h, "やまのてせん");
        assert_eq!(line.line_name_r, Some("Yamanote Line".to_string()));
        assert_eq!(line.line_name_zh, Some("山手线".to_string()));
        assert_eq!(line.line_name_ko, Some("야마노테선".to_string()));
        assert_eq!(line.line_color_c, Some("#00B261".to_string()));
        assert_eq!(line.line_type, Some(0));
        assert_eq!(line.line_symbols.len(), 1);
        assert_eq!(line.line_symbol1, Some("JY".to_string()));
        assert_eq!(line.line_symbol1_color, Some("#00B261".to_string()));
        assert_eq!(line.line_symbol1_shape, Some("square".to_string()));
        assert_eq!(line.e_status, 1);
        assert_eq!(line.e_sort, 1130201);
        assert_eq!(line.average_distance, 0.97);
        assert_eq!(line.line_group_cd, Some(1001));
        assert_eq!(line.station_cd, Some(11302));
        assert_eq!(line.station_g_cd, Some(1130201));
        assert_eq!(line.type_cd, Some(1));
    }

    #[test]
    fn test_line_new_minimal() {
        let line = create_test_line_minimal();

        assert_eq!(line.line_cd, 11302);
        assert_eq!(line.company_cd, 1001);
        assert!(line.company.is_none());
        assert_eq!(line.line_name, "山手線");
        assert_eq!(line.line_name_k, "ヤマノテセン");
        assert_eq!(line.line_name_h, "やまのてせん");
        assert_eq!(line.line_name_r, None);
        assert_eq!(line.line_name_zh, None);
        assert_eq!(line.line_name_ko, None);
        assert_eq!(line.line_color_c, None);
        assert_eq!(line.line_type, None);
        assert!(line.line_symbols.is_empty());
        assert_eq!(line.line_symbol1, None);
        assert_eq!(line.line_symbol2, None);
        assert_eq!(line.line_symbol3, None);
        assert_eq!(line.line_symbol4, None);
        assert_eq!(line.line_symbol1_color, None);
        assert_eq!(line.line_symbol2_color, None);
        assert_eq!(line.line_symbol3_color, None);
        assert_eq!(line.line_symbol4_color, None);
        assert_eq!(line.line_symbol1_shape, None);
        assert_eq!(line.line_symbol2_shape, None);
        assert_eq!(line.line_symbol3_shape, None);
        assert_eq!(line.line_symbol4_shape, None);
        assert_eq!(line.average_distance, 0.0);
        assert_eq!(line.line_group_cd, None);
        assert_eq!(line.station_cd, None);
        assert_eq!(line.station_g_cd, None);
        assert_eq!(line.type_cd, None);
    }

    #[test]
    fn test_line_clone() {
        let original = create_test_line();
        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(original.line_cd, cloned.line_cd);
        assert_eq!(original.line_name, cloned.line_name);
        assert_eq!(original.company_cd, cloned.company_cd);
        assert_eq!(original.average_distance, cloned.average_distance);
        assert_eq!(original.type_cd, cloned.type_cd);
    }

    #[test]
    fn test_line_partial_eq() {
        let line1 = create_test_line();
        let line2 = create_test_line();
        let line3 = create_test_line_minimal();

        // 同じ値を持つ構造体は等しい
        assert_eq!(line1, line2);

        // 異なる値を持つ構造体は等しくない
        assert_ne!(line1, line3);
    }

    #[test]
    fn test_line_serialization() {
        let line = create_test_line();

        // JSONにシリアライズ
        let json = serde_json::to_string(&line).expect("シリアライゼーションに失敗しました");
        assert!(json.contains("\"line_cd\":11302"));
        assert!(json.contains("\"line_name\":\"山手線\""));
        assert!(json.contains("\"line_color_c\":\"#00B261\""));
        assert!(json.contains("\"type_cd\":1"));

        // JSONからデシリアライズ
        let deserialized: Line =
            serde_json::from_str(&json).expect("デシリアライゼーションに失敗しました");
        assert_eq!(line, deserialized);
    }

    #[test]
    fn test_line_serialization_minimal() {
        let line = create_test_line_minimal();

        // JSONにシリアライズ（Optional fieldsがNone）
        let json = serde_json::to_string(&line).expect("シリアライゼーションに失敗しました");
        assert!(json.contains("\"company\":null"));
        assert!(json.contains("\"line_name_r\":null"));
        assert!(json.contains("\"line_color_c\":null"));
        assert!(json.contains("\"type_cd\":null"));

        // JSONからデシリアライズ
        let deserialized: Line =
            serde_json::from_str(&json).expect("デシリアライゼーションに失敗しました");
        assert_eq!(line, deserialized);
    }

    #[test]
    fn test_line_debug() {
        let line = create_test_line();
        let debug_string = format!("{line:?}");

        assert!(debug_string.contains("Line"));
        assert!(debug_string.contains("line_cd: 11302"));
        assert!(debug_string.contains("山手線"));
        assert!(debug_string.contains("#00B261"));
        assert!(debug_string.contains("type_cd: Some(1)"));
    }

    #[test]
    fn test_line_with_multiple_symbols() {
        let symbols = vec![
            LineSymbol::new(
                "JY".to_string(),
                "#00B261".to_string(),
                "square".to_string(),
            ),
            LineSymbol::new(
                "JR".to_string(),
                "#FF0000".to_string(),
                "circle".to_string(),
            ),
        ];

        let line = Line::new(
            11302,
            1001,
            None,
            "山手線".to_string(),
            "ヤマノテセン".to_string(),
            "やまのてせん".to_string(),
            None,
            None,
            None,
            None,
            None,
            symbols.clone(),
            Some("JY".to_string()),
            Some("JR".to_string()),
            None,
            None,
            Some("#00B261".to_string()),
            Some("#FF0000".to_string()),
            None,
            None,
            Some("square".to_string()),
            Some("circle".to_string()),
            None,
            None,
            1,
            1130201,
            None,
            None,
            None,
            None,
            None,
            0.97,
            Some(2),
        );

        assert_eq!(line.line_symbols.len(), 2);
        assert_eq!(line.line_symbol1, Some("JY".to_string()));
        assert_eq!(line.line_symbol2, Some("JR".to_string()));
        assert_eq!(line.line_symbol1_color, Some("#00B261".to_string()));
        assert_eq!(line.line_symbol2_color, Some("#FF0000".to_string()));
        assert_eq!(line.line_symbol1_shape, Some("square".to_string()));
        assert_eq!(line.line_symbol2_shape, Some("circle".to_string()));
        assert_eq!(line.type_cd, Some(2));
    }

    #[test]
    fn test_line_field_types() {
        let line = create_test_line();

        // 各フィールドの型が期待されるものであることを確認
        let _: i32 = line.line_cd;
        let _: i32 = line.company_cd;
        let _: Option<Company> = line.company;
        let _: String = line.line_name;
        let _: String = line.line_name_k;
        let _: String = line.line_name_h;
        let _: Option<String> = line.line_name_r;
        let _: Option<String> = line.line_name_zh;
        let _: Option<String> = line.line_name_ko;
        let _: Option<String> = line.line_color_c;
        let _: Option<i32> = line.line_type;
        let _: Vec<LineSymbol> = line.line_symbols;
        let _: Option<String> = line.line_symbol1;
        let _: Option<String> = line.line_symbol2;
        let _: Option<String> = line.line_symbol3;
        let _: Option<String> = line.line_symbol4;
        let _: Option<String> = line.line_symbol1_color;
        let _: Option<String> = line.line_symbol2_color;
        let _: Option<String> = line.line_symbol3_color;
        let _: Option<String> = line.line_symbol4_color;
        let _: Option<String> = line.line_symbol1_shape;
        let _: Option<String> = line.line_symbol2_shape;
        let _: Option<String> = line.line_symbol3_shape;
        let _: Option<String> = line.line_symbol4_shape;
        let _: i32 = line.e_status;
        let _: i32 = line.e_sort;
        let _: f64 = line.average_distance;
        let _: Option<Station> = line.station;
        let _: Option<TrainType> = line.train_type;
        let _: Option<i32> = line.line_group_cd;
        let _: Option<i32> = line.station_cd;
        let _: Option<i32> = line.station_g_cd;
        let _: Option<i32> = line.type_cd;
    }

    #[test]
    fn test_line_with_unicode_characters() {
        let line = Line::new(
            11302,
            1001,
            None,
            "東海道新幹線".to_string(),
            "トウカイドウシンカンセン".to_string(),
            "とうかいどうしんかんせん".to_string(),
            Some("Tōkaidō Shinkansen".to_string()),
            Some("东海道新干线".to_string()),
            Some("도카이도 신칸센".to_string()),
            Some(("#FFD400").to_string()),
            Some(7),
            vec![],
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            1,
            1130201,
            None,
            None,
            None,
            None,
            None,
            515.4,
            Some(7),
        );

        assert_eq!(line.line_name, "東海道新幹線");
        assert_eq!(line.line_name_k, "トウカイドウシンカンセン");
        assert_eq!(line.line_name_h, "とうかいどうしんかんせん");
        assert_eq!(line.line_name_r, Some("Tōkaidō Shinkansen".to_string()));
        assert_eq!(line.line_name_zh, Some("东海道新干线".to_string()));
        assert_eq!(line.line_name_ko, Some("도카이도 신칸센".to_string()));
        assert_eq!(line.type_cd, Some(7));
    }

    #[test]
    fn test_line_with_empty_strings() {
        let line = Line::new(
            0,
            0,
            None,
            "".to_string(),
            "".to_string(),
            "".to_string(),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some(0),
            vec![],
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            0,
            0,
            None,
            None,
            Some(0),
            Some(0),
            Some(0),
            0.0,
            Some(0),
        );

        assert_eq!(line.line_name, "");
        assert_eq!(line.line_name_k, "");
        assert_eq!(line.line_name_h, "");
        assert_eq!(line.line_name_r, Some("".to_string()));
        assert_eq!(line.line_symbol1, Some("".to_string()));
        assert_eq!(line.line_color_c, Some("".to_string()));
        assert_eq!(line.type_cd, Some(0));
    }

    #[test]
    fn test_line_edge_cases() {
        // 負の値のテスト
        let line = Line::new(
            -1,
            -1,
            None,
            "テスト線".to_string(),
            "テストセン".to_string(),
            "てすとせん".to_string(),
            None,
            None,
            None,
            None,
            Some(-1),
            vec![],
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            -1,
            -1,
            None,
            None,
            Some(-1),
            Some(-1),
            Some(-1),
            -1.0,
            Some(-1),
        );

        assert_eq!(line.line_cd, -1);
        assert_eq!(line.company_cd, -1);
        assert_eq!(line.line_type, Some(-1));
        assert_eq!(line.e_status, -1);
        assert_eq!(line.e_sort, -1);
        assert_eq!(line.average_distance, -1.0);
        assert_eq!(line.line_group_cd, Some(-1));
        assert_eq!(line.station_cd, Some(-1));
        assert_eq!(line.station_g_cd, Some(-1));
        assert_eq!(line.type_cd, Some(-1));
    }

    #[test]
    fn test_line_type_cd_specific_values() {
        // type_cdフィールドの特定の値をテスト
        let test_cases = vec![
            (Some(0), "普通列車"),
            (Some(1), "快速列車"),
            (Some(2), "急行列車"),
            (Some(3), "特急列車"),
            (Some(7), "新幹線"),
            (None, "未指定"),
        ];

        for (type_cd, description) in test_cases {
            let line = Line::new(
                1,
                1,
                None,
                format!("テスト線 ({description})"),
                "テストセン".to_string(),
                "てすとせん".to_string(),
                None,
                None,
                None,
                None,
                None,
                vec![],
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                1,
                1,
                None,
                None,
                None,
                None,
                None,
                1.0,
                type_cd,
            );

            assert_eq!(line.type_cd, type_cd);
        }
    }

    #[test]
    fn test_line_with_all_optional_fields_some() {
        // すべてのOptionalフィールドがSomeの場合のテスト
        let line = Line::new(
            12345,
            5678,
            Some(create_test_company()),
            "全項目設定線".to_string(),
            "ゼンコウモクセッテイセン".to_string(),
            "ぜんこうもくせっていせん".to_string(),
            Some("All Fields Set Line".to_string()),
            Some("全项目设定线".to_string()),
            Some("전 항목 설정선".to_string()),
            Some("#FF5733".to_string()),
            Some(5),
            vec![create_test_line_symbol()],
            Some("AF".to_string()),
            Some("FS".to_string()),
            Some("SL".to_string()),
            Some("XX".to_string()),
            Some("#FF5733".to_string()),
            Some("#33FF57".to_string()),
            Some("#3357FF".to_string()),
            Some("#F333FF".to_string()),
            Some("circle".to_string()),
            Some("square".to_string()),
            Some("triangle".to_string()),
            Some("diamond".to_string()),
            1,
            999999,
            None,
            None,
            Some(9999),
            Some(8888),
            Some(7777),
            123.45,
            Some(99),
        );

        // すべてのOptionalフィールドがSomeであることを確認
        assert!(line.company.is_some());
        assert!(line.line_name_r.is_some());
        assert!(line.line_name_zh.is_some());
        assert!(line.line_name_ko.is_some());
        assert!(line.line_color_c.is_some());
        assert!(line.line_type.is_some());
        assert!(line.line_symbol1.is_some());
        assert!(line.line_symbol2.is_some());
        assert!(line.line_symbol3.is_some());
        assert!(line.line_symbol4.is_some());
        assert!(line.line_symbol1_color.is_some());
        assert!(line.line_symbol2_color.is_some());
        assert!(line.line_symbol3_color.is_some());
        assert!(line.line_symbol4_color.is_some());
        assert!(line.line_symbol1_shape.is_some());
        assert!(line.line_symbol2_shape.is_some());
        assert!(line.line_symbol3_shape.is_some());
        assert!(line.line_symbol4_shape.is_some());
        assert!(line.line_group_cd.is_some());
        assert!(line.station_cd.is_some());
        assert!(line.station_g_cd.is_some());
        assert!(line.type_cd.is_some());

        // 具体的な値の確認
        assert_eq!(line.type_cd, Some(99));
        assert_eq!(line.line_group_cd, Some(9999));
        assert_eq!(line.station_cd, Some(8888));
        assert_eq!(line.station_g_cd, Some(7777));
    }

    #[test]
    fn test_line_serialization_with_type_cd() {
        // type_cdが設定されている場合のシリアライゼーションテスト
        let line_with_type = Line::new(
            1,
            1,
            None,
            "テスト".to_string(),
            "テスト".to_string(),
            "てすと".to_string(),
            None,
            None,
            None,
            None,
            None,
            vec![],
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            1,
            1,
            None,
            None,
            None,
            None,
            None,
            1.0,
            Some(42),
        );

        let json = serde_json::to_string(&line_with_type).expect("シリアライゼーションに失敗");
        assert!(json.contains("\"type_cd\":42"));

        let deserialized: Line = serde_json::from_str(&json).expect("デシリアライゼーションに失敗");
        assert_eq!(deserialized.type_cd, Some(42));
    }
}
