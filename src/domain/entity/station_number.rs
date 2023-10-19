use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StationNumber {
    pub line_symbol: String,
    pub line_symbol_color: String,
    pub line_symbol_shape: String,
    pub station_number: String,
}
