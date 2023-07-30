use std::{cell::RefCell, rc::Rc, vec};

use async_trait::async_trait;
use bigdecimal::Zero;

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
        let Some(station) = self.station_repository.find_by_id(station_id).await? else {
            return Ok(None);
        };
        let result_vec = &mut vec![station];
        self.update_station_vec_with_attributes(result_vec).await?;
        Ok(result_vec.get(0).cloned())
    }

    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let mut stations = self
            .station_repository
            .get_by_station_group_id(station_group_id)
            .await?;

        self.update_station_vec_with_attributes(&mut stations)
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let mut stations = self
            .station_repository
            .get_by_coordinates(latitude, longitude, limit)
            .await?;

        self.update_station_vec_with_attributes(&mut stations)
            .await?;

        Ok(stations)
    }

    async fn get_stations_by_line_id(&self, line_id: u32) -> Result<Vec<Station>, UseCaseError> {
        let mut stations = self.station_repository.get_by_line_id(line_id).await?;

        self.update_station_vec_with_attributes(&mut stations)
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let mut stations = self
            .station_repository
            .get_by_name(station_name, limit)
            .await?;

        self.update_station_vec_with_attributes(&mut stations)
            .await?;

        Ok(stations)
    }
    async fn find_company_by_id_vec(
        &self,
        company_id_vec: Vec<u32>,
    ) -> Result<Vec<Company>, UseCaseError> {
        let companies = self
            .company_repository
            .find_by_id_vec(company_id_vec)
            .await?;

        Ok(companies)
    }

    async fn update_station_vec_with_attributes(
        &self,
        stations: &mut Vec<Station>,
    ) -> Result<(), UseCaseError> {
        let company_ids = stations
            .iter()
            .map(|station| station.company_cd)
            .collect::<Vec<u32>>();
        let companies = self.find_company_by_id_vec(company_ids).await?;

        for (index, station) in stations.iter_mut().enumerate() {
            let station_numbers: Vec<StationNumber> = self.get_station_numbers(station);
            station.station_numbers = station_numbers;
            let mut line = self.extract_line_from_station(station);
            line.line_symbols = self.get_line_symbols(&line);
            line.company = companies.get(index).cloned();
            line.station = Some(station.clone());
            station.station_numbers = self.get_station_numbers(station);
            station.line = Some(Box::new(line.clone()));

            let mut lines = self
                .get_lines_by_station_group_id(station.station_g_cd)
                .await?;
            let mut lines_tmp: Vec<&Line> = Vec::with_capacity(lines.len());

            for line in lines.iter_mut() {
                let companies = self.find_company_by_id_vec(vec![line.company_cd]).await?;
                line.line_symbols = self.get_line_symbols(line);
                line.company = companies.get(0).cloned();

                if let Some(mut station) = self
                    .station_repository
                    .get_by_station_group_and_line_id(station.station_g_cd, line.line_cd)
                    .await?
                {
                    let station_numbers: Vec<StationNumber> = self.get_station_numbers(&station);
                    station.station_numbers = station_numbers;
                    line.station = Some(station);
                }
                line.line_symbols = self.get_line_symbols(line);
                line.company = companies.get(index).cloned();

                lines_tmp.push(&*line);
            }
            station.lines = lines_tmp.into_iter().cloned().collect();
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
    async fn get_stations_by_line_group_id(
        &self,
        line_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let mut stations = self
            .station_repository
            .get_by_line_group_id(line_group_id)
            .await?;

        self.update_station_vec_with_attributes(&mut stations)
            .await?;

        Ok(stations)
    }
    fn get_station_numbers(&self, station: &Station) -> Vec<StationNumber> {
        let line_symbol_primary = &station.line_symbol_primary;
        let line_symbol_secondary = &station.line_symbol_secondary;
        let line_symbol_extra = &station.line_symbol_extra;
        let line_symbols_raw = vec![
            line_symbol_primary,
            line_symbol_secondary,
            line_symbol_extra,
        ];

        let station_rc = Rc::new(RefCell::new(station));
        let station = Rc::clone(&station_rc);
        let station = station.borrow();
        let station = station.clone();

        let line_symbol_colors_raw: Vec<Option<String>> = vec![
            station.line_symbol_primary_color,
            station.line_symbol_secondary_color,
            station.line_symbol_extra_color,
        ];

        let station = station_rc.borrow();
        let station = station.clone();

        let station_numbers_raw = vec![
            station.primary_station_number.unwrap_or("".to_string()),
            station.secondary_station_number.unwrap_or("".to_string()),
            station.extra_station_number.unwrap_or("".to_string()),
        ];

        let station = station_rc.borrow();
        let station = station.clone();

        let line_symbols_shape_raw: Vec<Option<String>> = vec![
            station.line_symbol_primary_shape,
            station.line_symbol_secondary_shape,
            station.line_symbol_extra_shape,
        ];

        let mut station_numbers: Vec<StationNumber> = Vec::with_capacity(station_numbers_raw.len());

        for (index, station_number) in station_numbers_raw.into_iter().enumerate() {
            let sym_color = &line_symbol_colors_raw[index];
            let sym_shape = &line_symbols_shape_raw[index];

            if station_number.is_empty() {
                continue;
            }

            if let Some(sym) = line_symbols_raw[index] {
                let station_number_string = format!("{}-{}", sym, station_number);

                let Some(sym_color) = sym_color else {
                    return station_numbers;
                };
                let Some(sym_shape) = sym_shape else {
                    return station_numbers;
                };

                let station_number = StationNumber {
                    line_symbol: sym.to_string(),
                    line_symbol_color: sym_color.to_string(),
                    line_symbol_shape: sym_shape.to_string(),
                    station_number: station_number_string,
                };

                station_numbers.push(station_number);
            };
        }

        station_numbers
    }

    fn extract_line_from_station(&self, station: &Station) -> Line {
        let station = Rc::new(RefCell::new(station));
        let station = Rc::clone(&station);
        let station = station.borrow();
        let station = station.clone();

        Line {
            line_cd: station.line_cd,
            company_cd: station.company_cd,
            company: None,
            line_name: station.line_name,
            line_name_k: station.line_name_k,
            line_name_h: station.line_name_h,
            line_name_r: station.line_name_r,
            line_name_zh: station.line_name_zh,
            line_name_ko: station.line_name_ko,
            line_color_c: station.line_color_c,
            line_type: station.line_type,
            line_symbols: vec![],
            line_symbol_primary: station.line_symbol_primary,
            line_symbol_secondary: station.line_symbol_secondary,
            line_symbol_extra: station.line_symbol_extra,
            line_symbol_primary_color: station.line_symbol_primary_color,
            line_symbol_secondary_color: station.line_symbol_secondary_color,
            line_symbol_extra_color: station.line_symbol_extra_color,
            line_symbol_primary_shape: station.line_symbol_primary_shape,
            line_symbol_secondary_shape: station.line_symbol_secondary_shape,
            line_symbol_extra_shape: station.line_symbol_extra_shape,
            e_status: 0,
            e_sort: 0,
            station: None,
            train_type: None,
        }
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

        for tt in train_types.iter_mut() {
            let mut lines = self
                .line_repository
                .get_by_line_group_id(tt.line_group_cd)
                .await?;

            let company_ids = lines.iter().map(|l| l.company_cd).collect();
            let companies = self.company_repository.find_by_id_vec(company_ids).await?;

            for (index, line) in lines.iter_mut().enumerate() {
                let train_type: Option<TrainType> = self
                    .train_type_repository
                    .find_by_line_group_id_and_line_id(tt.line_group_cd, line.line_cd)
                    .await?;
                line.train_type = train_type;
                line.company = companies.get(index).cloned();
                line.line_symbols = self.get_line_symbols(line);
                tt.line = Some(Box::new(line.clone()));
            }

            tt.lines = lines;
        }

        Ok(train_types)
    }
}
