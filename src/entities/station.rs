use bigdecimal::BigDecimal;

use crate::service::StationResponse;

#[derive(sqlx::FromRow, Clone)]
pub struct Station {
    pub station_cd: u32,
    pub station_g_cd: u32,
    pub station_name: String,
    pub station_name_k: String,
    pub station_name_r: String,
    pub station_name_zh: String,
    pub station_name_ko: String,
    pub primary_station_number: Option<String>,
    pub secondary_station_number: Option<String>,
    pub extra_station_number: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: u32,
    pub pref_cd: u32,
    pub post: String,
    pub address: String,
    pub lon: BigDecimal,
    pub lat: BigDecimal,
    pub open_ymd: String,
    pub close_ymd: String,
    pub e_status: u32,
    pub e_sort: u32,
    #[sqlx(default)]
    pub distance: Option<f64>,
}

impl From<Station> for StationResponse {
    fn from(value: Station) -> Self {
        StationResponse {
            id: value.station_cd,
            group_id: value.station_g_cd,
            name: value.station_name,
            name_katakana: value.station_name_k,
            name_roman: value.station_name_r,
            name_chinese: value.station_name_zh,
            name_korean: value.station_name_ko,
            primary_station_number: value.primary_station_number,
            secondary_station_number: value.secondary_station_number,
            extra_station_number: value.extra_station_number,
            three_letter_code: value.three_letter_code,
            lines: vec![],
            line: None,
            prefecture: value.pref_cd as i32,
            postal_code: value.post,
            address: value.address,
            latitude: serde_json::from_str(value.lat.to_string().as_str()).unwrap(),
            longitude: serde_json::from_str(value.lon.to_string().as_str()).unwrap(),
            opened_at: value.open_ymd,
            closed_at: value.close_ymd,
            status: value.e_status as i32,
            station_numbers: vec![],
            stop_condition: 0,
            distance: value.distance,
        }
    }
}
