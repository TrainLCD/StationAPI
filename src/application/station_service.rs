use anyhow::Result;

use crate::{
    domain::models::station::{station_model::Station, station_repository::StationRepository},
    pb::StationResponse,
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
            line_id: value.line_cd,
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
        limit: Option<i32>,
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
    pub async fn get_stations_by_name(&self, name: &str) -> Result<Vec<Station>> {
        match self.station_repository.find_by_name(name).await {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the station. Provided Station Name: {:?}",
                name
            )),
        }
    }
}
