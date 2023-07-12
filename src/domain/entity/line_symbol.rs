use fake::Dummy;

#[derive(Dummy, Clone, Debug)]
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
