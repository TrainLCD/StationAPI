use anyhow::Result;
use async_trait::async_trait;
use futures::future::try_join_all;

use crate::{
    domain::{
        entity::{line::Line, station::Station},
        repository::{line_repository::LineRepository, station_repository::StationRepository},
    },
    pb::StationNumber,
    use_case::{error::UseCaseError, traits::query::QueryUseCase},
};

#[derive(Debug, Clone)]
pub struct QueryInteractor<SR, LR> {
    pub station_repository: SR,
    pub line_repository: LR,
}

#[async_trait]
impl<SR, LR> QueryUseCase for QueryInteractor<SR, LR>
where
    SR: StationRepository,
    LR: LineRepository,
{
    async fn find_station_by_id(&self, station_id: u32) -> Result<Option<Station>, UseCaseError> {
        if let Ok(Some(station)) = self.station_repository.find_by_id(station_id).await {
            let station = self.get_station_with_attributes(station).await?;
            return Ok(Some(station));
        }

        Ok(None)
    }

    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_station_group_id(station_group_id)
            .await?;
        let stations = self.get_stations_with_attributes(stations).await?;
        Ok(stations)
    }
    async fn get_stations_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_stations_by_coordinates(latitude, longitude, limit)
            .await?;

        let stations = self.get_stations_with_attributes(stations).await?;

        Ok(stations)
    }
    async fn get_stations_by_line_id(&self, line_id: u32) -> Result<Vec<Station>, UseCaseError> {
        let stations = self.station_repository.get_by_line_id(line_id).await?;
        let stations = self.get_stations_with_attributes(stations).await?;
        Ok(stations)
    }
    async fn get_stations_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_stations_by_name(station_name, limit)
            .await?;
        let stations = self.get_stations_with_attributes(stations).await?;
        Ok(stations)
    }
    async fn find_line_by_id(&self, line_id: u32) -> Result<Option<Line>, UseCaseError> {
        let line = self.line_repository.find_by_id(line_id).await?;
        Ok(line)
    }

    async fn get_station_with_attributes(
        &self,
        mut station: Station,
    ) -> Result<Station, UseCaseError> {
        let belong_lines = match self.find_line_by_id(station.line_cd).await {
            Ok(line) => line,
            Err(err) => return Err(UseCaseError::Unexpected(err.to_string())),
        };

        let lines = self
            .get_lines_by_station_group_id(station.station_g_cd)
            .await?;

        let belong_line = belong_lines
            .clone()
            .into_iter()
            .find(|line| station.line_cd == line.line_cd);
        station.set_line(belong_line.clone());
        station.set_lines(lines);

        if let Some(belong_line) = belong_line {
            let station_numbers = self.get_station_numbers(station.clone(), belong_line);
            station.set_station_numbers(station_numbers);
        }

        Ok(station)
    }

    async fn get_lines_by_ids(&self, line_ids: Vec<u32>) -> Result<Vec<Line>, UseCaseError> {
        let lines = self.line_repository.get_by_ids(line_ids).await?;
        Ok(lines)
    }

    async fn get_lines_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Line>, UseCaseError> {
        let lines = self
            .line_repository
            .get_by_station_group_id(station_group_id)
            .await?;
        Ok(lines)
    }

    async fn get_stations_with_attributes(
        &self,
        stations: Vec<Station>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let line_ids: Vec<u32> = stations.iter().map(|station| station.line_cd).collect();

        let belong_lines = match self.get_lines_by_ids(line_ids).await {
            Ok(lines) => lines,
            Err(err) => return Err(UseCaseError::Unexpected(err.to_string())),
        };

        let get_lines_futures = stations
            .iter()
            .map(|station| self.get_lines_by_station_group_id(station.station_g_cd));
        let lines = try_join_all(get_lines_futures).await?;

        let stations = stations
            .into_iter()
            .enumerate()
            .map(|(index, mut station)| {
                let belong_line = belong_lines
                    .clone()
                    .into_iter()
                    .find(|line| station.line_cd == line.line_cd);
                station.set_line(belong_line.clone());
                if let Some(lines) = lines.get(index).cloned() {
                    station.set_lines(lines);
                }

                if let Some(belong_line) = belong_line {
                    let station_numbers = self.get_station_numbers(station.clone(), belong_line);
                    station.set_station_numbers(station_numbers);
                }

                station
            })
            .collect();

        Ok(stations)
    }

    fn get_station_numbers(&self, station: Station, line: Line) -> Vec<StationNumber> {
        let cloned_station_line = line.clone();
        let line_symbols_raw: Vec<Option<String>> = vec![
            cloned_station_line.line_symbol_primary,
            cloned_station_line.line_symbol_secondary,
            cloned_station_line.line_symbol_extra,
        ];

        let line_color = &line.line_color_c;
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

        let cloned_station = station;
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

        let mut station_numbers: Vec<StationNumber> = Vec::with_capacity(station_numbers_raw.len());

        for index in 0..station_numbers_raw.len() {
            let Some(num) = station_numbers_raw.get(index) else {
                break;
            };
            let Some(num) = num else {
                break;
            };
            if num.is_empty() {
                break;
            }

            let Some(sym_color) = &line_symbol_colors_raw[index] else {
                break;
            };
            let Some(sym_shape) = &line_symbols_shape_raw[index] else {
                break;
            };

            let opt_sym = &line_symbols_raw[index];
            let sym = opt_sym.to_owned().unwrap_or(String::from(""));

            let station_number_string = match sym.is_empty() {
                true => num.clone(),
                false => format!("{}-{}", sym, num),
            };

            let station_number = StationNumber {
                line_symbol: sym,
                line_symbol_color: sym_color.to_string(),
                line_symbol_shape: sym_shape.to_string(),
                station_number: station_number_string,
            };

            station_numbers.push(station_number);
        }

        station_numbers
    }
}
