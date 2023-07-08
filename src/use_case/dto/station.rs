use crate::{
    domain::entity::station::Station,
    pb::{Station as GrpcStation, StopCondition},
};

impl From<Station> for GrpcStation {
    fn from(station: Station) -> Self {
        let stop_condition: i32 = match station.pass {
            0 => StopCondition::All.into(),
            1 => StopCondition::Not.into(),
            2 => StopCondition::Partial.into(),
            3 => StopCondition::Weekday.into(),
            4 => StopCondition::Holiday.into(),
            5 => StopCondition::PartialStop.into(),
            _ => StopCondition::All.into(),
        };

        Self {
            id: station.station_cd,
            group_id: station.station_g_cd,
            name: station.station_name,
            name_katakana: station.station_name_k,
            name_roman: station.station_name_r,
            name_chinese: station.station_name_zh,
            name_korean: station.station_name_ko,
            three_letter_code: station.three_letter_code,
            lines: station.lines.into_iter().map(|line| line.into()).collect(),
            line: station.line.map(|line| Box::new((*line).into())),
            prefecture_id: station.pref_cd,
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
            stop_condition,
            distance: station.distance,
            pass: station.pass == 1,
            has_train_types: station.station_types_count > 0,
        }
    }
}
