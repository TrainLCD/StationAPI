use crate::{
    domain::entity::{gtfs::TransportType, line::Line},
    proto::{Line as GrpcLine, TransportType as GrpcTransportType},
};

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
            line_type: line.line_type.unwrap_or_default(),
            line_symbols: line.line_symbols.into_iter().map(|s| s.into()).collect(),
            status: line.e_status,
            station: line.station.map(|s| Box::new(s.into())),
            company: line.company.map(|c| c.into()),
            train_type: line
                .train_type
                .map(|train_type| Box::new(train_type.into())),
            average_distance: line.average_distance.unwrap_or(0.0),
            transport_type: convert_transport_type(line.transport_type),
        }
    }
}

fn convert_transport_type(t: TransportType) -> i32 {
    match t {
        TransportType::Rail => GrpcTransportType::Rail as i32,
        TransportType::Bus => GrpcTransportType::Bus as i32,
    }
}
