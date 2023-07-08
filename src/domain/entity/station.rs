use bigdecimal::num_bigint::BigInt;
use fake::Dummy;

use super::{line::Line, station_number::StationNumber};

#[derive(Dummy, Clone, Debug)]
pub struct Station {
    pub station_cd: u32,
    pub station_g_cd: u32,
    pub station_name: String,
    pub station_name_k: String,
    pub station_name_r: String,
    pub station_name_zh: String,
    pub station_name_ko: String,
    pub station_numbers: Vec<StationNumber>,
    pub primary_station_number: Option<String>,
    pub secondary_station_number: Option<String>,
    pub extra_station_number: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: u32,
    pub line: Option<Box<Line>>,
    pub lines: Vec<Line>,
    pub pref_cd: u32,
    pub post: String,
    pub address: String,
    pub lon: f64,
    pub lat: f64,
    pub open_ymd: String,
    pub close_ymd: String,
    pub e_status: u32,
    pub e_sort: u32,
    pub pass: bool,
    pub distance: Option<f64>,
    pub station_types_count: BigInt,
}

impl Station {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        station_cd: u32,
        station_g_cd: u32,
        station_name: String,
        station_name_k: String,
        station_name_r: String,
        station_name_zh: String,
        station_name_ko: String,
        station_numbers: Vec<StationNumber>,
        primary_station_number: Option<String>,
        secondary_station_number: Option<String>,
        extra_station_number: Option<String>,
        three_letter_code: Option<String>,
        line_cd: u32,
        line: Option<Box<Line>>,
        lines: Vec<Line>,
        pref_cd: u32,
        post: String,
        address: String,
        lon: f64,
        lat: f64,
        open_ymd: String,
        close_ymd: String,
        e_status: u32,
        e_sort: u32,
        _stop_condition: i32,
        pass: bool,
        distance: Option<f64>,
        station_types_count: BigInt,
    ) -> Self {
        Self {
            station_cd,
            station_g_cd,
            station_name,
            station_name_k,
            station_name_r,
            station_name_zh,
            station_name_ko,
            station_numbers,
            primary_station_number,
            secondary_station_number,
            extra_station_number,
            three_letter_code,
            line_cd,
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
            e_sort,
            pass,
            distance,
            station_types_count,
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use fake::{Fake, Faker};

//     use crate::domain::entity::station::Station;

//     #[test]
//     fn new() {
//         expected
//         let Station {
//             station_cd,
//             station_g_cd,
//             station_name,
//             station_name_k,
//             station_name_r,
//             station_name_zh,
//             station_name_ko,
//             station_numbers: _,
//             primary_station_number,
//             secondary_station_number,
//             extra_station_number,
//             three_letter_code,
//             line_cd,
//             line: _,
//             lines: _,
//             pref_cd,
//             post,
//             address,
//             lon,
//             lat,
//             open_ymd,
//             close_ymd,
//             e_status,
//             e_sort,
//             stop_condition,
//             distance,
//         } = Faker.fake();

//         actual
//         let actual_station = Station::new(
//             station_cd,
//             station_g_cd,
//             station_name.clone(),
//             station_name_k.clone(),
//             station_name_r.clone(),
//             station_name_zh.clone(),
//             station_name_ko.clone(),
//             vec![],
//             primary_station_number.clone(),
//             secondary_station_number.clone(),
//             extra_station_number.clone(),
//             three_letter_code.clone(),
//             line_cd,
//             None,
//             vec![],
//             pref_cd,
//             post.clone(),
//             address.clone(),
//             lon,
//             lat,
//             open_ymd.clone(),
//             close_ymd.clone(),
//             e_status,
//             e_sort,
//             stop_condition,
//             distance,
//         );
//         assert_eq!(actual_station.station_cd, station_cd);
//         assert_eq!(actual_station.station_g_cd, station_g_cd);
//         assert_eq!(actual_station.station_name, station_name);
//         assert_eq!(actual_station.station_name_k, station_name_k);
//         assert_eq!(actual_station.station_name_r, station_name_r);
//         assert_eq!(actual_station.station_name_zh, station_name_zh);
//         assert_eq!(actual_station.station_name_ko, station_name_ko);
//         assert_eq!(actual_station.station_numbers, station_numbers);
//         assert_eq!(
//             actual_station.primary_station_number,
//             primary_station_number
//         );
//         assert_eq!(
//             actual_station.secondary_station_number,
//             secondary_station_number
//         );
//         assert_eq!(actual_station.extra_station_number, extra_station_number);
//         assert_eq!(actual_station.three_letter_code, three_letter_code);
//         assert_eq!(actual_station.line_cd, line_cd);
//         assert_eq!(actual_station.line, line);
//         assert_eq!(actual_station.lines, lines);
//         assert_eq!(actual_station.pref_cd, pref_cd);
//         assert_eq!(actual_station.post, post);
//         assert_eq!(actual_station.address, address);
//         assert_eq!(actual_station.lon, lon);
//         assert_eq!(actual_station.lat, lat);
//         assert_eq!(actual_station.open_ymd, open_ymd);
//         assert_eq!(actual_station.close_ymd, close_ymd);
//         assert_eq!(actual_station.e_status, e_status);
//         assert_eq!(actual_station.e_sort, e_sort);
//         assert_eq!(actual_station.stop_condition, stop_condition);
//         assert_eq!(actual_station.distance, distance);
//     }
// }
