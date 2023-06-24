use bigdecimal::{BigDecimal, ToPrimitive};
use getset::{Getters, Setters};

use crate::pb::StationNumber;

use super::line::Line;

#[derive(Debug, Clone, PartialEq, Getters, Setters)]
pub struct Station {
    #[getset(get = "pub")]
    pub station_cd: u32,
    #[getset(get = "pub")]
    pub station_g_cd: u32,
    #[getset(get = "pub")]
    pub station_name: String,
    #[getset(get = "pub")]
    pub station_name_k: String,
    #[getset(get = "pub")]
    pub station_name_r: String,
    #[getset(get = "pub")]
    pub station_name_zh: String,
    #[getset(get = "pub")]
    pub station_name_ko: String,
    #[getset(get = "pub")]
    pub station_numbers: Vec<StationNumber>,
    #[getset(get = "pub")]
    pub three_letter_code: Option<String>,
    #[getset(get = "pub", set = "pub")]
    pub line: Option<Line>,
    #[getset(get = "pub", set = "pub")]
    pub lines: Vec<Line>,
    #[getset(get = "pub")]
    pub pref_cd: u32,
    #[getset(get = "pub")]
    pub post: String,
    #[getset(get = "pub")]
    pub address: String,
    #[getset(get = "pub")]
    pub lon: f64,
    #[getset(get = "pub")]
    pub lat: f64,
    pub open_ymd: String,
    #[getset(get = "pub")]
    pub close_ymd: String,
    #[getset(get = "pub")]
    pub e_status: u32,
    #[getset(get = "pub")]
    pub e_sort: u32,
    #[getset(get = "pub")]
    pub distance: Option<f64>,
}

impl Station {
    pub fn new(
        station_cd: u32,
        station_g_cd: u32,
        station_name: String,
        station_name_k: String,
        station_name_r: String,
        station_name_zh: String,
        station_name_ko: String,
        station_numbers: Vec<StationNumber>,
        three_letter_code: Option<String>,
        line: Option<Line>,
        lines: Vec<Line>,
        pref_cd: u32,
        post: String,
        address: String,
        lon: BigDecimal,
        lat: BigDecimal,
        open_ymd: String,
        close_ymd: String,
        e_status: u32,
        e_sort: u32,
        distance: Option<f64>,
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
            three_letter_code,
            line,
            lines,
            pref_cd,
            post,
            address,
            lon: lon.to_f64().unwrap_or(0.0),
            lat: lat.to_f64().unwrap_or(0.0),
            open_ymd,
            close_ymd,
            e_status,
            e_sort,
            distance,
        }
    }
}
