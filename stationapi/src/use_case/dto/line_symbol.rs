use crate::{domain::entity::line_symbol::LineSymbol, model::LineSymbol as ModelLineSymbol};

impl From<LineSymbol> for ModelLineSymbol {
    fn from(symbol: LineSymbol) -> Self {
        Self {
            symbol: symbol.symbol,
            color: symbol.color,
            shape: symbol.shape,
        }
    }
}
