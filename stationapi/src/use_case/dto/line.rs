use crate::{
    domain::entity::{gtfs::TransportType, line::Line},
    proto::{Line as GrpcLine, TransportType as GrpcTransportType},
};

impl From<Line> for GrpcLine {
    fn from(line: Line) -> Self {
        // バス路線の場合は line_type を OtherLineType (0) に強制
        // (鉄道用の line_type が誤って設定されている可能性があるため)
        let line_type = if line.transport_type == TransportType::Bus {
            0 // OtherLineType
        } else {
            line.line_type.unwrap_or_default()
        };

        Self {
            id: line.line_cd as u32,
            name_short: line.line_name,
            name_katakana: line.line_name_k,
            name_full: line.line_name_h,
            name_roman: Some(line.line_name_r.unwrap_or_default()),
            name_chinese: line.line_name_zh,
            name_korean: line.line_name_ko,
            color: line.line_color_c.unwrap_or_default(),
            line_type,
            line_symbols: line.line_symbols.into_iter().map(|s| s.into()).collect(),
            status: line.e_status,
            station: line.station.map(|s| Box::new(s.into())),
            company: line.company.map(|c| c.into()),
            train_type: line
                .train_type
                .map(|train_type| Box::new(train_type.into())),
            average_distance: line.average_distance.unwrap_or(0.0),
            transport_type: convert_transport_type(line.transport_type),
        }
    }
}

fn convert_transport_type(t: TransportType) -> i32 {
    match t {
        TransportType::Rail => GrpcTransportType::Rail as i32,
        TransportType::Bus => GrpcTransportType::Bus as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::LineType as GrpcLineType;

    fn create_test_line(transport_type: TransportType, line_type: Option<i32>) -> Line {
        Line {
            line_cd: 1,
            company_cd: 1,
            company: None,
            line_name: "テスト路線".to_string(),
            line_name_k: "テストロセン".to_string(),
            line_name_h: "てすとろせん".to_string(),
            line_name_r: Some("Test Line".to_string()),
            line_name_zh: Some("测试线路".to_string()),
            line_name_ko: Some("테스트노선".to_string()),
            line_color_c: Some("#FF0000".to_string()),
            line_type,
            line_symbols: vec![],
            line_symbol1: None,
            line_symbol2: None,
            line_symbol3: None,
            line_symbol4: None,
            line_symbol1_color: None,
            line_symbol2_color: None,
            line_symbol3_color: None,
            line_symbol4_color: None,
            line_symbol1_shape: None,
            line_symbol2_shape: None,
            line_symbol3_shape: None,
            line_symbol4_shape: None,
            e_status: 0,
            e_sort: 1,
            station: None,
            train_type: None,
            line_group_cd: None,
            station_cd: None,
            station_g_cd: None,
            average_distance: Some(1.5),
            type_cd: None,
            transport_type,
        }
    }

    // ============================================
    // バス路線の line_type 変換テスト
    // ============================================

    #[test]
    fn test_bus_line_with_subway_line_type_returns_other() {
        // バス路線にSubway(3)が設定されていても、OtherLineType(0)が返される
        let bus_line = create_test_line(TransportType::Bus, Some(GrpcLineType::Subway as i32));
        let grpc_line: GrpcLine = bus_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::OtherLineType as i32);
        assert_eq!(grpc_line.transport_type, GrpcTransportType::Bus as i32);
    }

    #[test]
    fn test_bus_line_with_bullet_train_line_type_returns_other() {
        // バス路線にBulletTrain(1)が設定されていても、OtherLineType(0)が返される
        let bus_line = create_test_line(TransportType::Bus, Some(GrpcLineType::BulletTrain as i32));
        let grpc_line: GrpcLine = bus_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::OtherLineType as i32);
    }

    #[test]
    fn test_bus_line_with_normal_line_type_returns_other() {
        // バス路線にNormal(2)が設定されていても、OtherLineType(0)が返される
        let bus_line = create_test_line(TransportType::Bus, Some(GrpcLineType::Normal as i32));
        let grpc_line: GrpcLine = bus_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::OtherLineType as i32);
    }

    #[test]
    fn test_bus_line_with_tram_line_type_returns_other() {
        // バス路線にTram(4)が設定されていても、OtherLineType(0)が返される
        let bus_line = create_test_line(TransportType::Bus, Some(GrpcLineType::Tram as i32));
        let grpc_line: GrpcLine = bus_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::OtherLineType as i32);
    }

    #[test]
    fn test_bus_line_with_monorail_line_type_returns_other() {
        // バス路線にMonorailOrAGT(5)が設定されていても、OtherLineType(0)が返される
        let bus_line =
            create_test_line(TransportType::Bus, Some(GrpcLineType::MonorailOrAgt as i32));
        let grpc_line: GrpcLine = bus_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::OtherLineType as i32);
    }

    #[test]
    fn test_bus_line_with_none_line_type_returns_other() {
        // バス路線でline_typeがNoneでも、OtherLineType(0)が返される
        let bus_line = create_test_line(TransportType::Bus, None);
        let grpc_line: GrpcLine = bus_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::OtherLineType as i32);
    }

    #[test]
    fn test_bus_line_with_other_line_type_returns_other() {
        // バス路線でOtherLineType(0)が設定されていれば、そのままOtherLineType(0)が返される
        let bus_line =
            create_test_line(TransportType::Bus, Some(GrpcLineType::OtherLineType as i32));
        let grpc_line: GrpcLine = bus_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::OtherLineType as i32);
    }

    // ============================================
    // 鉄道路線の line_type 変換テスト
    // ============================================

    #[test]
    fn test_rail_line_with_subway_line_type_preserved() {
        // 鉄道路線ではSubway(3)がそのまま返される
        let rail_line = create_test_line(TransportType::Rail, Some(GrpcLineType::Subway as i32));
        let grpc_line: GrpcLine = rail_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::Subway as i32);
        assert_eq!(grpc_line.transport_type, GrpcTransportType::Rail as i32);
    }

    #[test]
    fn test_rail_line_with_bullet_train_line_type_preserved() {
        // 鉄道路線ではBulletTrain(1)がそのまま返される
        let rail_line =
            create_test_line(TransportType::Rail, Some(GrpcLineType::BulletTrain as i32));
        let grpc_line: GrpcLine = rail_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::BulletTrain as i32);
    }

    #[test]
    fn test_rail_line_with_normal_line_type_preserved() {
        // 鉄道路線ではNormal(2)がそのまま返される
        let rail_line = create_test_line(TransportType::Rail, Some(GrpcLineType::Normal as i32));
        let grpc_line: GrpcLine = rail_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::Normal as i32);
    }

    #[test]
    fn test_rail_line_with_tram_line_type_preserved() {
        // 鉄道路線ではTram(4)がそのまま返される
        let rail_line = create_test_line(TransportType::Rail, Some(GrpcLineType::Tram as i32));
        let grpc_line: GrpcLine = rail_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::Tram as i32);
    }

    #[test]
    fn test_rail_line_with_monorail_line_type_preserved() {
        // 鉄道路線ではMonorailOrAGT(5)がそのまま返される
        let rail_line = create_test_line(
            TransportType::Rail,
            Some(GrpcLineType::MonorailOrAgt as i32),
        );
        let grpc_line: GrpcLine = rail_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::MonorailOrAgt as i32);
    }

    #[test]
    fn test_rail_line_with_none_line_type_defaults_to_other() {
        // 鉄道路線でline_typeがNoneの場合、OtherLineType(0)がデフォルトで返される
        let rail_line = create_test_line(TransportType::Rail, None);
        let grpc_line: GrpcLine = rail_line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::OtherLineType as i32);
    }

    // ============================================
    // convert_transport_type 関数のテスト
    // ============================================

    #[test]
    fn test_convert_transport_type_rail() {
        let result = convert_transport_type(TransportType::Rail);
        assert_eq!(result, GrpcTransportType::Rail as i32);
    }

    #[test]
    fn test_convert_transport_type_bus() {
        let result = convert_transport_type(TransportType::Bus);
        assert_eq!(result, GrpcTransportType::Bus as i32);
    }

    // ============================================
    // Line から GrpcLine への変換テスト (その他フィールド)
    // ============================================

    #[test]
    fn test_line_to_grpc_line_basic_fields() {
        let line = create_test_line(TransportType::Rail, Some(GrpcLineType::Normal as i32));
        let grpc_line: GrpcLine = line.into();

        assert_eq!(grpc_line.id, 1);
        assert_eq!(grpc_line.name_short, "テスト路線");
        assert_eq!(grpc_line.name_katakana, "テストロセン");
        assert_eq!(grpc_line.name_full, "てすとろせん");
        assert_eq!(grpc_line.name_roman, Some("Test Line".to_string()));
        assert_eq!(grpc_line.name_chinese, Some("测试线路".to_string()));
        assert_eq!(grpc_line.name_korean, Some("테스트노선".to_string()));
        assert_eq!(grpc_line.color, "#FF0000");
        assert_eq!(grpc_line.status, 0);
        assert!((grpc_line.average_distance - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_line_to_grpc_line_with_none_optional_fields() {
        let mut line = create_test_line(TransportType::Rail, None);
        line.line_name_r = None;
        line.line_name_zh = None;
        line.line_name_ko = None;
        line.line_color_c = None;
        line.average_distance = None;

        let grpc_line: GrpcLine = line.into();

        // name_romanはNoneでもSome("")になる
        assert_eq!(grpc_line.name_roman, Some("".to_string()));
        assert_eq!(grpc_line.name_chinese, None);
        assert_eq!(grpc_line.name_korean, None);
        // colorはNoneでも""になる
        assert_eq!(grpc_line.color, "");
        // average_distanceはNoneでも0.0になる
        assert!((grpc_line.average_distance - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_line_to_grpc_line_empty_line_symbols() {
        let line = create_test_line(TransportType::Rail, None);
        let grpc_line: GrpcLine = line.into();

        assert!(grpc_line.line_symbols.is_empty());
    }

    #[test]
    fn test_line_to_grpc_line_without_station() {
        let line = create_test_line(TransportType::Rail, None);
        let grpc_line: GrpcLine = line.into();

        assert!(grpc_line.station.is_none());
    }

    #[test]
    fn test_line_to_grpc_line_without_company() {
        let line = create_test_line(TransportType::Rail, None);
        let grpc_line: GrpcLine = line.into();

        assert!(grpc_line.company.is_none());
    }

    #[test]
    fn test_line_to_grpc_line_without_train_type() {
        let line = create_test_line(TransportType::Rail, None);
        let grpc_line: GrpcLine = line.into();

        assert!(grpc_line.train_type.is_none());
    }

    // ============================================
    // 境界値・エッジケースのテスト
    // ============================================

    #[test]
    fn test_line_type_boundary_values() {
        // line_typeの境界値テスト
        let test_cases = vec![
            (0, GrpcLineType::OtherLineType as i32),
            (1, GrpcLineType::BulletTrain as i32),
            (2, GrpcLineType::Normal as i32),
            (3, GrpcLineType::Subway as i32),
            (4, GrpcLineType::Tram as i32),
            (5, GrpcLineType::MonorailOrAgt as i32),
        ];

        for (input, expected) in test_cases {
            let line = create_test_line(TransportType::Rail, Some(input));
            let grpc_line: GrpcLine = line.into();
            assert_eq!(
                grpc_line.line_type, expected,
                "Failed for line_type: {input}"
            );
        }
    }

    #[test]
    fn test_line_type_unknown_value_preserved_for_rail() {
        // 鉄道路線で未知のline_type値は保持される
        let line = create_test_line(TransportType::Rail, Some(99));
        let grpc_line: GrpcLine = line.into();

        assert_eq!(grpc_line.line_type, 99);
    }

    #[test]
    fn test_bus_line_unknown_line_type_returns_other() {
        // バス路線では未知のline_type値もOtherLineTypeになる
        let line = create_test_line(TransportType::Bus, Some(99));
        let grpc_line: GrpcLine = line.into();

        assert_eq!(grpc_line.line_type, GrpcLineType::OtherLineType as i32);
    }

    #[test]
    fn test_line_cd_to_id_conversion() {
        // line_cdがu32に正しく変換されることを確認
        let mut line = create_test_line(TransportType::Rail, None);
        line.line_cd = 12345;
        let grpc_line: GrpcLine = line.into();

        assert_eq!(grpc_line.id, 12345);
    }
}
