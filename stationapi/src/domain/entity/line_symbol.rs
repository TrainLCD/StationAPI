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
    use super::LineSymbol;

    #[test]
    fn new() {
        let line_symbol = LineSymbol::new(
            "JY".to_string(),
            "#80C241".to_string(),
            "SQUARE".to_string(),
        );
        assert_eq!(
            line_symbol,
            LineSymbol {
                symbol: "JY".to_string(),
                color: "#80C241".to_string(),
                shape: "SQUARE".to_string()
            }
        );
    }
}
