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
    use super::StationNumber;

    #[test]
    fn new() {
        let station_number = StationNumber::new(
            "JY".to_string(),
            "#80C241".to_string(),
            "SQUARE".to_string(),
            "01".to_string(),
        );
        assert_eq!(
            station_number,
            StationNumber {
                line_symbol: "JY".to_string(),
                line_symbol_color: "#80C241".to_string(),
                line_symbol_shape: "SQUARE".to_string(),
                station_number: "01".to_string()
            }
        );
    }
}
