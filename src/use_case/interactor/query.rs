use std::vec;

use async_trait::async_trait;
use bigdecimal::Zero;
use moka::future::Cache;

use crate::{
    domain::{
        entity::{
            company::Company, line::Line, line_symbol::LineSymbol, station::Station,
            station_number::StationNumber, train_type::TrainType,
        },
        repository::{
            company_repository::CompanyRepository, line_repository::LineRepository,
            station_repository::StationRepository, train_type_repository::TrainTypeRepository,
        },
    },
    use_case::{error::UseCaseError, traits::query::QueryUseCase},
};

#[derive(Debug, Clone)]
pub struct QueryInteractor<SR, LR, TR, CR> {
    pub station_repository: SR,
    pub line_repository: LR,
    pub train_type_repository: TR,
    pub company_repository: CR,
    pub attributes_cache: Cache<String, Station>,
}

#[async_trait]
impl<SR, LR, TR, CR> QueryUseCase for QueryInteractor<SR, LR, TR, CR>
where
    SR: StationRepository,
    LR: LineRepository,
    TR: TrainTypeRepository,
    CR: CompanyRepository,
{
    async fn find_station_by_id(&self, station_id: u32) -> Result<Option<Station>, UseCaseError> {
        let Some( station) = self.station_repository.find_by_id(station_id).await? else {
            return Ok(None);
        };
        let station = self.get_station_with_attributes(station).await?;
        Ok(Some(station))
    }

    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_station_group_id(station_group_id)
            .await?;

        let mut result: Vec<Station> = Vec::with_capacity(stations.len());

        for station in stations.into_iter() {
            let station = self.get_station_with_attributes(station).await?;
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
            .get_by_coordinates(latitude, longitude, limit)
            .await?;

        let mut result: Vec<Station> = Vec::with_capacity(stations.len());

        for station in stations.into_iter() {
            let station = self.get_station_with_attributes(station).await?;
            result.push(station);
        }

        Ok(result)
    }

    async fn get_stations_by_line_id(&self, line_id: u32) -> Result<Vec<Station>, UseCaseError> {
        let stations = self.station_repository.get_by_line_id(line_id).await?;
        let mut result: Vec<Station> = Vec::with_capacity(stations.len());

        for station in stations.into_iter() {
            let station = self.get_station_with_attributes(station).await?;
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
            .get_by_name(station_name, limit)
            .await?;
        let mut result: Vec<Station> = Vec::with_capacity(stations.len());

        for station in stations.into_iter() {
            let station = self.get_station_with_attributes(station).await?;
            result.push(station);
        }

        Ok(result)
    }
    async fn find_line_by_id(&self, line_id: u32) -> Result<Option<Line>, UseCaseError> {
        let line = self.line_repository.find_by_id(line_id).await?;
        Ok(line)
    }
    async fn find_company_by_id(&self, company_id: u32) -> Result<Option<Company>, UseCaseError> {
        let Some(company) = self.company_repository.find_by_id(company_id).await? else {
            return Ok(None);
        };

        Ok(Some(company))
    }

    async fn get_station_with_attributes(&self, station: Station) -> Result<Station, UseCaseError> {
        let cloned_station = station.clone();
        let mut mutable_station = station;

        let cache = self.attributes_cache.clone();
        let cache_key = format!(
            "station_with_attributes:id:{}:stop_condition:{:?}",
            cloned_station.station_cd, cloned_station.stop_condition
        );
        if let Some(ref mut cache_data) = cache.get(&cache_key) {
            return Ok(cache_data.clone());
        }

        let mut belong_line = match self.find_line_by_id(cloned_station.line_cd).await {
            Ok(line) => line,
            Err(err) => return Err(UseCaseError::Unexpected(err.to_string())),
        };

        let lines = self
            .get_lines_by_station_group_id(cloned_station.station_g_cd)
            .await?;

        let mut lines_tmp: Vec<Option<Line>> = Vec::with_capacity(lines.len());

        let mut stations = self
            .station_repository
            .get_by_station_group_id(cloned_station.station_g_cd)
            .await?;

        for ref mut line in lines.into_iter() {
            for station in stations.iter_mut() {
                if station.line_cd == line.line_cd {
                    station.station_numbers = self.get_station_numbers(
                        Box::new(station.to_owned()),
                        Box::new(line.to_owned()),
                    );

                    let company = self.find_company_by_id(line.company_cd).await?;
                    line.company = company;

                    line.station = Some(station.to_owned());
                }
            }

            line.line_symbols = self.get_line_symbols(line);
            lines_tmp.push(Some(line.clone()));
        }

        mutable_station.lines = lines_tmp.into_iter().flatten().collect();

        if let Some(ref mut belong_line) = belong_line {
            let station_numbers: Vec<StationNumber> = self.get_station_numbers(
                Box::new(mutable_station.to_owned()),
                Box::new(belong_line.to_owned()),
            );

            mutable_station.station_numbers = station_numbers;

            let company = self.find_company_by_id(belong_line.company_cd).await?;
            belong_line.company = company;

            mutable_station.line = Some(Box::new(belong_line.to_owned()));
        }

        cache.insert(cache_key, mutable_station.clone()).await;

        Ok(mutable_station.clone())
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
    async fn get_stations_by_line_group_id(
        &self,
        line_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_line_group_id(line_group_id)
            .await?;

        let mut result: Vec<Station> = Vec::with_capacity(stations.len());

        for station in stations.into_iter() {
            let station = self.get_station_with_attributes(station).await?;
            result.push(station);
        }

        Ok(result)
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

            if station_number.is_empty() {
                continue;
            }

            if let Some(sym) = line_symbols_raw[index] {
                let station_number_string = format!("{}-{}", sym, station_number);

                let station_number = StationNumber {
                    line_symbol: sym.to_string(),
                    line_symbol_color: sym_color,
                    line_symbol_shape: sym_shape,
                    station_number: station_number_string,
                };

                station_numbers.push(station_number);
            };
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

    async fn get_train_types_by_station_id(
        &self,
        station_id: u32,
    ) -> Result<Vec<TrainType>, UseCaseError> {
        let mut train_types = self
            .train_type_repository
            .get_by_station_id(station_id)
            .await?;

        // TODO: SQL発行しすぎ罪で即死刑になるので神奈川県警に見つかる前にバッチ的にデータを取れるようにする
        for tt in train_types.iter_mut() {
            let mut lines = self
                .line_repository
                .get_by_line_group_id(tt.line_group_cd)
                .await?;
            for line in lines.iter_mut() {
                let train_type: Option<TrainType> = self
                    .train_type_repository
                    .find_by_line_group_id_and_line_id(tt.line_group_cd, line.line_cd)
                    .await?;
                line.train_type = train_type;
                let company = self.find_company_by_id(line.company_cd).await?;
                line.company = company;
            }

            tt.lines = lines;
            let line = self.line_repository.find_by_station_id(station_id).await?;
            tt.line = line.map(Box::new);
        }

        Ok(train_types)
    }
}
