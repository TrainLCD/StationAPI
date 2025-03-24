use crate::{domain::entity::line::Line, proto::Line as GrpcLine};

impl From<Line> for GrpcLine {
    fn from(line: Line) -> Self {
        Self {
            id: line.line_cd as u32,
            name_short: line.line_name,
            name_katakana: line.line_name_k,
            name_full: line.line_name_h,
            name_roman: Some(line.line_name_r.unwrap_or_default()),
            name_chinese: line.line_name_zh,
            name_korean: line.line_name_ko,
            color: line.line_color_c.unwrap_or_default(),
            line_type: line.line_type.unwrap_or_default() as i32,
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
