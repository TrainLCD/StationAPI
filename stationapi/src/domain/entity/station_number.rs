use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StationNumber {
    pub line_symbol: String,
    pub line_symbol_color: String,
    pub line_symbol_shape: String,
    pub station_number: String,
}

impl StationNumber {
    pub fn new(
        line_symbol: String,
        line_symbol_color: String,
        line_symbol_shape: String,
        station_number: String,
    ) -> Self {
        Self {
            line_symbol,
            line_symbol_color,
            line_symbol_shape,
            station_number,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_station_number_new() {
        let station_number = StationNumber::new(
            "JY".to_string(),
            "#00a650".to_string(),
            "round".to_string(),
            "01".to_string(),
        );

        assert_eq!(station_number.line_symbol, "JY");
        assert_eq!(station_number.line_symbol_color, "#00a650");
        assert_eq!(station_number.line_symbol_shape, "round");
        assert_eq!(station_number.station_number, "01");
    }

    #[test]
    fn test_station_number_clone() {
        let original = StationNumber::new(
            "JY".to_string(),
            "#00a650".to_string(),
            "round".to_string(),
            "01".to_string(),
        );

        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(original.line_symbol, cloned.line_symbol);
        assert_eq!(original.line_symbol_color, cloned.line_symbol_color);
        assert_eq!(original.line_symbol_shape, cloned.line_symbol_shape);
        assert_eq!(original.station_number, cloned.station_number);
    }

    #[test]
    fn test_station_number_partial_eq() {
        let station_number1 = StationNumber::new(
            "JY".to_string(),
            "#00a650".to_string(),
            "round".to_string(),
            "01".to_string(),
        );

        let station_number2 = StationNumber::new(
            "JY".to_string(),
            "#00a650".to_string(),
            "round".to_string(),
            "01".to_string(),
        );

        let station_number3 = StationNumber::new(
            "JC".to_string(),
            "#ff0000".to_string(),
            "square".to_string(),
            "02".to_string(),
        );

        assert_eq!(station_number1, station_number2);
        assert_ne!(station_number1, station_number3);
    }

    #[test]
    fn test_station_number_debug() {
        let station_number = StationNumber::new(
            "JY".to_string(),
            "#00a650".to_string(),
            "round".to_string(),
            "01".to_string(),
        );

        let debug_string = format!("{station_number:?}");
        assert!(debug_string.contains("StationNumber"));
        assert!(debug_string.contains("JY"));
        assert!(debug_string.contains("#00a650"));
        assert!(debug_string.contains("round"));
        assert!(debug_string.contains("01"));
    }

    #[test]
    fn test_station_number_serialize_deserialize() {
        let original = StationNumber::new(
            "JY".to_string(),
            "#00a650".to_string(),
            "round".to_string(),
            "01".to_string(),
        );

        // JSONにシリアライズ
        let json = serde_json::to_string(&original).expect("シリアライズに失敗しました");

        // JSONからデシリアライズ
        let deserialized: StationNumber =
            serde_json::from_str(&json).expect("デシリアライズに失敗しました");

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_station_number_with_empty_strings() {
        let station_number =
            StationNumber::new(String::new(), String::new(), String::new(), String::new());

        assert_eq!(station_number.line_symbol, "");
        assert_eq!(station_number.line_symbol_color, "");
        assert_eq!(station_number.line_symbol_shape, "");
        assert_eq!(station_number.station_number, "");
    }

    #[test]
    fn test_station_number_with_special_characters() {
        let station_number = StationNumber::new(
            "東京メトロ".to_string(),
            "#ff0000".to_string(),
            "circle".to_string(),
            "駅01".to_string(),
        );

        assert_eq!(station_number.line_symbol, "東京メトロ");
        assert_eq!(station_number.line_symbol_color, "#ff0000");
        assert_eq!(station_number.line_symbol_shape, "circle");
        assert_eq!(station_number.station_number, "駅01");
    }
}
