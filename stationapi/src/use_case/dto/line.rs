use crate::{domain::entity::line::Line, station_api::Line as GrpcLine};

impl From<Line> for GrpcLine {
    fn from(line: Line) -> Self {
        Self {
            id: line.line_cd,
            name_short: line.line_name.unwrap_or("".to_string()),
            name_katakana: line.line_name_k.unwrap_or("".to_string()),
            name_full: line.line_name_h.unwrap_or("".to_string()),
            name_roman: line.line_name_r,
            name_chinese: line.line_name_zh,
            name_korean: line.line_name_ko,
            color: line.line_color_c.unwrap_or("".to_string()),
            line_type: line.line_type.unwrap_or(0) as i32,
            line_symbols: line.line_symbols.into_iter().map(|s| s.into()).collect(),
            status: line.e_status as i32,
            station: line.station.map(|s| Box::new(s.into())),
            company: line.company.map(|c| c.into()),
            train_type: line
                .train_type
                .map(|train_type| Box::new(train_type.into())),
            average_distance: line.average_distance,
        }
    }
}
