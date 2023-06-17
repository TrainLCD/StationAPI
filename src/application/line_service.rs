use std::vec;

use anyhow::Result;
use bigdecimal::Zero;

use crate::{
    domain::models::{
        line::{line_model::Line, line_repository::LineRepository},
        station::station_model::Station,
    },
    pb::{LineResponse, LineSymbol, SingleStationResponse, StationResponse},
};

#[derive(Debug)]
pub struct LineService<T>
where
    T: LineRepository,
{
    line_repository: T,
}

impl From<Line> for LineResponse {
    fn from(value: Line) -> Self {
        Self {
            id: value.line_cd,
            name_short: value.line_name,
            name_katakana: value.line_name_k,
            name_full: value.line_name_h,
            name_roman: value.line_name_r,
            name_chinese: value.line_name_zh.unwrap_or("".to_string()),
            name_korean: value.line_name_ko.unwrap_or("".to_string()),
            color: value.line_color_c,
            line_type: value.line_type as i32,
            company_id: value.company_cd,
            line_symbols: vec![],
            status: value.e_status as i32,
            station: None,
        }
    }
}

impl<T: LineRepository> LineService<T> {
    pub fn new(line_repository: T) -> Self {
        Self { line_repository }
    }
    pub async fn find_by_id(&self, id: u32) -> Result<Line> {
        match self.line_repository.find_by_id(id).await {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the line. Provided ID: {:?}",
                id
            )),
        }
    }
    pub async fn get_by_station_group_id(&self, station_group_id: u32) -> Result<Vec<Line>> {
        match self
            .line_repository
            .get_by_station_group_id(station_group_id)
            .await
        {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the line. Provided Group ID: {:?}",
                station_group_id
            )),
        }
    }

    pub async fn find_by_station_id(&self, station_id: u32) -> Result<Line> {
        match self.line_repository.find_by_station_id(station_id).await {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the line. Provided Station ID: {:?}",
                station_id
            )),
        }
    }

    pub fn get_line_symbols(&self, line: &mut Line) -> Vec<LineSymbol> {
        let mut line_symbols = vec![];

        if !line
            .line_symbol_primary
            .clone()
            .unwrap_or("".to_string())
            .len()
            .is_zero()
        {
            let line_symbol = LineSymbol {
                symbol: line.clone().line_symbol_primary.unwrap_or("".to_string()),
                color: line
                    .clone()
                    .line_symbol_primary_color
                    .unwrap_or("".to_string()),
                shape: line
                    .clone()
                    .line_symbol_primary_shape
                    .unwrap_or("".to_string()),
            };
            line_symbols.push(line_symbol);
        }

        if !line
            .line_symbol_secondary
            .clone()
            .unwrap_or("".to_string())
            .len()
            .is_zero()
        {
            let line_symbol = LineSymbol {
                symbol: line.clone().line_symbol_secondary.unwrap_or("".to_string()),
                color: line
                    .clone()
                    .line_symbol_secondary_color
                    .unwrap_or("".to_string()),
                shape: line
                    .clone()
                    .line_symbol_secondary_shape
                    .unwrap_or("".to_string()),
            };
            line_symbols.push(line_symbol);
        }

        if !line
            .line_symbol_extra
            .clone()
            .unwrap_or("".to_string())
            .len()
            .is_zero()
        {
            let line_symbol = LineSymbol {
                symbol: line.clone().line_symbol_extra.unwrap_or("".to_string()),
                color: line
                    .clone()
                    .line_symbol_extra_color
                    .unwrap_or("".to_string()),
                shape: line
                    .clone()
                    .line_symbol_extra_shape
                    .unwrap_or("".to_string()),
            };
            line_symbols.push(line_symbol);
        }
        line_symbols
    }

    pub async fn get_stations_lines(&self, stations: &[Station]) -> Result<Vec<Vec<Line>>> {
        let station_group_ids: Vec<u32> = stations
            .iter()
            .map(|station| station.station_g_cd)
            .collect();

        let mut stations_lines = vec![];
        for station_group_id in station_group_ids {
            let line = self.get_by_station_group_id(station_group_id).await?;
            stations_lines.push(line);
        }

        Ok(stations_lines)
    }

    pub async fn get_response_stations_from_stations(
        &self,
        stations: &[Station],
        stations_lines: &[Vec<Line>],
    ) -> Result<Vec<SingleStationResponse>> {
        let response_stations: Vec<SingleStationResponse> = stations
            .iter()
            .enumerate()
            .map(|(index, station)| {
                let mut station_response: StationResponse = station.clone().into();
                let station_lines = stations_lines.get(index).unwrap();
                let station_line = station_lines
                    .iter()
                    .find(|line| line.line_cd == station.line_cd)
                    .unwrap();

                let station_lines_response: Vec<LineResponse> = station_lines
                    .iter()
                    .map(|line| {
                        let mut line_response: LineResponse = line.clone().into();
                        let line_symbols = self.get_line_symbols(&mut line.clone());
                        line_response.line_symbols = line_symbols;
                        line_response.station = Some(Box::new(station_response.clone()));
                        line_response
                    })
                    .collect();

                let mut line_response: LineResponse = station_line.clone().into();

                let line_symbols = self.get_line_symbols(&mut station_line.clone());
                line_response.line_symbols = line_symbols;

                station_response.line = Some(Box::new(line_response));
                station_response.lines = station_lines_response;

                SingleStationResponse {
                    station: Some(station_response),
                }
            })
            .collect();

        Ok(response_stations)
    }
}
