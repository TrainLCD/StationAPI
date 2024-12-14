use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
    domain::{
        entity::{
            company::Company, line::Line, line_symbol::LineSymbol, misc::StationIdWithDistance,
            station::Station, station_number::StationNumber, train_type::TrainType,
        },
        repository::{
            company_repository::CompanyRepository, line_repository::LineRepository,
            station_repository::StationRepository, train_type_repository::TrainTypeRepository,
        },
    },
    station_api::{self, Route},
    use_case::{error::UseCaseError, traits::query::QueryUseCase},
};

#[derive(Clone)]
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
        let stations = self
            .update_station_vec_with_attributes(vec![station], None)
            .await?;
        let station = stations.first().cloned();

        Ok(station)
    }
    async fn get_stations_by_id_vec(
        &self,
        station_ids: &[u32],
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self.station_repository.get_by_id_vec(station_ids).await?;
        let stations = self
            .update_station_vec_with_attributes(stations, None)
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_station_group_id(station_group_id)
            .await?;

        let stations = self
            .update_station_vec_with_attributes(stations, Some(station_group_id))
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_station_group_id_vec(station_group_id_vec)
            .await?;

        Ok(stations)
    }
    async fn get_lines_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, UseCaseError> {
        let lines = self
            .line_repository
            .get_by_station_group_id_vec(station_group_id_vec)
            .await?;

        Ok(lines)
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

        let stations = self
            .update_station_vec_with_attributes(stations, None)
            .await?;

        Ok(stations)
    }
    async fn get_station_id_and_distance_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        line_id: Option<u32>,
    ) -> Result<StationIdWithDistance, UseCaseError> {
        let station = self
            .station_repository
            .get_station_id_and_distance_by_coordinates(latitude, longitude, line_id)
            .await?;

        Ok(station)
    }
    async fn get_stations_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_line_id(line_id, station_id)
            .await?;

        let line_group_id = if let Some(sta) = stations
            .iter()
            .find(|sta| sta.station_cd == station_id.unwrap_or(0))
        {
            sta.line_group_cd
        } else {
            None
        };

        let stations = self
            .update_station_vec_with_attributes(stations, line_group_id)
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
        from_station_group_id: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_name(station_name, limit, from_station_group_id)
            .await?;

        let stations = self
            .update_station_vec_with_attributes(stations, None)
            .await?;

        Ok(stations)
    }
    async fn find_company_by_id_vec(
        &self,
        company_id_vec: &[u32],
    ) -> Result<Vec<Company>, UseCaseError> {
        let companies = self
            .company_repository
            .find_by_id_vec(company_id_vec)
            .await?;

        Ok(companies)
    }
    async fn update_station_vec_with_attributes(
        &self,
        stations: Vec<Station>,
        line_group_id: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = Arc::new(Mutex::new(stations));

        let station_group_ids = Arc::clone(&stations)
            .lock()
            .unwrap()
            .iter()
            .map(|station| station.station_g_cd)
            .collect::<Vec<u32>>();

        let stations_by_group_ids = self
            .get_stations_by_group_id_vec(&station_group_ids)
            .await?;

        let station_ids = stations_by_group_ids
            .iter()
            .map(|station| station.station_cd)
            .collect::<Vec<u32>>();

        let lines = &self
            .get_lines_by_station_group_id_vec(&station_group_ids)
            .await?;

        let company_ids = &lines
            .iter()
            .map(|station| station.company_cd)
            .collect::<Vec<u32>>();
        let companies = self.find_company_by_id_vec(company_ids).await?;

        let train_types = self
            .get_train_types_by_station_id_vec(&station_ids, line_group_id)
            .await?;

        let stations = Arc::clone(&stations)
            .lock()
            .unwrap()
            .iter_mut()
            .map(|station| {
                let mut line = self.extract_line_from_station(station);
                line.line_symbols = self.get_line_symbols(&line);
                line.company = companies
                    .iter()
                    .find(|c| c.company_cd == line.company_cd)
                    .cloned();
                line.station = Some(station.clone());

                let station_numbers: Vec<StationNumber> = self.get_station_numbers(station);
                station.station_numbers = station_numbers;
                station.line = Some(Box::new(line.clone()));
                if let Some(tt) = train_types
                    .iter()
                    .find(|tt| tt.station_cd == station.station_cd)
                    .cloned()
                    .map(Box::new)
                {
                    station.train_type = Some(tt.clone());
                };

                let mut lines: Vec<Line> = lines
                    .iter()
                    .filter(|&l| l.station_g_cd.unwrap_or(0) == station.station_g_cd)
                    .cloned()
                    .collect();
                for line in lines.iter_mut() {
                    line.company = companies
                        .iter()
                        .find(|c| c.company_cd == line.company_cd)
                        .cloned();
                    line.line_symbols = self.get_line_symbols(line);
                    if let Some(station) = stations_by_group_ids
                        .clone()
                        .iter_mut()
                        .filter(|s| s.line_cd == line.line_cd)
                        .find(|s| s.station_g_cd == station.station_g_cd)
                    {
                        let station_numbers: Vec<StationNumber> = self.get_station_numbers(station);
                        station.station_numbers = station_numbers;
                        if let Some(tt) = train_types
                            .iter()
                            .find(|tt| tt.station_cd == station.station_cd)
                            .cloned()
                            .map(Box::new)
                        {
                            station.train_type = Some(tt.clone());
                        };
                        line.station = Some(station.clone());
                    }
                }
                let station_numbers: Vec<StationNumber> = self.get_station_numbers(station);
                station.station_numbers = station_numbers;
                station.lines = lines;

                station.clone()
            })
            .collect();

        Ok(stations)
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

        let stations = self
            .update_station_vec_with_attributes(stations, Some(line_group_id))
            .await?;

        Ok(stations)
    }
    fn get_station_numbers(&self, station: &Station) -> Vec<StationNumber> {
        let station = station.clone();

        let line_symbol_primary = &station.line_symbol_primary;
        let line_symbol_secondary = &station.line_symbol_secondary;
        let line_symbol_extra = &station.line_symbol_extra;
        let line_symbols_raw = [
            line_symbol_primary,
            line_symbol_secondary,
            line_symbol_extra,
        ];

        let line_symbol_colors_raw: Vec<String> = vec![
            station.line_symbol_primary_color.unwrap_or("".to_string()),
            station
                .line_symbol_secondary_color
                .unwrap_or("".to_string()),
            station.line_symbol_extra_color.unwrap_or("".to_string()),
        ];

        let station_numbers_raw = vec![
            station.primary_station_number.unwrap_or("".to_string()),
            station.secondary_station_number.unwrap_or("".to_string()),
            station.extra_station_number.unwrap_or("".to_string()),
        ];

        let line_symbols_shape_raw: Vec<String> = vec![
            station.line_symbol_primary_shape.unwrap_or("".to_string()),
            station
                .line_symbol_secondary_shape
                .unwrap_or("".to_string()),
            station.line_symbol_extra_shape.unwrap_or("".to_string()),
        ];

        station_numbers_raw
            .into_iter()
            .enumerate()
            .filter_map(|(index, station_number)| {
                let sym_color = line_symbol_colors_raw[index].to_string();
                let sym_shape = line_symbols_shape_raw[index].to_string();

                if station_number.is_empty() {
                    return None;
                }

                if let Some(sym) = line_symbols_raw[index] {
                    let station_number_string = format!("{}-{}", sym, station_number);

                    let station_number = StationNumber {
                        line_symbol: sym.to_string(),
                        line_symbol_color: sym_color,
                        line_symbol_shape: sym_shape,
                        station_number: station_number_string,
                    };
                    return Some(station_number);
                }
                let station_number = StationNumber {
                    line_symbol: "".to_string(),
                    line_symbol_color: sym_color,
                    line_symbol_shape: sym_shape,
                    station_number,
                };
                Some(station_number)
            })
            .collect()
    }
    fn extract_line_from_station(&self, station: &Station) -> Line {
        let station = station.clone();
        Line {
            line_cd: station.line_cd,
            company_cd: station.company_cd.unwrap_or_default(),
            company: None,
            line_name: station.line_name.unwrap_or_default(),
            line_name_k: station.line_name_k.unwrap_or_default(),
            line_name_h: station.line_name_h.unwrap_or_default(),
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
            line_group_cd: None,
            station_cd: Some(station.station_cd),
            station_g_cd: Some(station.station_g_cd),
            average_distance: station.average_distance,
        }
    }
    fn get_line_symbols(&self, line: &Line) -> Vec<LineSymbol> {
        let line = Arc::new(line);

        let line_symbol_primary = &Arc::clone(&line).line_symbol_primary;
        let line_symbol_secondary = &Arc::clone(&line).line_symbol_secondary;
        let line_symbol_extra = &Arc::clone(&line).line_symbol_extra;
        let line_symbols_raw = [
            line_symbol_primary,
            line_symbol_secondary,
            line_symbol_extra,
        ];

        let line_color = &Arc::clone(&line).line_color_c;

        let line_symbol_primary_color = &Arc::clone(&line).line_symbol_primary_color;
        let line_symbol_primary_color = match line_symbol_primary_color.is_some() {
            true => line_symbol_primary_color,
            false => line_color,
        };

        let line_symbol_secondary_color = &Arc::clone(&line).line_symbol_secondary_color;
        let line_symbol_extra_color = &Arc::clone(&line).line_symbol_extra_color;

        let line_symbol_colors_raw = [
            line_symbol_primary_color,
            line_symbol_secondary_color,
            line_symbol_extra_color,
        ];

        let line_symbol_primary_shape = &Arc::clone(&line).line_symbol_primary_shape;
        let line_symbol_secondary_shape = &Arc::clone(&line).line_symbol_secondary_shape;
        let line_symbol_extra_shape = &Arc::clone(&line).line_symbol_extra_shape;

        let line_symbols_shape_raw = [
            line_symbol_primary_shape,
            line_symbol_secondary_shape,
            line_symbol_extra_shape,
        ];

        if line_symbols_raw.is_empty() {
            return vec![];
        }

        (0..line_symbols_raw.len())
            .filter_map(|index| {
                let symbol = line_symbols_raw[index];
                let color = line_symbol_colors_raw[index];
                let shape = line_symbols_shape_raw[index];

                if symbol.is_none() {
                    return None;
                }
                if shape.is_none() {
                    return None;
                }

                Some(LineSymbol {
                    symbol: symbol.clone().unwrap_or_default(),
                    color: color.clone().unwrap_or_default(),
                    shape: shape.clone().unwrap_or_default(),
                })
            })
            .collect()
    }
    async fn get_train_types_by_station_id(
        &self,
        station_id: u32,
    ) -> Result<Vec<TrainType>, UseCaseError> {
        let mut train_types = self
            .train_type_repository
            .get_by_station_id(station_id)
            .await?;

        let train_type_ids = train_types
            .iter()
            .map(|tt| tt.line_group_cd)
            .collect::<Vec<u32>>();

        let mut lines = self
            .line_repository
            .get_by_line_group_id_vec(&train_type_ids)
            .await?;

        let company_ids = lines.iter().map(|l| l.company_cd).collect::<Vec<u32>>();

        let companies = self.company_repository.find_by_id_vec(&company_ids).await?;

        let line = self.line_repository.find_by_station_id(station_id).await?;
        let Some(mut line) = line else {
            return Ok(vec![]);
        };

        for tt in train_types.iter_mut() {
            let mut lines: Vec<Line> = lines
                .iter_mut()
                .map(|l| l.clone())
                .filter(|l| l.line_group_cd.is_some())
                .filter(|l| l.line_group_cd.unwrap() == tt.line_group_cd)
                .collect::<Vec<Line>>();

            for line in lines.iter_mut() {
                line.company = companies
                    .iter()
                    .find(|c| c.company_cd == line.company_cd)
                    .cloned();
                line.line_symbols = self.get_line_symbols(line);
                let train_type: Option<TrainType> = self
                    .train_type_repository
                    .find_by_line_group_id_and_line_id(tt.line_group_cd, line.line_cd)
                    .await?;
                line.train_type = train_type;
            }

            line.company = companies
                .iter()
                .find(|c| c.company_cd == line.company_cd)
                .cloned();
            line.line_symbols = self.get_line_symbols(&line);

            tt.lines = lines;
            tt.line = Some(Box::new(line.clone()));
        }

        Ok(train_types)
    }

    async fn get_train_types_by_station_id_vec(
        &self,
        station_id_vec: &[u32],
        line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, UseCaseError> {
        let train_types = self
            .train_type_repository
            .get_types_by_station_id_vec(station_id_vec, line_group_id)
            .await?;

        Ok(train_types)
    }

    async fn get_routes(
        &self,
        from_station_id: u32,
        to_station_id: u32,
    ) -> Result<Vec<Route>, UseCaseError> {
        let stops = self
            .station_repository
            .get_route_stops(from_station_id, to_station_id)
            .await?;
        let stops = Arc::new(stops);

        let line_group_id_vec = Arc::clone(&stops)
            .iter()
            .filter_map(|row| row.line_group_cd)
            .collect::<Vec<u32>>();

        let tt_lines = self
            .line_repository
            .get_by_line_group_id_vec_for_routes(&line_group_id_vec)
            .await?;

        let tt_lines = Arc::new(tt_lines);

        let route_row_tree_map: BTreeMap<u32, Vec<Station>> = Arc::clone(&stops).iter().fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<u32, Vec<Station>>, value| {
                if let Some(line_group_cd) = value.line_group_cd {
                    acc.entry(line_group_cd).or_default().push(value.clone());
                } else {
                    acc.entry(value.line_cd).or_default().push(value.clone());
                };
                acc
            },
        );

        let routes: Vec<Route> = route_row_tree_map
            .iter()
            .filter_map(|(id, stops)| {
                let stops = stops
                    .iter()
                    .map(|row| {
                        let row = Arc::new(row);
                        let extracted_line =
                            Arc::new(self.extract_line_from_station(&Arc::clone(&row)));

                        if let Some(tt_line) = Arc::clone(&tt_lines)
                            .iter()
                            .find(|line| line.line_cd == row.line_cd)
                        {
                            let tt_lines: Vec<Line> = Arc::clone(&tt_lines)
                                .iter()
                                .map(|l| Line {
                                    line_cd: l.line_cd,
                                    company_cd: l.company_cd,
                                    company: None,
                                    line_name: l.line_name.clone(),
                                    line_name_k: l.line_name_k.clone(),
                                    line_name_h: l.line_name_h.clone(),
                                    line_name_r: l.line_name_r.clone(),
                                    line_name_zh: l.line_name_zh.clone(),
                                    line_name_ko: l.line_name_ko.clone(),
                                    line_color_c: l.line_color_c.clone(),
                                    line_type: l.line_type,
                                    line_symbols: l.line_symbols.clone(),
                                    line_symbol_primary: l.line_symbol_primary.clone(),
                                    line_symbol_secondary: l.line_symbol_secondary.clone(),
                                    line_symbol_extra: l.line_symbol_extra.clone(),
                                    line_symbol_primary_color: l.line_symbol_primary_color.clone(),
                                    line_symbol_secondary_color: l
                                        .line_symbol_secondary_color
                                        .clone(),
                                    line_symbol_extra_color: l.line_symbol_extra_color.clone(),
                                    line_symbol_primary_shape: l.line_symbol_primary_shape.clone(),
                                    line_symbol_secondary_shape: l
                                        .line_symbol_secondary_shape
                                        .clone(),
                                    line_symbol_extra_shape: l.line_symbol_extra_shape.clone(),
                                    e_status: l.e_status,
                                    e_sort: l.e_sort,
                                    station: None,
                                    train_type: None,
                                    line_group_cd: l.line_group_cd,
                                    station_cd: l.station_cd,
                                    station_g_cd: l.station_g_cd,
                                    average_distance: l.average_distance,
                                })
                                .collect();

                            let train_type = match row.type_id.is_some() {
                                true => Some(Box::new(TrainType {
                                    id: Arc::clone(&row).type_id.unwrap_or_default(),
                                    station_cd: Arc::clone(&row).station_cd,
                                    type_cd: Arc::clone(&row).type_cd.unwrap_or_default(),
                                    line_group_cd: Arc::clone(&row)
                                        .line_group_cd
                                        .unwrap_or_default(),
                                    pass: Arc::clone(&row).pass.unwrap_or_default(),
                                    type_name: Arc::clone(&row)
                                        .type_name
                                        .clone()
                                        .unwrap_or_default(),
                                    type_name_k: Arc::clone(&row)
                                        .type_name_k
                                        .clone()
                                        .unwrap_or_default(),
                                    type_name_r: Arc::clone(&row).type_name_r.clone(),
                                    type_name_zh: Arc::clone(&row).type_name_zh.clone(),
                                    type_name_ko: Arc::clone(&row).type_name_ko.clone(),
                                    color: Arc::clone(&row).color.clone().unwrap_or_default(),
                                    direction: Arc::clone(&row).direction.unwrap_or_default(),
                                    kind: Arc::clone(&row).kind.unwrap_or_default(),
                                    line: Some(Box::new(tt_line.clone())),
                                    lines: tt_lines,
                                })),
                                false => None,
                            };

                            let stop = Station {
                                station_cd: row.station_cd,
                                station_g_cd: row.station_g_cd,
                                station_name: row.station_name.clone(),
                                station_name_k: row.station_name_k.clone(),
                                station_name_r: row.station_name_r.clone(),
                                station_name_zh: row.station_name_zh.clone(),
                                station_name_ko: row.station_name_ko.clone(),
                                station_numbers: self.get_station_numbers(&Arc::clone(&row)),
                                primary_station_number: row.primary_station_number.clone(),
                                secondary_station_number: row.secondary_station_number.clone(),
                                extra_station_number: row.extra_station_number.clone(),
                                three_letter_code: row.three_letter_code.clone(),
                                line_cd: row.line_cd,
                                line: Some(Box::new(Arc::clone(&extracted_line).as_ref().clone())),
                                lines: vec![],
                                pref_cd: row.pref_cd,
                                post: row.post.clone(),
                                address: row.address.clone(),
                                lon: row.lon,
                                lat: row.lat,
                                open_ymd: row.open_ymd.clone(),
                                close_ymd: row.close_ymd.clone(),
                                e_status: row.e_status,
                                e_sort: row.e_sort,
                                stop_condition: row.stop_condition,
                                distance: row.distance,
                                train_type,
                                has_train_types: row.has_train_types,
                                company_cd: row.company_cd,
                                line_name: row.line_name.clone(),
                                line_name_k: row.line_name_k.clone(),
                                line_name_h: row.line_name_h.clone(),
                                line_name_r: row.line_name_r.clone(),
                                line_name_zh: row.line_name_zh.clone(),
                                line_name_ko: row.line_name_ko.clone(),
                                line_color_c: row.line_color_c.clone(),
                                line_type: row.line_type,
                                line_symbol_primary: row.line_symbol_primary.clone(),
                                line_symbol_secondary: row.line_symbol_secondary.clone(),
                                line_symbol_extra: row.line_symbol_extra.clone(),
                                line_symbol_primary_color: row.line_symbol_primary_color.clone(),
                                line_symbol_secondary_color: row
                                    .line_symbol_secondary_color
                                    .clone(),
                                line_symbol_extra_color: row.line_symbol_extra_color.clone(),
                                line_symbol_primary_shape: row.line_symbol_primary_shape.clone(),
                                line_symbol_secondary_shape: row
                                    .line_symbol_secondary_shape
                                    .clone(),
                                line_symbol_extra_shape: row.line_symbol_extra_shape.clone(),
                                average_distance: row.average_distance,
                                type_id: row.type_id,
                                sst_id: row.sst_id,
                                type_cd: row.type_cd,
                                line_group_cd: row.line_group_cd,
                                pass: row.pass,
                                type_name: row.type_name.clone(),
                                type_name_k: row.type_name_k.clone(),
                                type_name_r: row.type_name_r.clone(),
                                type_name_zh: row.type_name_zh.clone(),
                                type_name_ko: row.type_name_ko.clone(),
                                color: row.color.clone(),
                                direction: row.direction,
                                kind: row.kind,
                            };

                            return stop.into();
                        }

                        let stop = Station {
                            station_cd: row.station_cd,
                            station_g_cd: row.station_g_cd,
                            station_name: row.station_name.clone(),
                            station_name_k: row.station_name_k.clone(),
                            station_name_r: row.station_name_r.clone(),
                            station_name_zh: row.station_name_zh.clone(),
                            station_name_ko: row.station_name_ko.clone(),
                            station_numbers: self.get_station_numbers(&Arc::clone(&row)),
                            primary_station_number: row.primary_station_number.clone(),
                            secondary_station_number: row.secondary_station_number.clone(),
                            extra_station_number: row.extra_station_number.clone(),
                            three_letter_code: row.three_letter_code.clone(),
                            line_cd: row.line_cd,
                            line: Some(Box::new(Arc::clone(&extracted_line).as_ref().clone())),
                            lines: vec![],
                            pref_cd: row.pref_cd,
                            post: row.post.clone(),
                            address: row.address.clone(),
                            lon: row.lon,
                            lat: row.lat,
                            open_ymd: row.open_ymd.clone(),
                            close_ymd: row.close_ymd.clone(),
                            e_status: row.e_status,
                            e_sort: row.e_sort,
                            stop_condition: row.stop_condition,
                            distance: row.distance,
                            train_type: None,
                            has_train_types: row.has_train_types,
                            company_cd: row.company_cd,
                            line_name: row.line_name.clone(),
                            line_name_k: row.line_name_k.clone(),
                            line_name_h: row.line_name_h.clone(),
                            line_name_r: row.line_name_r.clone(),
                            line_name_zh: row.line_name_zh.clone(),
                            line_name_ko: row.line_name_ko.clone(),
                            line_color_c: row.line_color_c.clone(),
                            line_type: row.line_type,
                            line_symbol_primary: row.line_symbol_primary.clone(),
                            line_symbol_secondary: row.line_symbol_secondary.clone(),
                            line_symbol_extra: row.line_symbol_extra.clone(),
                            line_symbol_primary_color: row.line_symbol_primary_color.clone(),
                            line_symbol_secondary_color: row.line_symbol_secondary_color.clone(),
                            line_symbol_extra_color: row.line_symbol_extra_color.clone(),
                            line_symbol_primary_shape: row.line_symbol_primary_shape.clone(),
                            line_symbol_secondary_shape: row.line_symbol_secondary_shape.clone(),
                            line_symbol_extra_shape: row.line_symbol_extra_shape.clone(),
                            average_distance: row.average_distance,
                            type_id: row.type_id,
                            sst_id: row.sst_id,
                            type_cd: row.type_cd,
                            line_group_cd: row.line_group_cd,
                            pass: row.pass,
                            type_name: row.type_name.clone(),
                            type_name_k: row.type_name_k.clone(),
                            type_name_r: row.type_name_r.clone(),
                            type_name_zh: row.type_name_zh.clone(),
                            type_name_ko: row.type_name_ko.clone(),
                            color: row.color.clone(),
                            direction: row.direction,
                            kind: row.kind,
                        };

                        stop.into()
                    })
                    .collect::<Vec<station_api::Station>>();

                // TODO: SQLで同等の処理を行う
                let includes_requested_station = stops
                    .iter()
                    .any(|stop| stop.group_id == from_station_id || stop.group_id == to_station_id);
                if !includes_requested_station {
                    return None;
                }

                Some(Route { id: *id, stops })
            })
            .collect();
        Ok(routes)
    }

    async fn find_line_by_id(&self, line_id: u32) -> Result<Option<Line>, UseCaseError> {
        let line = self.line_repository.find_by_id(line_id).await?;
        Ok(line)
    }

    async fn get_lines_by_name(
        &self,
        line_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Line>, UseCaseError> {
        let lines = self.line_repository.get_by_name(line_name, limit).await?;
        Ok(lines)
    }
}
