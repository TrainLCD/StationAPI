use crate::{domain::entity::line::Line, pb::LineResponse};

impl From<Line> for LineResponse {
    fn from(value: Line) -> Self {
        Self {
            id: value.line_cd,
            name_short: value.line_name,
            name_katakana: value.line_name_k,
            name_full: value.line_name_h,
            name_roman: value.line_name_r,
            name_chinese: value.line_name_zh.unwrap_or("".to_string()),
            name_korean: value.line_name_ko.unwrap_or("".to_string()),
            color: value.line_color_c,
            line_type: value.line_type as i32,
            company_id: value.company_cd,
            line_symbols: vec![],
            status: value.e_status as i32,
            station: None,
        }
    }
}
