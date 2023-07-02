use anyhow::Result;
use async_trait::async_trait;
use bigdecimal::Zero;
use futures::future::try_join_all;

use crate::{
    domain::{
        entity::{line::Line, station::Station},
        repository::{line_repository::LineRepository, station_repository::StationRepository},
    },
    pb::{LineSymbol, Station as GrpcStation, StationNumber},
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

        let lines: Vec<Line> = self
            .get_lines_by_station_group_id(station.station_g_cd)
            .await?;

        let line_ids: Vec<u32> = lines.iter().map(|line| line.line_cd).collect();

        let stations = self
            .station_repository
            .get_by_station_group_id(station.station_g_cd)
            .await?;
        let belong_station_vec = self.get_stations_with_attributes(stations).await?;

        let belong_station: Option<GrpcStation> = belong_station_vec
            .into_iter()
            .enumerate()
            .find_map(|(index, station)| {
                if line_ids.get(index) == Some(&station.line_cd) {
                    Some(station.into())
                } else {
                    None
                }
            });
        let lines = lines
            .into_iter()
            .map(|line| {
                let mut line = line;
                line.set_station(belong_station.clone());
                line.set_line_symbols(self.get_line_symbols(line.clone()));
                line
            })
            .collect();

        let belong_line = belong_lines
            .clone()
            .into_iter()
            .find(|line| station.line_cd == line.line_cd);
        station.set_lines(lines);

        let Some(mut belong_line) = belong_line else {
            return Err(UseCaseError::Unexpected(
                "station does not belong to any line".to_string(),
            ));
        };

        let line_symbols = self.get_line_symbols(belong_line.clone());
        belong_line.set_line_symbols(line_symbols);
        let station_numbers = self.get_station_numbers(station.clone(), belong_line.clone());
        station.set_station_numbers(station_numbers);
        station.set_line(Some(belong_line));

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

        let belong_lines = match self.get_lines_by_ids(line_ids.clone()).await {
            Ok(lines) => lines,
            Err(err) => return Err(UseCaseError::Unexpected(err.to_string())),
        };

        let belong_lines: Vec<Line> = belong_lines
            .into_iter()
            .map(|line| {
                let mut line = line;
                line.set_line_symbols(self.get_line_symbols(line.clone()));
                line
            })
            .collect();

        let get_lines_futures = stations
            .iter()
            .map(|station| self.get_lines_by_station_group_id(station.station_g_cd));
        let lines = try_join_all(get_lines_futures).await?;

        let get_stations_futures = stations.iter().map(|station| {
            self.station_repository
                .get_by_station_group_id(station.station_g_cd)
        });
        let belong_stations_vec = try_join_all(get_stations_futures).await?;

        let stations = stations
            .into_iter()
            .enumerate()
            .map(|(index, mut station)| {
                let Some(belong_stations) = belong_stations_vec.get(index).cloned() else {
                    return station;
                };
                let belong_line = belong_lines
                    .clone()
                    .into_iter()
                    .find(|line| station.line_cd == line.line_cd);

                if let Some(lines) = lines.get(index).cloned() {
                    let lines = lines
                        .into_iter()
                        .enumerate()
                        .map(|(index, line)| {
                            let mut line = line;
                            let mut belong_stations = belong_stations.clone();
                            let station_numbers =
                                self.get_station_numbers(station.clone(), line.clone());
                            station.set_station_numbers(station_numbers);

                            line.set_line_symbols(self.get_line_symbols(line.clone()));
                            let Some(station) = belong_stations.get_mut(index) else {
                                return line;
                            };

                            let station_numbers =
                                self.get_station_numbers(station.clone(), line.clone());
                            station.set_station_numbers(station_numbers);

                            line.set_station(Some(station.clone().into()));
                            line
                        })
                        .collect();
                    station.set_line(belong_line.clone());
                    station.set_lines(lines);
                }

                if let Some(belong_line) = belong_line {
                    let mut belong_line = belong_line;
                    belong_line.set_line_symbols(self.get_line_symbols(belong_line.clone()));

                    let station_numbers = self.get_station_numbers(station.clone(), belong_line);
                    station.set_station_numbers(station_numbers);
                }

                station
            })
            .collect();

        Ok(stations)
    }

    fn get_station_numbers(&self, station: Station, line: Line) -> Vec<StationNumber> {
        let line_symbols_raw = vec![
            line.line_symbol_primary,
            line.line_symbol_secondary,
            line.line_symbol_extra,
        ];

        let line_color = &line.line_color_c;
        let line_symbol_colors_raw: Vec<String> = vec![
            line.line_symbol_primary_color
                .unwrap_or(line_color.to_string()),
            line.line_symbol_secondary_color
                .unwrap_or(line_color.to_string()),
            line.line_symbol_extra_color
                .unwrap_or(line_color.to_string()),
        ];

        let cloned_station = station;
        let station_numbers_raw = vec![
            cloned_station.primary_station_number,
            cloned_station.secondary_station_number,
            cloned_station.extra_station_number,
        ];

        let line_symbols_shape_raw: Vec<Option<String>> = vec![
            line.line_symbol_primary_shape,
            line.line_symbol_secondary_shape,
            line.line_symbol_extra_shape,
        ];

        if station_numbers_raw.len().is_zero() {
            return vec![];
        }

        let mut station_numbers: Vec<StationNumber> = Vec::with_capacity(station_numbers_raw.len());

        (0..station_numbers_raw.len()).for_each(|index| {
            let Some(Some(num)) = station_numbers_raw.get(index) else {
                return;
            };
            let Some(sym_color) = line_symbol_colors_raw.get(index) else {
                return;
            };
            let Some(Some(sym_shape)) = line_symbols_shape_raw.get(index) else {
                return;
            };
            let Some(Some(sym)) = line_symbols_raw.get(index) else {
                return;
            };

            if sym.is_empty() || num.is_empty() {
                return;
            }

            let station_number_string = format!("{}-{}", sym, num);

            let station_number = StationNumber {
                line_symbol: sym.to_string(),
                line_symbol_color: sym_color.to_string(),
                line_symbol_shape: sym_shape.to_string(),
                station_number: station_number_string,
            };

            station_numbers.push(station_number);
        });

        station_numbers
    }

    fn get_line_symbols(&self, line: Line) -> Vec<LineSymbol> {
        let line_symbols_raw = vec![
            line.line_symbol_primary,
            line.line_symbol_secondary,
            line.line_symbol_extra,
        ];

        let line_color = &line.line_color_c;
        let line_symbol_colors_raw: Vec<String> = vec![
            line.line_symbol_primary_color
                .unwrap_or(line_color.to_string()),
            line.line_symbol_secondary_color
                .unwrap_or(line_color.to_string()),
            line.line_symbol_extra_color
                .unwrap_or(line_color.to_string()),
        ];

        let line_symbols_shape_raw = vec![
            line.line_symbol_primary_shape,
            line.line_symbol_secondary_shape,
            line.line_symbol_extra_shape,
        ];

        if line_symbols_raw.len().is_zero() {
            return vec![];
        }

        let mut line_symbols: Vec<LineSymbol> = Vec::with_capacity(line_symbols_raw.len());

        (0..line_symbols_raw.len()).for_each(|index| {
            let Some(symbol) = line_symbols_raw.get(index) else{
                return;
            };
            let Some(color) = line_symbol_colors_raw.get(index) else{
                return;
            };
            let Some(shape) = line_symbols_shape_raw.get(index) else{
                return;
            };

            let Some(symbol) = symbol else {return;};
            let Some(shape) = shape else {return;};
            if symbol.is_empty() {
                return;
            }
            if shape.is_empty() {
                return;
            }

            line_symbols.push(LineSymbol {
                symbol: symbol.to_string(),
                color: color.to_string(),
                shape: shape.to_string(),
            });
        });

        line_symbols
    }
}
