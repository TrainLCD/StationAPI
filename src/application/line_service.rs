use std::vec;

use anyhow::Result;
use bigdecimal::Zero;
use futures::stream;

use crate::{
    domain::models::{
        line::{line_model::Line, line_repository::LineRepository},
        station::{station_model::Station, station_repository::StationRepository},
    },
    pb::{LineResponse, LineSymbol, SingleStationResponse, StationResponse},
};
use futures::stream::StreamExt;

#[derive(Debug)]
pub struct LineService<LR, SR>
where
    LR: LineRepository,
    SR: StationRepository,
{
    line_repository: LR,
    station_repository: SR,
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

impl<LR: LineRepository, SR: StationRepository> LineService<LR, SR> {
    pub fn new(line_repository: LR, station_repository: SR) -> Self {
        Self {
            line_repository,
            station_repository,
        }
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

    pub fn get_line_symbols(&self, line: Line) -> Vec<LineSymbol> {
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
                shape: line.line_symbol_extra_shape.unwrap_or("".to_string()),
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
            let lines = self.get_by_station_group_id(station_group_id).await?;
            stations_lines.push(lines);
        }

        Ok(stations_lines)
    }

    async fn get_stations_by_group_id(&self, group_id: u32) -> Result<Vec<Station>> {
        let stations = self.station_repository.get_by_group_id(group_id).await?;
        Ok(stations)
    }

    async fn process_stations_async(
        &self,
        station: &Station,
        station_lines: &[Line],
    ) -> SingleStationResponse {
        let mut station_response: StationResponse = station.clone().into();

        let line = self.find_by_station_id(station.station_cd).await.unwrap();
        let mut line_response: LineResponse = line.clone().into();
        let line_symbols = self.get_line_symbols(line);
        line_response.line_symbols = line_symbols;
        station_response.line = Some(Box::new(line_response));

        let same_group_stations: Vec<Station> = self
            .get_stations_by_group_id(station.station_g_cd)
            .await
            .unwrap();

        let station_lines_response = station_lines
            .iter()
            .map(|station_line| {
                let line_symbols = self.get_line_symbols(station_line.clone());
                let mut resp: LineResponse = station_line.clone().into();
                resp.line_symbols = line_symbols;

                let transfer_station_response: StationResponse = same_group_stations
                    .iter()
                    .find(|&group_station| group_station.line_cd == station_line.line_cd)
                    .unwrap()
                    .clone()
                    .into();
                resp.station = Some(Box::new(transfer_station_response));

                resp
            })
            .collect::<Vec<LineResponse>>();
        station_response.lines = station_lines_response;

        SingleStationResponse {
            station: Some(station_response),
        }
    }

    pub async fn get_response_stations_from_stations(
        &self,
        stations: &[Station],
        stations_lines: &[Vec<Line>],
    ) -> Result<Vec<SingleStationResponse>> {
        let response_stations: Vec<SingleStationResponse> = stream::iter(stations)
            .enumerate()
            .then(|(index, station)| {
                self.process_stations_async(station, stations_lines.get(index).unwrap())
            })
            .collect()
            .await;
        Ok(response_stations)
    }
}
