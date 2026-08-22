use serde::{Deserialize, Serialize};

use crate::model::StopCondition;

use super::{
    gtfs::TransportType, line::Line, station_number::StationNumber,
    train_type::TrainType as TrainTypeEntity,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Station {
    pub station_cd: i32,
    pub station_g_cd: i32,
    pub station_name: String,
    pub station_name_k: String,
    pub station_name_r: Option<String>,
    pub station_name_zh: Option<String>,
    pub station_name_ko: Option<String>,
    pub station_numbers: Vec<StationNumber>,
    pub station_number1: Option<String>,
    pub station_number2: Option<String>,
    pub station_number3: Option<String>,
    pub station_number4: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: i32,
    pub line: Option<Box<Line>>,
    pub lines: Vec<Line>,
    pub pref_cd: i32,
    pub post: String,
    pub address: String,
    pub lon: f64,
    pub lat: f64,
    pub open_ymd: String,
    pub close_ymd: String,
    pub e_status: i32,
    pub e_sort: i32,
    pub stop_condition: StopCondition,
    pub distance: Option<f64>,
    pub has_train_types: bool,
    pub train_type: Option<Box<TrainTypeEntity>>,
    // linesからJOIN
    pub company_cd: Option<i32>,
    pub line_name: Option<String>,
    pub line_name_k: Option<String>,
    pub line_name_h: Option<String>,
    pub line_name_r: Option<String>,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: Option<String>,
    pub line_type: Option<i32>,
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
    pub average_distance: Option<f64>,
    // station_station_typesからJOIN
    pub type_id: Option<i32>,
    pub sst_id: Option<i32>,
    pub type_cd: Option<i32>,
    pub line_group_cd: Option<i32>,
    pub pass: Option<i32>,
    // typesからJOIN
    pub type_name: Option<String>,
    pub type_name_k: Option<String>,
    pub type_name_r: Option<String>,
    pub type_name_zh: Option<String>,
    pub type_name_ko: Option<String>,
    pub color: Option<String>,
    pub direction: Option<i32>,
    pub kind: Option<i32>,
    pub transport_type: TransportType,
}

impl Station {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        station_cd: i32,
        station_g_cd: i32,
        station_name: String,
        station_name_k: String,
        station_name_r: Option<String>,
        station_name_zh: Option<String>,
        station_name_ko: Option<String>,
        station_numbers: Vec<StationNumber>,
        station_number1: Option<String>,
        station_number2: Option<String>,
        station_number3: Option<String>,
        station_number4: Option<String>,
        three_letter_code: Option<String>,
        line_cd: i32,
        line: Option<Box<Line>>,
        lines: Vec<Line>,
        pref_cd: i32,
        post: String,
        address: String,
        lon: f64,
        lat: f64,
        open_ymd: String,
        close_ymd: String,
        e_status: i32,
        e_sort: i32,
        stop_condition: StopCondition,
        distance: Option<f64>,
        has_train_types: bool,
        train_type: Option<Box<TrainTypeEntity>>,
        company_cd: Option<i32>,
        line_name: Option<String>,
        line_name_k: Option<String>,
        line_name_h: Option<String>,
        line_name_r: Option<String>,
        line_name_zh: Option<String>,
        line_name_ko: Option<String>,
        line_color_c: Option<String>,
        line_type: Option<i32>,
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
        line_group_cd: Option<i32>,
        average_distance: Option<f64>,
        pass: Option<i32>,
        type_id: Option<i32>,
        sst_id: Option<i32>,
        type_cd: Option<i32>,
        type_name: Option<String>,
        type_name_k: Option<String>,
        type_name_r: Option<String>,
        type_name_zh: Option<String>,
        type_name_ko: Option<String>,
        color: Option<String>,
        direction: Option<i32>,
        kind: Option<i32>,
        transport_type: TransportType,
    ) -> Self {
        Self {
            station_cd,
            station_g_cd,
            station_name,
            station_name_k,
            station_name_r,
            station_name_zh,
            station_name_ko,
            station_numbers,
            station_number1,
            station_number2,
            station_number3,
            station_number4,
            three_letter_code,
            line_cd,
            line,
            lines,
            pref_cd,
            post,
            address,
            lon,
            lat,
            open_ymd,
            close_ymd,
            e_status,
            e_sort,
            stop_condition,
            distance,
            has_train_types,
            train_type,
            company_cd,
            line_name,
            line_name_k,
            line_name_h,
            line_name_r,
            line_name_zh,
            line_name_ko,
            line_color_c,
            line_type,
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
            line_group_cd,
            pass,
            average_distance,
            type_id,
            sst_id,
            type_cd,
            type_name,
            type_name_k,
            type_name_r,
            type_name_zh,
            type_name_ko,
            color,
            direction,
            kind,
            transport_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::StopCondition;

    fn create_test_station_number() -> StationNumber {
        StationNumber::new(
            "JY".to_string(),
            "#00B261".to_string(),
            "square".to_string(),
            "01".to_string(),
        )
    }

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
            Some(0.97),                        // average_distance
            None,                              // type_cd
            TransportType::Rail,               // transport_type
        )
    }

    fn create_test_train_type() -> TrainTypeEntity {
        TrainTypeEntity::new(
            Some(1),
            Some(100201),
            Some(1),
            Some(1),
            Some(0),
            "快速".to_string(),
            "カイソク".to_string(),
            Some("Rapid".to_string()),
            Some("快速".to_string()),
            Some("쾌속".to_string()),
            "#FF6600".to_string(),
            Some(0),
            Some(1),
        )
    }

    fn create_test_station_full() -> Station {
        Station::new(
            1130201,                                  // station_cd
            1130201,                                  // station_g_cd
            "新宿".to_string(),                       // station_name
            "シンジュク".to_string(),                 // station_name_k
            Some("Shinjuku".to_string()),             // station_name_r
            Some("新宿".to_string()),                 // station_name_zh
            Some("신주쿠".to_string()),               // station_name_ko
            vec![create_test_station_number()],       // station_numbers
            Some("JY01".to_string()),                 // station_number1
            Some("JC05".to_string()),                 // station_number2
            Some("JS19".to_string()),                 // station_number3
            Some("SS01".to_string()),                 // station_number4
            Some("SJK".to_string()),                  // three_letter_code
            11302,                                    // line_cd
            Some(Box::new(create_test_line())),       // line
            vec![create_test_line()],                 // lines
            13,                                       // pref_cd
            "160".to_string(),                        // post
            "東京都新宿区新宿".to_string(),           // address
            139.700258,                               // lon
            35.690921,                                // lat
            "19061116".to_string(),                   // open_ymd
            "".to_string(),                           // close_ymd
            1,                                        // e_status
            1130201,                                  // e_sort
            StopCondition::All,                       // stop_condition
            Some(1.2),                                // distance
            true,                                     // has_train_types
            Some(Box::new(create_test_train_type())), // train_type
            Some(1001),                               // company_cd
            Some("山手線".to_string()),               // line_name
            Some("ヤマノテセン".to_string()),         // line_name_k
            Some("やまのてせん".to_string()),         // line_name_h
            Some("Yamanote Line".to_string()),        // line_name_r
            Some("山手线".to_string()),               // line_name_zh
            Some("야마노테선".to_string()),           // line_name_ko
            Some("#00B261".to_string()),              // line_color_c
            Some(0),                                  // line_type
            Some("JY".to_string()),                   // line_symbol1
            None,                                     // line_symbol2
            None,                                     // line_symbol3
            None,                                     // line_symbol4
            Some("#00B261".to_string()),              // line_symbol1_color
            None,                                     // line_symbol2_color
            None,                                     // line_symbol3_color
            None,                                     // line_symbol4_color
            Some("square".to_string()),               // line_symbol1_shape
            None,                                     // line_symbol2_shape
            None,                                     // line_symbol3_shape
            None,                                     // line_symbol4_shape
            Some(1001),                               // line_group_cd
            Some(0.97),                               // average_distance
            Some(0),                                  // pass
            Some(1),                                  // type_id
            Some(1),                                  // sst_id
            Some(1),                                  // type_cd
            Some("乗換駅".to_string()),               // type_name
            Some("ノリカエエキ".to_string()),         // type_name_k
            Some("Transfer Station".to_string()),     // type_name_r
            Some("换乘站".to_string()),               // type_name_zh
            Some("환승역".to_string()),               // type_name_ko
            Some("#0066CC".to_string()),              // color
            Some(0),                                  // direction
            Some(1),                                  // kind
            TransportType::Rail,                      // transport_type
        )
    }

    fn create_test_station_minimal() -> Station {
        Station::new(
            1000001,                  // station_cd
            1000001,                  // station_g_cd
            "テスト駅".to_string(),   // station_name
            "テストエキ".to_string(), // station_name_k
            None,                     // station_name_r
            None,                     // station_name_zh
            None,                     // station_name_ko
            vec![],                   // station_numbers
            None,                     // station_number1
            None,                     // station_number2
            None,                     // station_number3
            None,                     // station_number4
            None,                     // three_letter_code
            10001,                    // line_cd
            None,                     // line
            vec![],                   // lines
            1,                        // pref_cd
            "".to_string(),           // post
            "".to_string(),           // address
            0.0,                      // lon
            0.0,                      // lat
            "".to_string(),           // open_ymd
            "".to_string(),           // close_ymd
            0,                        // e_status
            0,                        // e_sort
            StopCondition::All,       // stop_condition
            None,                     // distance
            false,                    // has_train_types
            None,                     // train_type
            None,                     // company_cd
            None,                     // line_name
            None,                     // line_name_k
            None,                     // line_name_h
            None,                     // line_name_r
            None,                     // line_name_zh
            None,                     // line_name_ko
            None,                     // line_color_c
            None,                     // line_type
            None,                     // line_symbol1
            None,                     // line_symbol2
            None,                     // line_symbol3
            None,                     // line_symbol4
            None,                     // line_symbol1_color
            None,                     // line_symbol2_color
            None,                     // line_symbol3_color
            None,                     // line_symbol4_color
            None,                     // line_symbol1_shape
            None,                     // line_symbol2_shape
            None,                     // line_symbol3_shape
            None,                     // line_symbol4_shape
            None,                     // line_group_cd
            None,                     // average_distance
            None,                     // pass
            None,                     // type_id
            None,                     // sst_id
            None,                     // type_cd
            None,                     // type_name
            None,                     // type_name_k
            None,                     // type_name_r
            None,                     // type_name_zh
            None,                     // type_name_ko
            None,                     // color
            None,                     // direction
            None,                     // kind
            TransportType::Rail,      // transport_type
        )
    }

    #[test]
    fn test_station_new_full() {
        let station = create_test_station_full();

        assert_eq!(station.station_cd, 1130201);
        assert_eq!(station.station_g_cd, 1130201);
        assert_eq!(station.station_name, "新宿");
        assert_eq!(station.station_name_k, "シンジュク");
        assert_eq!(station.station_name_r, Some("Shinjuku".to_string()));
        assert_eq!(station.station_name_zh, Some("新宿".to_string()));
        assert_eq!(station.station_name_ko, Some("신주쿠".to_string()));
        assert_eq!(station.station_numbers.len(), 1);
        assert_eq!(station.station_number1, Some("JY01".to_string()));
        assert_eq!(station.station_number2, Some("JC05".to_string()));
        assert_eq!(station.station_number3, Some("JS19".to_string()));
        assert_eq!(station.station_number4, Some("SS01".to_string()));
        assert_eq!(station.three_letter_code, Some("SJK".to_string()));
        assert_eq!(station.line_cd, 11302);
        assert!(station.line.is_some());
        assert_eq!(station.lines.len(), 1);
        assert_eq!(station.pref_cd, 13);
        assert_eq!(station.post, "160");
        assert_eq!(station.address, "東京都新宿区新宿");
        assert_eq!(station.lon, 139.700258);
        assert_eq!(station.lat, 35.690921);
        assert_eq!(station.open_ymd, "19061116");
        assert_eq!(station.close_ymd, "");
        assert_eq!(station.e_status, 1);
        assert_eq!(station.e_sort, 1130201);
        assert_eq!(station.stop_condition, StopCondition::All);
        assert_eq!(station.distance, Some(1.2));
        assert!(station.has_train_types);
        assert!(station.train_type.is_some());
    }

    #[test]
    fn test_station_new_minimal() {
        let station = create_test_station_minimal();

        assert_eq!(station.station_cd, 1000001);
        assert_eq!(station.station_g_cd, 1000001);
        assert_eq!(station.station_name, "テスト駅");
        assert_eq!(station.station_name_k, "テストエキ");
        assert_eq!(station.station_name_r, None);
        assert_eq!(station.station_name_zh, None);
        assert_eq!(station.station_name_ko, None);
        assert!(station.station_numbers.is_empty());
        assert_eq!(station.station_number1, None);
        assert_eq!(station.station_number2, None);
        assert_eq!(station.station_number3, None);
        assert_eq!(station.station_number4, None);
        assert_eq!(station.three_letter_code, None);
        assert_eq!(station.line_cd, 10001);
        assert!(station.line.is_none());
        assert!(station.lines.is_empty());
        assert_eq!(station.pref_cd, 1);
        assert_eq!(station.post, "");
        assert_eq!(station.address, "");
        assert_eq!(station.lon, 0.0);
        assert_eq!(station.lat, 0.0);
        assert_eq!(station.open_ymd, "");
        assert_eq!(station.close_ymd, "");
        assert_eq!(station.e_status, 0);
        assert_eq!(station.e_sort, 0);
        assert_eq!(station.stop_condition, StopCondition::All);
        assert_eq!(station.distance, None);
        assert!(!station.has_train_types);
        assert!(station.train_type.is_none());
    }

    #[test]
    fn test_station_clone() {
        let original = create_test_station_full();
        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(original.station_cd, cloned.station_cd);
        assert_eq!(original.station_name, cloned.station_name);
        assert_eq!(original.station_numbers.len(), cloned.station_numbers.len());
        assert_eq!(original.lines.len(), cloned.lines.len());
    }

    #[test]
    fn test_station_partial_eq() {
        let station1 = create_test_station_full();
        let station2 = create_test_station_full();
        let station3 = create_test_station_minimal();

        assert_eq!(station1, station2);
        assert_ne!(station1, station3);
    }

    #[test]
    fn test_station_debug() {
        let station = create_test_station_minimal();
        let debug_string = format!("{station:?}");

        assert!(debug_string.contains("Station"));
        assert!(debug_string.contains("テスト駅"));
        assert!(debug_string.contains("1000001"));
    }

    #[test]
    fn test_station_serialization() {
        let original = create_test_station_minimal();

        // JSONにシリアライズ
        let json = serde_json::to_string(&original).expect("シリアライズに失敗しました");

        // JSONからデシリアライズ
        let deserialized: Station =
            serde_json::from_str(&json).expect("デシリアライズに失敗しました");

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_station_with_multiple_station_numbers() {
        let station_numbers = vec![
            create_test_station_number(),
            StationNumber::new(
                "JC".to_string(),
                "#0066CC".to_string(),
                "circle".to_string(),
                "05".to_string(),
            ),
        ];

        let mut station = create_test_station_minimal();
        station.station_numbers = station_numbers;

        assert_eq!(station.station_numbers.len(), 2);
        assert_eq!(station.station_numbers[0].line_symbol, "JY");
        assert_eq!(station.station_numbers[1].line_symbol, "JC");
    }

    #[test]
    fn test_station_with_different_stop_conditions() {
        let stop_conditions = vec![
            StopCondition::All,
            StopCondition::Not,
            StopCondition::Partial,
            StopCondition::Weekday,
            StopCondition::Holiday,
            StopCondition::PartialStop,
        ];

        for condition in stop_conditions {
            let mut station = create_test_station_minimal();
            station.stop_condition = condition;
            assert_eq!(station.stop_condition, condition);
        }
    }

    #[test]
    fn test_station_coordinates() {
        let mut station = create_test_station_minimal();

        // 正常な座標値
        station.lon = 139.691706;
        station.lat = 35.689488;
        assert_eq!(station.lon, 139.691706);
        assert_eq!(station.lat, 35.689488);

        // 境界値
        station.lon = -180.0;
        station.lat = -90.0;
        assert_eq!(station.lon, -180.0);
        assert_eq!(station.lat, -90.0);

        station.lon = 180.0;
        station.lat = 90.0;
        assert_eq!(station.lon, 180.0);
        assert_eq!(station.lat, 90.0);
    }

    #[test]
    fn test_station_optional_fields() {
        let station = create_test_station_minimal();

        // Optional<String> フィールドのテスト
        assert!(station.station_name_r.is_none());
        assert!(station.station_name_zh.is_none());
        assert!(station.station_name_ko.is_none());
        assert!(station.three_letter_code.is_none());

        // Optional<i32> フィールドのテスト
        assert!(station.company_cd.is_none());
        assert!(station.line_type.is_none());
        assert!(station.type_id.is_none());

        // Optional<f64> フィールドのテスト
        assert!(station.distance.is_none());

        // Optional<Box<T>> フィールドのテスト
        assert!(station.line.is_none());
        assert!(station.train_type.is_none());
    }

    #[test]
    fn test_station_with_unicode_characters() {
        let mut station = create_test_station_minimal();

        station.station_name = "渋谷🚉".to_string();
        station.station_name_k = "シブヤ".to_string();
        station.station_name_r = Some("Shibuya 🚆".to_string());
        station.station_name_zh = Some("涩谷".to_string());
        station.station_name_ko = Some("시부야".to_string());
        station.address = "東京都渋谷区道玄坂１丁目".to_string();

        assert_eq!(station.station_name, "渋谷🚉");
        assert_eq!(station.station_name_k, "シブヤ");
        assert_eq!(station.station_name_r, Some("Shibuya 🚆".to_string()));
        assert_eq!(station.station_name_zh, Some("涩谷".to_string()));
        assert_eq!(station.station_name_ko, Some("시부야".to_string()));
        assert_eq!(station.address, "東京都渋谷区道玄坂１丁目");
    }

    #[test]
    fn test_station_distance_calculation() {
        let mut station = create_test_station_minimal();

        // distance が None の場合
        assert!(station.distance.is_none());

        // distance が Some の場合
        station.distance = Some(1.5);
        assert_eq!(station.distance, Some(1.5));

        // 負の値も許可されるかテスト
        station.distance = Some(-0.5);
        assert_eq!(station.distance, Some(-0.5));

        // 0の場合
        station.distance = Some(0.0);
        assert_eq!(station.distance, Some(0.0));
    }

    #[test]
    fn test_station_line_symbols() {
        let mut station = create_test_station_full();

        assert_eq!(station.line_symbol1, Some("JY".to_string()));
        assert_eq!(station.line_symbol1_color, Some("#00B261".to_string()));
        assert_eq!(station.line_symbol1_shape, Some("square".to_string()));

        // 複数のシンボルをテスト
        station.line_symbol2 = Some("JC".to_string());
        station.line_symbol2_color = Some("#0066CC".to_string());
        station.line_symbol2_shape = Some("circle".to_string());

        assert_eq!(station.line_symbol2, Some("JC".to_string()));
        assert_eq!(station.line_symbol2_color, Some("#0066CC".to_string()));
        assert_eq!(station.line_symbol2_shape, Some("circle".to_string()));
    }

    #[test]
    fn test_station_edge_cases() {
        let mut station = create_test_station_minimal();

        // 空文字列のテスト
        station.station_name = "".to_string();
        station.post = "".to_string();
        station.address = "".to_string();

        assert_eq!(station.station_name, "");
        assert_eq!(station.post, "");
        assert_eq!(station.address, "");

        // 極端に大きな値のテスト
        station.station_cd = i32::MAX;
        station.e_sort = i32::MAX;

        assert_eq!(station.station_cd, i32::MAX);
        assert_eq!(station.e_sort, i32::MAX);

        // 極端に小さな値のテスト
        station.station_cd = i32::MIN;
        station.e_sort = i32::MIN;

        assert_eq!(station.station_cd, i32::MIN);
        assert_eq!(station.e_sort, i32::MIN);
    }

    #[test]
    fn test_station_train_type_integration() {
        let station = create_test_station_full();

        assert!(station.has_train_types);
        assert!(station.train_type.is_some());

        if let Some(train_type) = &station.train_type {
            assert_eq!(train_type.type_name, "快速");
            assert_eq!(train_type.type_name_k, "カイソク");
            assert_eq!(train_type.color, "#FF6600");
        }
    }

    #[test]
    fn test_station_line_integration() {
        let station = create_test_station_full();

        assert!(station.line.is_some());
        assert_eq!(station.lines.len(), 1);

        if let Some(line) = &station.line {
            assert_eq!(line.line_cd, 11302);
            assert_eq!(line.line_name, "山手線");
            assert_eq!(line.line_color_c, Some("#00B261".to_string()));
        }

        assert_eq!(station.lines[0].line_cd, 11302);
        assert_eq!(station.lines[0].line_name, "山手線");
    }

    #[test]
    fn test_station_with_bus_transport_type() {
        let bus_stop = Station::new(
            9000001,                           // station_cd
            9000001,                           // station_g_cd
            "新宿駅前".to_string(),            // station_name
            "シンジュクエキマエ".to_string(),  // station_name_k
            Some("Shinjuku Sta.".to_string()), // station_name_r
            None,                              // station_name_zh
            None,                              // station_name_ko
            vec![],                            // station_numbers
            None,                              // station_number1
            None,                              // station_number2
            None,                              // station_number3
            None,                              // station_number4
            None,                              // three_letter_code
            99001,                             // line_cd
            None,                              // line
            vec![],                            // lines
            13,                                // pref_cd (東京都)
            "".to_string(),                    // post
            "".to_string(),                    // address
            139.700464,                        // lon
            35.689738,                         // lat
            "".to_string(),                    // open_ymd
            "".to_string(),                    // close_ymd
            0,                                 // e_status
            1,                                 // e_sort
            StopCondition::All,                // stop_condition
            None,                              // distance
            false,                             // has_train_types
            None,                              // train_type
            None,                              // company_cd
            None,                              // line_name
            None,                              // line_name_k
            None,                              // line_name_h
            None,                              // line_name_r
            None,                              // line_name_zh
            None,                              // line_name_ko
            None,                              // line_color_c
            None,                              // line_type
            None,                              // line_symbol1
            None,                              // line_symbol2
            None,                              // line_symbol3
            None,                              // line_symbol4
            None,                              // line_symbol1_color
            None,                              // line_symbol2_color
            None,                              // line_symbol3_color
            None,                              // line_symbol4_color
            None,                              // line_symbol1_shape
            None,                              // line_symbol2_shape
            None,                              // line_symbol3_shape
            None,                              // line_symbol4_shape
            None,                              // line_group_cd
            None,                              // average_distance
            None,                              // pass
            None,                              // type_id
            None,                              // sst_id
            None,                              // type_cd
            None,                              // type_name
            None,                              // type_name_k
            None,                              // type_name_r
            None,                              // type_name_zh
            None,                              // type_name_ko
            None,                              // color
            None,                              // direction
            None,                              // kind
            TransportType::Bus,                // transport_type
        );

        assert_eq!(bus_stop.station_cd, 9000001);
        assert_eq!(bus_stop.station_name, "新宿駅前");
        assert_eq!(bus_stop.transport_type, TransportType::Bus);

        // JSONシリアライゼーションでtransport_typeが正しく出力されることを確認
        let json = serde_json::to_string(&bus_stop).expect("シリアライゼーションに失敗");
        assert!(json.contains("\"transport_type\":\"Bus\"")); // Bus enum variant
    }
}
