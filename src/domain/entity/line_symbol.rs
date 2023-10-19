use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
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
