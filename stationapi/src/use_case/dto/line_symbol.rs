use crate::{domain::entity::line_symbol::LineSymbol, proto::LineSymbol as GrpcLineSymbol};

impl From<LineSymbol> for GrpcLineSymbol {
    fn from(symbol: LineSymbol) -> Self {
        Self {
            symbol: symbol.symbol,
            color: symbol.color,
            shape: symbol.shape,
        }
    }
}
