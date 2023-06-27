use crate::{
    domain::entity::station::Station,
    pb::{LineResponse, StationResponse},
};

impl From<Station> for StationResponse {
    fn from(station: Station) -> Self {
        let Station {
            station_cd,
            station_g_cd,
            station_name,
            station_name_k,
            station_name_r,
            station_name_zh,
            station_name_ko,
            station_numbers,
            primary_station_number: _,
            secondary_station_number: _,
            extra_station_number: _,
            three_letter_code,
            line_cd: _,
            line,
            lines,
            pref_cd,
            post,
            address,
            lon,
            lat,
            open_ymd,
            close_ymd,
            e_status,
            e_sort: _,
            stop_condition,
            distance,
        } = station;

        let default_station = Self {
            id: station_cd,
            group_id: station_g_cd,
            name: station_name.clone(),
            name_katakana: station_name_k.clone(),
            name_roman: station_name_r.clone(),
            name_chinese: station_name_zh.clone(),
            name_korean: station_name_ko.clone(),
            three_letter_code: three_letter_code.clone(),
            lines: vec![],
            line: None,
            prefecture_id: pref_cd,
            postal_code: post.clone(),
            address: address.clone(),
            latitude: lat,
            longitude: lon,
            opened_at: open_ymd.clone(),
            closed_at: close_ymd.clone(),
            status: e_status as i32,
            station_numbers: station_numbers.clone(),
            stop_condition,
            distance,
        };

        let lines = lines.into_iter().map(|l| l.into()).collect();
        let line = match line {
            Some(l) => l,
            None => return default_station,
        };
        let line: Option<Box<LineResponse>> = Some(Box::new(line.into()));

        Self {
            id: station_cd,
            group_id: station_g_cd,
            name: station_name,
            name_katakana: station_name_k,
            name_roman: station_name_r,
            name_chinese: station_name_zh,
            name_korean: station_name_ko,
            three_letter_code,
            lines,
            line,
            prefecture_id: pref_cd,
            postal_code: post,
            address,
            latitude: lat,
            longitude: lon,
            opened_at: open_ymd,
            closed_at: close_ymd,
            status: e_status as i32,
            station_numbers,
            stop_condition,
            distance,
        }
    }
}
