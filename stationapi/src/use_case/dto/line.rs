use crate::{
    domain::{
        entity::{gtfs::TransportType, line::Line},
        ipa::compute_line_ipa_cached,
    },
    model::{Line as ModelLine, TransportType as ModelTransportType},
    use_case::dto::tts::to_tts_segments,
};

impl From<Line> for ModelLine {
    fn from(line: Line) -> Self {
        let ipa = compute_line_ipa_cached(&line.line_name_k, line.line_name_r.as_deref());
        let name_ipa = ipa.name_ipa.clone();
        let name_roman_ipa = ipa.name_roman_ipa.clone();
        let name_tts_segments = to_tts_segments(&ipa.tts_segments);
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
            name_ipa,
            name_roman_ipa,
            name_tts_segments,
        }
    }
}

fn convert_transport_type(t: TransportType) -> i32 {
    match t {
        TransportType::Rail => ModelTransportType::Rail as i32,
        TransportType::Bus => ModelTransportType::Bus as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LineType as ModelLineType;

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
        let bus_line = create_test_line(TransportType::Bus, Some(ModelLineType::Subway as i32));
        let model_line: ModelLine = bus_line.into();

        assert_eq!(model_line.line_type, ModelLineType::OtherLineType as i32);
        assert_eq!(model_line.transport_type, ModelTransportType::Bus as i32);
    }

    #[test]
    fn test_bus_line_with_bullet_train_line_type_returns_other() {
        // バス路線にBulletTrain(1)が設定されていても、OtherLineType(0)が返される
        let bus_line =
            create_test_line(TransportType::Bus, Some(ModelLineType::BulletTrain as i32));
        let model_line: ModelLine = bus_line.into();

        assert_eq!(model_line.line_type, ModelLineType::OtherLineType as i32);
    }

    #[test]
    fn test_bus_line_with_normal_line_type_returns_other() {
        // バス路線にNormal(2)が設定されていても、OtherLineType(0)が返される
        let bus_line = create_test_line(TransportType::Bus, Some(ModelLineType::Normal as i32));
        let model_line: ModelLine = bus_line.into();

        assert_eq!(model_line.line_type, ModelLineType::OtherLineType as i32);
    }

    #[test]
    fn test_bus_line_with_tram_line_type_returns_other() {
        // バス路線にTram(4)が設定されていても、OtherLineType(0)が返される
        let bus_line = create_test_line(TransportType::Bus, Some(ModelLineType::Tram as i32));
        let model_line: ModelLine = bus_line.into();

        assert_eq!(model_line.line_type, ModelLineType::OtherLineType as i32);
    }

    #[test]
    fn test_bus_line_with_monorail_line_type_returns_other() {
        // バス路線にMonorailOrAGT(5)が設定されていても、OtherLineType(0)が返される
        let bus_line = create_test_line(
            TransportType::Bus,
            Some(ModelLineType::MonorailOrAgt as i32),
        );
        let model_line: ModelLine = bus_line.into();

        assert_eq!(model_line.line_type, ModelLineType::OtherLineType as i32);
    }

    #[test]
    fn test_bus_line_with_none_line_type_returns_other() {
        // バス路線でline_typeがNoneでも、OtherLineType(0)が返される
        let bus_line = create_test_line(TransportType::Bus, None);
        let model_line: ModelLine = bus_line.into();

        assert_eq!(model_line.line_type, ModelLineType::OtherLineType as i32);
    }

    #[test]
    fn test_bus_line_with_other_line_type_returns_other() {
        // バス路線でOtherLineType(0)が設定されていれば、そのままOtherLineType(0)が返される
        let bus_line = create_test_line(
            TransportType::Bus,
            Some(ModelLineType::OtherLineType as i32),
        );
        let model_line: ModelLine = bus_line.into();

        assert_eq!(model_line.line_type, ModelLineType::OtherLineType as i32);
    }

    // ============================================
    // 鉄道路線の line_type 変換テスト
    // ============================================

    #[test]
    fn test_rail_line_with_subway_line_type_preserved() {
        // 鉄道路線ではSubway(3)がそのまま返される
        let rail_line = create_test_line(TransportType::Rail, Some(ModelLineType::Subway as i32));
        let model_line: ModelLine = rail_line.into();

        assert_eq!(model_line.line_type, ModelLineType::Subway as i32);
        assert_eq!(model_line.transport_type, ModelTransportType::Rail as i32);
    }

    #[test]
    fn test_rail_line_with_bullet_train_line_type_preserved() {
        // 鉄道路線ではBulletTrain(1)がそのまま返される
        let rail_line =
            create_test_line(TransportType::Rail, Some(ModelLineType::BulletTrain as i32));
        let model_line: ModelLine = rail_line.into();

        assert_eq!(model_line.line_type, ModelLineType::BulletTrain as i32);
    }

    #[test]
    fn test_rail_line_with_normal_line_type_preserved() {
        // 鉄道路線ではNormal(2)がそのまま返される
        let rail_line = create_test_line(TransportType::Rail, Some(ModelLineType::Normal as i32));
        let model_line: ModelLine = rail_line.into();

        assert_eq!(model_line.line_type, ModelLineType::Normal as i32);
    }

    #[test]
    fn test_rail_line_with_tram_line_type_preserved() {
        // 鉄道路線ではTram(4)がそのまま返される
        let rail_line = create_test_line(TransportType::Rail, Some(ModelLineType::Tram as i32));
        let model_line: ModelLine = rail_line.into();

        assert_eq!(model_line.line_type, ModelLineType::Tram as i32);
    }

    #[test]
    fn test_rail_line_with_monorail_line_type_preserved() {
        // 鉄道路線ではMonorailOrAGT(5)がそのまま返される
        let rail_line = create_test_line(
            TransportType::Rail,
            Some(ModelLineType::MonorailOrAgt as i32),
        );
        let model_line: ModelLine = rail_line.into();

        assert_eq!(model_line.line_type, ModelLineType::MonorailOrAgt as i32);
    }

    #[test]
    fn test_rail_line_with_none_line_type_defaults_to_other() {
        // 鉄道路線でline_typeがNoneの場合、OtherLineType(0)がデフォルトで返される
        let rail_line = create_test_line(TransportType::Rail, None);
        let model_line: ModelLine = rail_line.into();

        assert_eq!(model_line.line_type, ModelLineType::OtherLineType as i32);
    }

    // ============================================
    // convert_transport_type 関数のテスト
    // ============================================

    #[test]
    fn test_convert_transport_type_rail() {
        let result = convert_transport_type(TransportType::Rail);
        assert_eq!(result, ModelTransportType::Rail as i32);
    }

    #[test]
    fn test_convert_transport_type_bus() {
        let result = convert_transport_type(TransportType::Bus);
        assert_eq!(result, ModelTransportType::Bus as i32);
    }

    // ============================================
    // Line から ModelLine への変換テスト (その他フィールド)
    // ============================================

    #[test]
    fn test_line_to_model_line_basic_fields() {
        let line = create_test_line(TransportType::Rail, Some(ModelLineType::Normal as i32));
        let model_line: ModelLine = line.into();

        assert_eq!(model_line.id, 1);
        assert_eq!(model_line.name_short, "テスト路線");
        assert_eq!(model_line.name_katakana, "テストロセン");
        assert_eq!(model_line.name_full, "てすとろせん");
        assert_eq!(model_line.name_roman, Some("Test Line".to_string()));
        assert_eq!(model_line.name_chinese, Some("测试线路".to_string()));
        assert_eq!(model_line.name_korean, Some("테스트노선".to_string()));
        assert_eq!(model_line.color, "#FF0000");
        assert_eq!(model_line.status, 0);
        assert!((model_line.average_distance - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_line_to_model_line_with_none_optional_fields() {
        let mut line = create_test_line(TransportType::Rail, None);
        line.line_name_r = None;
        line.line_name_zh = None;
        line.line_name_ko = None;
        line.line_color_c = None;
        line.average_distance = None;

        let model_line: ModelLine = line.into();

        // name_romanはNoneでもSome("")になる
        assert_eq!(model_line.name_roman, Some("".to_string()));
        assert_eq!(model_line.name_chinese, None);
        assert_eq!(model_line.name_korean, None);
        // colorはNoneでも""になる
        assert_eq!(model_line.color, "");
        // average_distanceはNoneでも0.0になる
        assert!((model_line.average_distance - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_line_to_model_line_empty_line_symbols() {
        let line = create_test_line(TransportType::Rail, None);
        let model_line: ModelLine = line.into();

        assert!(model_line.line_symbols.is_empty());
    }

    #[test]
    fn test_line_to_model_line_without_station() {
        let line = create_test_line(TransportType::Rail, None);
        let model_line: ModelLine = line.into();

        assert!(model_line.station.is_none());
    }

    #[test]
    fn test_line_to_model_line_without_company() {
        let line = create_test_line(TransportType::Rail, None);
        let model_line: ModelLine = line.into();

        assert!(model_line.company.is_none());
    }

    #[test]
    fn test_line_to_model_line_without_train_type() {
        let line = create_test_line(TransportType::Rail, None);
        let model_line: ModelLine = line.into();

        assert!(model_line.train_type.is_none());
    }

    // ============================================
    // 境界値・エッジケースのテスト
    // ============================================

    #[test]
    fn test_line_type_boundary_values() {
        // line_typeの境界値テスト
        let test_cases = vec![
            (0, ModelLineType::OtherLineType as i32),
            (1, ModelLineType::BulletTrain as i32),
            (2, ModelLineType::Normal as i32),
            (3, ModelLineType::Subway as i32),
            (4, ModelLineType::Tram as i32),
            (5, ModelLineType::MonorailOrAgt as i32),
        ];

        for (input, expected) in test_cases {
            let line = create_test_line(TransportType::Rail, Some(input));
            let model_line: ModelLine = line.into();
            assert_eq!(
                model_line.line_type, expected,
                "Failed for line_type: {input}"
            );
        }
    }

    #[test]
    fn test_line_type_unknown_value_preserved_for_rail() {
        // 鉄道路線で未知のline_type値は保持される
        let line = create_test_line(TransportType::Rail, Some(99));
        let model_line: ModelLine = line.into();

        assert_eq!(model_line.line_type, 99);
    }

    #[test]
    fn test_bus_line_unknown_line_type_returns_other() {
        // バス路線では未知のline_type値もOtherLineTypeになる
        let line = create_test_line(TransportType::Bus, Some(99));
        let model_line: ModelLine = line.into();

        assert_eq!(model_line.line_type, ModelLineType::OtherLineType as i32);
    }

    #[test]
    fn test_line_cd_to_id_conversion() {
        // line_cdがu32に正しく変換されることを確認
        let mut line = create_test_line(TransportType::Rail, None);
        line.line_cd = 12345;
        let model_line: ModelLine = line.into();

        assert_eq!(model_line.id, 12345);
    }

    // ============================================
    // IPA 変換テスト
    // ============================================

    #[test]
    fn test_name_ipa_sen_suffix_replaced_with_line() {
        // name_ipa はカタカナ由来
        let mut line = create_test_line(TransportType::Rail, None);
        line.line_name_k = "セイブイケブクロセン".to_string();
        let model_line: ModelLine = line.into();

        assert_eq!(
            model_line.name_ipa,
            Some("se.ibɯ.ikebɯkɯɾo laɪn".to_string())
        );
    }

    #[test]
    fn test_name_ipa_honsen_suffix_replaced_with_main_line() {
        let mut line = create_test_line(TransportType::Rail, None);
        line.line_name_k = "トウカイドウホンセン".to_string();
        let model_line: ModelLine = line.into();

        assert_eq!(
            model_line.name_ipa,
            Some("to.ɯka.ido.ɯ meɪn laɪn".to_string())
        );
    }

    #[test]
    fn test_name_ipa_shinkansen_preserved() {
        let mut line = create_test_line(TransportType::Rail, None);
        line.line_name_k = "トウホクシンカンセン".to_string();
        let model_line: ModelLine = line.into();

        assert_eq!(model_line.name_ipa, Some("to.ɯhokɯɕiŋkanseɴ".to_string()));
    }

    #[test]
    fn test_name_roman_ipa_prefers_romanized_line_name_for_keisei() {
        let mut line = create_test_line(TransportType::Rail, None);
        line.line_name_k = "ケイセイホンセン".to_string();
        line.line_name_r = Some("Keisei Main Line".to_string());
        let model_line: ModelLine = line.into();

        assert_eq!(model_line.name_ipa, Some("ke.ise.i meɪn laɪn".to_string()));
        assert_eq!(
            model_line.name_roman_ipa,
            Some("keːseː meɪn laɪn".to_string())
        );
    }

    #[test]
    fn test_name_ipa_with_nakaguro_is_not_null() {
        // 「・」を含む路線名でも name_ipa / name_tts_segments が null / 空にならない
        let mut line = create_test_line(TransportType::Rail, None);
        line.line_name_k = "チュウオウ・ソウブセン".to_string();
        line.line_name_r = None;
        let model_line: ModelLine = line.into();

        // 「・」が空白へ正規化され（連続空白も残さず）、「線」→ " laɪn" 置換も維持される。
        let ipa = model_line.name_ipa.expect("name_ipa should be present");
        assert!(!ipa.is_empty());
        assert!(ipa.contains(" laɪn"));
        assert!(!ipa.contains("  "));
        assert!(!ipa.contains('・'));
        assert!(!model_line.name_tts_segments.is_empty());
    }

    #[test]
    fn test_name_ipa_with_fullwidth_parentheses_is_not_null() {
        // 全角括弧を含む路線名でも name_ipa / name_tts_segments が null / 空にならない
        let mut line = create_test_line(TransportType::Rail, None);
        line.line_name_k = "トウキョウサクラトラム（トデンアラカワセン）".to_string();
        line.line_name_r = None;
        let model_line: ModelLine = line.into();

        assert!(model_line.name_ipa.is_some());
        assert!(!model_line.name_ipa.as_deref().unwrap().is_empty());
        assert!(!model_line.name_tts_segments.is_empty());
    }

    #[test]
    fn test_name_ipa_empty_result_is_normalized_to_none() {
        let mut line = create_test_line(TransportType::Rail, None);
        line.line_name_k = "".to_string();
        line.line_name_r = None;
        let model_line: ModelLine = line.into();

        assert_eq!(model_line.name_ipa, None);
        assert_eq!(model_line.name_roman_ipa, None);
    }
}
