use std::vec;

use async_trait::async_trait;
use bigdecimal::Zero;

use crate::{
    domain::{
        entity::{
            line::Line, line_symbol::LineSymbol, station::Station, station_number::StationNumber,
        },
        repository::{line_repository::LineRepository, station_repository::StationRepository},
    },
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
        let Some(mut station) = self.station_repository.find_by_id(station_id).await? else {
            return Ok(None);
        };
        let station = &mut station;

        self.update_station_with_attributes(station).await?;
        Ok(Some(station.clone()))
    }

    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_station_group_id(station_group_id)
            .await?;

        let mut result: Vec<Station> = vec![];

        for mut station in stations.into_iter() {
            self.update_station_with_attributes(&mut station).await?;
            result.push(station);
        }

        Ok(result)
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

        let mut result: Vec<Station> = vec![];

        for mut station in stations.into_iter() {
            self.update_station_with_attributes(&mut station).await?;
            result.push(station);
        }

        Ok(result)
    }

    async fn get_stations_by_line_id(&self, line_id: u32) -> Result<Vec<Station>, UseCaseError> {
        let stations = self.station_repository.get_by_line_id(line_id).await?;
        let mut result: Vec<Station> = vec![];

        for mut station in stations.into_iter() {
            self.update_station_with_attributes(&mut station).await?;
            result.push(station);
        }

        Ok(result)
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
        let mut result: Vec<Station> = vec![];

        for mut station in stations.into_iter() {
            self.update_station_with_attributes(&mut station).await?;
            result.push(station);
        }

        Ok(result)
    }
    async fn find_line_by_id(&self, line_id: u32) -> Result<Option<Line>, UseCaseError> {
        let line = self.line_repository.find_by_id(line_id).await?;
        Ok(line)
    }

    async fn update_station_with_attributes(
        &self,
        station: &mut Station,
    ) -> Result<(), UseCaseError> {
        let belong_line = match self.find_line_by_id(station.line_cd).await {
            Ok(line) => line,
            Err(err) => return Err(UseCaseError::Unexpected(err.to_string())),
        };

        let lines = self
            .get_lines_by_station_group_id(station.station_g_cd)
            .await?;
        let mut lines_tmp: Vec<Option<Line>> = vec![None; lines.len()];

        for (index, ref mut line) in lines.into_iter().enumerate() {
            if let Some(mut belong_station_for_line) = self
                .station_repository
                .find_by_id(station.station_cd)
                .await?
            {
                let belong_station_numbers = self.get_station_numbers(
                    Box::new(belong_station_for_line.to_owned()),
                    Box::new(line.to_owned()),
                );
                belong_station_for_line.station_numbers = belong_station_numbers;
                line.station = Some(belong_station_for_line);
            }

            line.line_symbols = self.get_line_symbols(line);
            lines_tmp[index] = Some(line.clone());
        }

        station.lines = lines_tmp.into_iter().flatten().collect();

        if let Some(belong_line) = &belong_line {
            let station_numbers: Vec<StationNumber> = self.get_station_numbers(
                Box::new(station.to_owned()),
                Box::new(belong_line.to_owned()),
            );
            station.station_numbers = station_numbers;
            station.line = Some(Box::new(belong_line.to_owned()));
        }

        Ok(())
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

    fn get_station_numbers(
        &self,
        boxed_station: Box<Station>,
        boxed_line: Box<Line>,
    ) -> Vec<StationNumber> {
        let line = *boxed_line;
        let line_symbol_primary = &line.line_symbol_primary;
        let line_symbol_secondary = &line.line_symbol_secondary;
        let line_symbol_extra = &line.line_symbol_extra;
        let line_symbols_raw = vec![
            line_symbol_primary,
            line_symbol_secondary,
            line_symbol_extra,
        ];

        let line_symbol_colors_raw: Vec<String> = vec![
            line.line_symbol_primary_color.unwrap_or("".to_string()),
            line.line_symbol_secondary_color.unwrap_or("".to_string()),
            line.line_symbol_extra_color.unwrap_or("".to_string()),
        ];

        let station = *boxed_station;

        let station_numbers_raw = vec![
            station.primary_station_number.unwrap_or("".to_string()),
            station.secondary_station_number.unwrap_or("".to_string()),
            station.extra_station_number.unwrap_or("".to_string()),
        ];

        let line_symbols_shape_raw: Vec<String> = vec![
            line.line_symbol_primary_shape.unwrap_or("".to_string()),
            line.line_symbol_secondary_shape.unwrap_or("".to_string()),
            line.line_symbol_extra_shape.unwrap_or("".to_string()),
        ];

        let mut station_numbers: Vec<StationNumber> = Vec::with_capacity(station_numbers_raw.len());

        for (index, station_number) in station_numbers_raw.into_iter().enumerate() {
            let sym_color = line_symbol_colors_raw[index].to_string();
            let sym_shape = line_symbols_shape_raw[index].to_string();

            let Some(sym) = line_symbols_raw[index] else {
                return station_numbers;
            };
            if station_number.is_empty() {
                return station_numbers;
            }

            let station_number_string = format!("{}-{}", sym, station_number);

            let station_number = StationNumber {
                line_symbol: sym.to_string(),
                line_symbol_color: sym_color,
                line_symbol_shape: sym_shape,
                station_number: station_number_string,
            };

            station_numbers.push(station_number);
        }

        station_numbers
    }

    fn get_line_symbols(&self, line: &Line) -> Vec<LineSymbol> {
        let line_symbol_primary = &line.line_symbol_primary;
        let line_symbol_secondary = &line.line_symbol_secondary;
        let line_symbol_extra = &line.line_symbol_extra;
        let line_symbols_raw = vec![
            line_symbol_primary,
            line_symbol_secondary,
            line_symbol_extra,
        ];

        let line_symbol_primary_color = match line.line_symbol_primary_color {
            Some(ref color) => color.to_string(),
            None => line.line_color_c.to_string(),
        };
        let line_symbol_secondary_color = line
            .line_symbol_secondary_color
            .clone()
            .unwrap_or(String::new());
        let line_symbol_extra_color = line
            .line_symbol_extra_color
            .clone()
            .unwrap_or(String::new());

        let line_symbol_colors_raw: Vec<String> = vec![
            line_symbol_primary_color,
            line_symbol_secondary_color,
            line_symbol_extra_color,
        ];

        let line_symbol_primary_shape = line
            .line_symbol_primary_shape
            .clone()
            .unwrap_or(String::new());
        let line_symbol_secondary_shape = line
            .line_symbol_secondary_shape
            .clone()
            .unwrap_or(String::new());
        let line_symbol_extra_shape = line
            .line_symbol_extra_shape
            .clone()
            .unwrap_or(String::new());

        let line_symbols_shape_raw = vec![
            line_symbol_primary_shape,
            line_symbol_secondary_shape,
            line_symbol_extra_shape,
        ];

        if line_symbols_raw.len().is_zero() {
            return vec![];
        }

        let mut line_symbols: Vec<LineSymbol> = Vec::with_capacity(line_symbols_raw.len());

        (0..line_symbols_raw.len()).for_each(|index| {
            let Some(symbol) = line_symbols_raw[index] else{
                return;
            };
            let color = &line_symbol_colors_raw[index];
            let shape = &line_symbols_shape_raw[index];

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
