use anyhow::Result;

use crate::{
    domain::models::{
        line::line_model::Line,
        station::{station_model::Station, station_repository::StationRepository},
    },
    pb::{StationNumber, StationResponse},
};

#[derive(Debug)]
pub struct StationService<T>
where
    T: StationRepository,
{
    station_repository: T,
}

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

impl<T: StationRepository> StationService<T> {
    pub fn new(station_repository: T) -> Self {
        Self { station_repository }
    }

    pub fn update_station_numbers(
        &self,
        station_response_ref: &mut StationResponse,
        station: &Station,
        station_line: &Line,
    ) {
        let cloned_station_line = station_line.clone();
        let line_symbols_raw: Vec<Option<String>> = vec![
            cloned_station_line.line_symbol_primary,
            cloned_station_line.line_symbol_secondary,
            cloned_station_line.line_symbol_extra,
        ];

        let line_color = &station_line.line_color_c;
        let line_symbol_colors_raw: Vec<Option<String>> = vec![
            cloned_station_line
                .line_symbol_primary_color
                .or(Some(line_color.to_string())),
            cloned_station_line
                .line_symbol_secondary_color
                .or(Some(line_color.to_string())),
            cloned_station_line
                .line_symbol_extra_color
                .or(Some(line_color.to_string())),
        ];

        let cloned_station = station.clone();
        let station_numbers_raw: Vec<Option<String>> = vec![
            cloned_station.primary_station_number,
            cloned_station.secondary_station_number,
            cloned_station.extra_station_number,
        ];

        let line_symbols_shape_raw: Vec<Option<String>> = vec![
            cloned_station_line.line_symbol_primary_shape,
            cloned_station_line.line_symbol_secondary_shape,
            cloned_station_line.line_symbol_extra_shape,
        ];

        let station_numbers: Vec<Option<StationNumber>> = station_numbers_raw
            .into_iter()
            .enumerate()
            .map(|(index, opt_num)| {
                let Some(num) = opt_num else {
                    return None;
                };
                if num.is_empty() {
                    return None;
                }

                let Some(sym_color) = &line_symbol_colors_raw[index] else {
                    return None;
                };
                let Some(sym_shape) = &line_symbols_shape_raw[index] else {
                    return None;
                };

                let opt_sym = &line_symbols_raw[index];
                let sym = opt_sym.to_owned().unwrap_or(String::from(""));

                let station_number_string = match sym.is_empty() {
                    true => num,
                    false => format!("{}-{}", sym, num),
                };

                let station_number = StationNumber {
                    line_symbol: sym,
                    line_symbol_color: sym_color.to_string(),
                    line_symbol_shape: sym_shape.to_string(),
                    station_number: station_number_string,
                };

                Some(station_number)
            })
            .filter(|num| num.is_some())
            .collect();
        station_response_ref.station_numbers = station_numbers
            .into_iter()
            .map(|num| num.unwrap())
            .collect();
    }

    pub async fn find_by_id(&self, id: u32) -> Result<Station> {
        match self.station_repository.find_by_id(id).await {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the station. Provided ID: {:?}",
                id
            )),
        }
    }
    pub async fn get_by_group_id(&self, group_id: u32) -> Result<Vec<Station>> {
        match self.station_repository.find_by_group_id(group_id).await {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the station. Provided Group ID: {:?}",
                group_id
            )),
        }
    }
    pub async fn get_station_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: &Option<u32>,
    ) -> Result<Vec<Station>> {
        match self
            .station_repository
            .find_by_coordinates(latitude, longitude, limit)
            .await
        {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the station. Provided Latitude: {:?}, Longitude: {:?}",
                latitude,
                longitude
            )),
        }
    }
    pub async fn get_stations_by_line_id(&self, line_id: u32) -> Result<Vec<Station>> {
        match self.station_repository.find_by_line_id(line_id).await {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the station. Provided Line ID: {:?}",
                line_id
            )),
        }
    }
    pub async fn get_stations_by_name(
        &self,
        name: &str,
        limit: &Option<u32>,
    ) -> Result<Vec<Station>> {
        match self.station_repository.find_by_name(name, limit).await {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the station. Provided Station Name: {:?}",
                name
            )),
        }
    }
}
