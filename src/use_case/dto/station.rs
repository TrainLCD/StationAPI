use crate::{domain::entity::station::Station, pb::StationResponse};

impl From<Station> for StationResponse {
    fn from(value: Station) -> Self {
        Self {
            id: value.station_cd,
            group_id: value.station_g_cd,
            name: value.station_name,
            name_katakana: value.station_name_k,
            name_roman: value.station_name_r,
            name_chinese: value.station_name_zh,
            name_korean: value.station_name_ko,
            three_letter_code: value.three_letter_code,
            line: None,
            lines: vec![],
            prefecture_id: value.pref_cd,
            postal_code: value.post,
            address: value.address,
            latitude: value.lat,
            longitude: value.lon,
            opened_at: value.open_ymd,
            closed_at: value.close_ymd,
            status: value.e_status as i32,
            station_numbers: vec![],
            stop_condition: 0,
            distance: value.distance,
        }
    }
}
