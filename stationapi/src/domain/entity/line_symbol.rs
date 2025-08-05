use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LineSymbol {
    pub symbol: String,
    pub color: String,
    pub shape: String,
}

impl LineSymbol {
    pub fn new(symbol: String, color: String, shape: String) -> Self {
        Self {
            symbol,
            color,
            shape,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let symbol = "JY".to_string();
        let color = "#00B261".to_string();
        let shape = "square".to_string();

        let line_symbol = LineSymbol::new(symbol.clone(), color.clone(), shape.clone());

        assert_eq!(line_symbol.symbol, symbol);
        assert_eq!(line_symbol.color, color);
        assert_eq!(line_symbol.shape, shape);
    }

    #[test]
    fn test_clone() {
        let original = LineSymbol::new(
            "JY".to_string(),
            "#00B261".to_string(),
            "square".to_string(),
        );

        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(original.symbol, cloned.symbol);
        assert_eq!(original.color, cloned.color);
        assert_eq!(original.shape, cloned.shape);
    }

    #[test]
    fn test_partial_eq() {
        let line_symbol1 = LineSymbol::new(
            "JY".to_string(),
            "#00B261".to_string(),
            "square".to_string(),
        );

        let line_symbol2 = LineSymbol::new(
            "JY".to_string(),
            "#00B261".to_string(),
            "square".to_string(),
        );

        let line_symbol3 = LineSymbol::new(
            "JR".to_string(),
            "#00B261".to_string(),
            "square".to_string(),
        );

        // 同じ値を持つ構造体は等しい
        assert_eq!(line_symbol1, line_symbol2);

        // 異なる値を持つ構造体は等しくない
        assert_ne!(line_symbol1, line_symbol3);
    }

    #[test]
    fn test_serialization() {
        let line_symbol = LineSymbol::new(
            "JY".to_string(),
            "#00B261".to_string(),
            "square".to_string(),
        );

        // JSONにシリアライズ
        let json = serde_json::to_string(&line_symbol).expect("Failed to serialize");

        // JSONからデシリアライズ
        let deserialized: LineSymbol = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(line_symbol, deserialized);
    }

    #[test]
    fn test_with_empty_strings() {
        let line_symbol = LineSymbol::new("".to_string(), "".to_string(), "".to_string());

        assert_eq!(line_symbol.symbol, "");
        assert_eq!(line_symbol.color, "");
        assert_eq!(line_symbol.shape, "");
    }

    #[test]
    fn test_with_unicode_characters() {
        let line_symbol = LineSymbol::new("山手線".to_string(), "#緑".to_string(), "○".to_string());

        assert_eq!(line_symbol.symbol, "山手線");
        assert_eq!(line_symbol.color, "#緑");
        assert_eq!(line_symbol.shape, "○");
    }

    #[test]
    fn test_debug_output() {
        let line_symbol = LineSymbol::new(
            "JY".to_string(),
            "#00B261".to_string(),
            "square".to_string(),
        );

        let debug_output = format!("{line_symbol:?}");
        assert!(debug_output.contains("JY"));
        assert!(debug_output.contains("#00B261"));
        assert!(debug_output.contains("square"));
    }
}
