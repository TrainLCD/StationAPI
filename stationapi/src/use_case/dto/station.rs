use crate::{domain::entity::station::Station, proto::Station as GrpcStation};

impl From<Station> for GrpcStation {
    fn from(station: Station) -> Self {
        Self {
            id: station.station_cd as u32,
            group_id: station.station_g_cd as u32,
            name: station.station_name,
            name_katakana: station.station_name_k,
            name_roman: station.station_name_r,
            name_chinese: station.station_name_zh,
            name_korean: station.station_name_ko,
            three_letter_code: station.three_letter_code,
            lines: station.lines.into_iter().map(|line| line.into()).collect(),
            line: station.line.map(|line| Box::new((*line).into())),
            prefecture_id: station.pref_cd as u32,
            postal_code: station.post,
            address: station.address,
            latitude: station.lat,
            longitude: station.lon,
            opened_at: station.open_ymd,
            closed_at: station.close_ymd,
            status: station.e_status as i32,
            station_numbers: station
                .station_numbers
                .into_iter()
                .map(|num| num.into())
                .collect(),
            stop_condition: station.stop_condition.into(),
            distance: station.distance,
            has_train_types: Some(station.has_train_types),
            train_type: station.train_type.map(|tt| Box::new((*tt).into())),
        }
    }
}
