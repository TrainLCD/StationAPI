use crate::service::LineResponse;
use bigdecimal::BigDecimal;

#[derive(sqlx::FromRow, Clone)]
pub struct Line {
    pub line_cd: u32,
    pub company_cd: u32,
    pub line_name: String,
    pub line_name_k: String,
    pub line_name_h: String,
    pub line_name_r: String,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: String,
    pub line_color_t: String,
    pub line_type: u32,
    pub line_symbol_primary: Option<String>,
    pub line_symbol_secondary: Option<String>,
    pub line_symbol_extra: Option<String>,
    pub line_symbol_primary_color: Option<String>,
    pub line_symbol_secondary_color: Option<String>,
    pub line_symbol_extra_color: Option<String>,
    pub line_symbol_primary_shape: Option<String>,
    pub line_symbol_secondary_shape: Option<String>,
    pub line_symbol_extra_shape: Option<String>,
    pub lon: BigDecimal,
    pub lat: BigDecimal,
    pub zoom: u32,
    pub e_status: u32,
    pub e_sort: u32,
}

impl From<Line> for LineResponse {
    fn from(value: Line) -> Self {
        LineResponse {
            id: value.line_cd,
            name_short: value.line_name,
            name_katakana: value.line_name_k,
            name_full: value.line_name_h,
            name_roman: value.line_name_r,
            name_chinese: value.line_name_zh.unwrap_or("".to_string()),
            name_korean: value.line_name_ko.unwrap_or("".to_string()),
            color: value.line_color_c,
            line_type: value.line_type as i32,
            company: None,
            line_symbols: vec![],
            status: value.e_status as i32,
            station: None,
        }
    }
}
