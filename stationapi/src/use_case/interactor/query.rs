use std::{
    collections::{BTreeMap, BinaryHeap, HashMap, HashSet},
    sync::{Arc, Mutex},
};

use crate::{
    domain::{
        entity::{
            company::Company, line::Line, line_symbol::LineSymbol, station::Station,
            station_number::StationNumber, train_type::TrainType,
        },
        normalize::normalize_for_search,
        repository::{
            company_repository::CompanyRepository, line_repository::LineRepository,
            station_repository::StationRepository, train_type_repository::TrainTypeRepository,
        },
    },
    proto::{self, Route},
    use_case::{error::UseCaseError, traits::query::QueryUseCase},
};
use async_trait::async_trait;
use petgraph::visit::EdgeRef;
use petgraph::{graph::NodeIndex, Graph, Undirected};

#[derive(Clone)]
pub struct QueryInteractor<SR, LR, TR, CR> {
    pub station_repository: SR,
    pub line_repository: LR,
    pub train_type_repository: TR,
    pub company_repository: CR,
}

#[derive(Copy, Clone, PartialEq)]
struct DijkstraState {
    cost: f64,
    node: NodeIndex,
}

impl Eq for DijkstraState {}

impl Ord for DijkstraState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for DijkstraState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
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
        let station = stations.into_iter().next();

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
            .find(|sta| sta.station_cd == station_id.unwrap_or(0) as i64)
        {
            sta.line_group_cd
        } else {
            None
        };

        let stations = self
            .update_station_vec_with_attributes(stations, line_group_id.map(|id| id as u32))
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
            .get_by_name(
                normalize_for_search(&station_name),
                limit,
                from_station_group_id,
            )
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

        let station_group_ids = {
            let stations_guard = stations.lock().unwrap();
            stations_guard
                .iter()
                .map(|station| station.station_g_cd as u32)
                .collect::<Vec<u32>>()
        };

        let stations_by_group_ids = self
            .get_stations_by_group_id_vec(&station_group_ids)
            .await?;

        let station_ids = stations_by_group_ids
            .iter()
            .map(|station| station.station_cd as u32)
            .collect::<Vec<u32>>();

        let lines = &self
            .get_lines_by_station_group_id_vec(&station_group_ids)
            .await?;

        let company_ids = &lines
            .iter()
            .map(|station| station.company_cd as u32)
            .collect::<Vec<u32>>();
        let companies = self.find_company_by_id_vec(company_ids).await?;

        let train_types = self
            .get_train_types_by_station_id_vec(&station_ids, line_group_id)
            .await?;

        let stations = {
            let mut stations_guard = stations.lock().unwrap();
            stations_guard
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
                    station.line = Some(Box::new(line));
                    if let Some(tt) = train_types
                        .iter()
                        .find(|tt| tt.station_cd == Some(station.station_cd))
                        .cloned()
                        .map(Box::new)
                    {
                        station.train_type = Some(tt);
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
                        if let Some(station_ref) = stations_by_group_ids
                            .iter()
                            .filter(|s| s.line_cd == line.line_cd)
                            .find(|s| s.station_g_cd == station.station_g_cd)
                        {
                            let mut station_copy = station_ref.clone();
                            let station_numbers: Vec<StationNumber> =
                                self.get_station_numbers(&station_copy);
                            station_copy.station_numbers = station_numbers;
                            if let Some(tt) = train_types
                                .iter()
                                .find(|tt| tt.station_cd == Some(station_copy.station_cd))
                                .cloned()
                                .map(Box::new)
                            {
                                station_copy.train_type = Some(tt);
                            };
                            line.station = Some(station_copy);
                        }
                    }
                    let station_numbers: Vec<StationNumber> = self.get_station_numbers(station);
                    station.station_numbers = station_numbers;
                    station.lines = lines;

                    station.clone()
                })
                .collect()
        };

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
        let line_symbols_raw = [
            &station.line_symbol1,
            &station.line_symbol2,
            &station.line_symbol3,
            &station.line_symbol4,
        ];

        let line_symbol_colors_raw: Vec<String> = vec![
            station.line_symbol1_color.clone().unwrap_or_default(),
            station.line_symbol2_color.clone().unwrap_or_default(),
            station.line_symbol3_color.clone().unwrap_or_default(),
            station.line_symbol4_color.clone().unwrap_or_default(),
        ];

        let station_numbers_raw = vec![
            station.station_number1.clone().unwrap_or_default(),
            station.station_number2.clone().unwrap_or_default(),
            station.station_number3.clone().unwrap_or_default(),
            station.station_number4.clone().unwrap_or_default(),
        ];

        let line_symbols_shape_raw: Vec<String> = vec![
            station.line_symbol1_shape.clone().unwrap_or_default(),
            station.line_symbol2_shape.clone().unwrap_or_default(),
            station.line_symbol3_shape.clone().unwrap_or_default(),
            station.line_symbol4_shape.clone().unwrap_or_default(),
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
        Line {
            line_cd: station.line_cd,
            company_cd: station.company_cd.unwrap_or_default(),
            company: None,
            line_name: station.line_name.clone().unwrap_or_default(),
            line_name_k: station.line_name_k.clone().unwrap_or_default(),
            line_name_h: station.line_name_h.clone().unwrap_or_default(),
            line_name_r: station.line_name_r.clone(),
            line_name_zh: station.line_name_zh.clone(),
            line_name_ko: station.line_name_ko.clone(),
            line_color_c: station.line_color_c.clone(),
            line_type: station.line_type,
            line_symbols: vec![],
            line_symbol1: station.line_symbol1.clone(),
            line_symbol2: station.line_symbol2.clone(),
            line_symbol3: station.line_symbol3.clone(),
            line_symbol4: station.line_symbol4.clone(),
            line_symbol1_color: station.line_symbol1_color.clone(),
            line_symbol2_color: station.line_symbol2_color.clone(),
            line_symbol3_color: station.line_symbol3_color.clone(),
            line_symbol4_color: station.line_symbol4_color.clone(),
            line_symbol1_shape: station.line_symbol1_shape.clone(),
            line_symbol2_shape: station.line_symbol2_shape.clone(),
            line_symbol3_shape: station.line_symbol3_shape.clone(),
            line_symbol4_shape: station.line_symbol4_shape.clone(),
            e_status: 0,
            e_sort: 0,
            station: None,
            train_type: None,
            line_group_cd: None,
            station_cd: Some(station.station_cd),
            station_g_cd: Some(station.station_g_cd),
            average_distance: station.average_distance,
            type_cd: station.type_cd,
        }
    }
    fn get_line_symbols(&self, line: &Line) -> Vec<LineSymbol> {
        let line_symbols_raw = [&line.line_symbol1, &line.line_symbol2, &line.line_symbol3];

        let line_symbol1_color = line
            .line_symbol1_color
            .as_ref()
            .or(line.line_color_c.as_ref());
        let line_symbol_colors_raw = [
            line_symbol1_color,
            line.line_symbol2_color.as_ref(),
            line.line_symbol3_color.as_ref(),
        ];

        let line_symbols_shape_raw = [
            &line.line_symbol1_shape,
            &line.line_symbol2_shape,
            &line.line_symbol3_shape,
        ];

        if line_symbols_raw.is_empty() {
            return vec![];
        }

        (0..line_symbols_raw.len())
            .filter_map(|index| {
                let symbol = line_symbols_raw[index].as_ref()?;
                let shape = line_symbols_shape_raw[index].as_ref()?;
                let color = line_symbol_colors_raw[index].cloned().unwrap_or_default();

                Some(LineSymbol {
                    symbol: symbol.clone(),
                    color,
                    shape: shape.clone(),
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
            .filter_map(|tt| tt.line_group_cd.map(|id| id as u32))
            .collect::<Vec<u32>>();

        let mut lines = self
            .line_repository
            .get_by_line_group_id_vec(&train_type_ids)
            .await?;

        let company_ids = lines
            .iter()
            .map(|l| l.company_cd as u32)
            .collect::<Vec<u32>>();

        let companies = self.company_repository.find_by_id_vec(&company_ids).await?;

        let line = self.line_repository.find_by_station_id(station_id).await?;
        let Some(mut line) = line else {
            return Ok(vec![]);
        };

        for tt in train_types.iter_mut() {
            if let Some(line_group_cd) = tt.line_group_cd {
                let mut lines: Vec<Line> = lines
                    .iter_mut()
                    .map(|l| l.clone())
                    .filter(|l| l.line_group_cd.is_some())
                    .filter(|l| l.line_group_cd.unwrap() == line_group_cd)
                    .collect::<Vec<Line>>();

                for line in lines.iter_mut() {
                    line.company = companies
                        .iter()
                        .find(|c| c.company_cd == line.company_cd)
                        .cloned();
                    line.line_symbols = self.get_line_symbols(line);

                    let train_type: Option<TrainType> = self
                        .train_type_repository
                        .find_by_line_group_id_and_line_id(
                            line_group_cd as u32,
                            line.line_cd as u32,
                        )
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
            .filter_map(|row| row.line_group_cd.map(|id| id as u32))
            .collect::<Vec<u32>>();

        let tt_lines = self
            .line_repository
            .get_by_line_group_id_vec_for_routes(&line_group_id_vec)
            .await?;

        let tt_lines = Arc::new(tt_lines);

        let route_row_tree_map: BTreeMap<i64, Vec<Station>> = Arc::clone(&stops).iter().fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<i64, Vec<Station>>, value| {
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
                        let extracted_line = self.extract_line_from_station(row);

                        if let Some(tt_line) =
                            tt_lines.iter().find(|line| line.line_cd == row.line_cd)
                        {
                            let tt_lines_cloned: Vec<Line> = tt_lines.iter().cloned().collect();

                            let train_type = match row.type_id.is_some() {
                                true => Some(Box::new(TrainType {
                                    id: row.type_id,
                                    station_cd: Some(row.station_cd),
                                    type_cd: row.type_cd,
                                    line_group_cd: row.line_group_cd,
                                    pass: row.pass,
                                    type_name: row.type_name.clone().unwrap_or_default(),
                                    type_name_k: row.type_name_k.clone().unwrap_or_default(),
                                    type_name_r: row.type_name_r.clone(),
                                    type_name_zh: row.type_name_zh.clone(),
                                    type_name_ko: row.type_name_ko.clone(),
                                    color: row.color.clone().unwrap_or_default(),
                                    direction: row.direction,
                                    kind: row.kind,
                                    line: Some(Box::new(tt_line.clone())),
                                    lines: tt_lines_cloned,
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
                                station_numbers: self.get_station_numbers(row),
                                station_number1: row.station_number1.clone(),
                                station_number2: row.station_number2.clone(),
                                station_number3: row.station_number3.clone(),
                                station_number4: row.station_number4.clone(),
                                three_letter_code: row.three_letter_code.clone(),
                                line_cd: row.line_cd,
                                line: Some(Box::new(extracted_line.clone())),
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
                                line_symbol1: row.line_symbol1.clone(),
                                line_symbol2: row.line_symbol2.clone(),
                                line_symbol3: row.line_symbol3.clone(),
                                line_symbol4: row.line_symbol4.clone(),
                                line_symbol1_color: row.line_symbol1_color.clone(),
                                line_symbol2_color: row.line_symbol2_color.clone(),
                                line_symbol3_color: row.line_symbol3_color.clone(),
                                line_symbol4_color: row.line_symbol4_color.clone(),
                                line_symbol1_shape: row.line_symbol1_shape.clone(),
                                line_symbol2_shape: row.line_symbol2_shape.clone(),
                                line_symbol3_shape: row.line_symbol3_shape.clone(),
                                line_symbol4_shape: row.line_symbol4_shape.clone(),
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
                            station_numbers: self.get_station_numbers(row),
                            station_number1: row.station_number1.clone(),
                            station_number2: row.station_number2.clone(),
                            station_number3: row.station_number3.clone(),
                            station_number4: row.station_number4.clone(),
                            three_letter_code: row.three_letter_code.clone(),
                            line_cd: row.line_cd,
                            line: Some(Box::new(extracted_line.clone())),
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
                            line_symbol1: row.line_symbol1.clone(),
                            line_symbol2: row.line_symbol2.clone(),
                            line_symbol3: row.line_symbol3.clone(),
                            line_symbol4: row.line_symbol4.clone(),
                            line_symbol1_color: row.line_symbol1_color.clone(),
                            line_symbol2_color: row.line_symbol2_color.clone(),
                            line_symbol3_color: row.line_symbol3_color.clone(),
                            line_symbol4_color: row.line_symbol4_color.clone(),
                            line_symbol1_shape: row.line_symbol1_shape.clone(),
                            line_symbol2_shape: row.line_symbol2_shape.clone(),
                            line_symbol3_shape: row.line_symbol3_shape.clone(),
                            line_symbol4_shape: row.line_symbol4_shape.clone(),
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
                    .collect::<Vec<proto::Station>>();

                // TODO: SQLで同等の処理を行う
                let includes_requested_station = stops
                    .iter()
                    .any(|stop| stop.group_id == from_station_id || stop.group_id == to_station_id);
                if !includes_requested_station {
                    return None;
                }

                Some(Route {
                    id: *id as u32,
                    stops,
                })
            })
            .collect();
        Ok(routes)
    }

    async fn get_train_types(
        &self,
        from_station_id: u32,
        to_station_id: u32,
    ) -> Result<Vec<TrainType>, UseCaseError> {
        let stops = self
            .station_repository
            .get_route_stops(from_station_id, to_station_id)
            .await?;
        let stops = Arc::new(stops);

        let line_group_id_vec: Vec<u32> = Arc::clone(&stops)
            .iter()
            .filter_map(|stop| stop.line_group_cd.map(|id| id as u32))
            .collect::<HashSet<u32>>()
            .into_iter()
            .collect();

        let mut result: Vec<TrainType> = Vec::with_capacity(line_group_id_vec.len());

        let train_types = self
            .train_type_repository
            .get_by_line_group_id_vec(&line_group_id_vec)
            .await?;

        let tt_lines = self
            .line_repository
            .get_by_line_group_id_vec(&line_group_id_vec)
            .await?;

        for mut train_type in train_types.clone() {
            if result
                .iter()
                .any(|t| t.line_group_cd == train_type.line_group_cd)
            {
                continue;
            }

            train_type.lines = tt_lines
                .iter()
                .filter(|line| line.line_group_cd == train_type.line_group_cd)
                .map(|line| Line {
                    line_cd: line.line_cd,
                    company_cd: line.company_cd,
                    company: None,
                    line_name: line.line_name.clone(),
                    line_name_k: line.line_name_k.clone(),
                    line_name_h: line.line_name_h.clone(),
                    line_name_r: line.line_name_r.clone(),
                    line_name_zh: line.line_name_zh.clone(),
                    line_name_ko: line.line_name_ko.clone(),
                    line_color_c: line.line_color_c.clone(),
                    line_type: line.line_type,
                    line_symbols: line.line_symbols.clone(),
                    line_symbol1: line.line_symbol1.clone(),
                    line_symbol2: line.line_symbol2.clone(),
                    line_symbol3: line.line_symbol3.clone(),
                    line_symbol4: line.line_symbol4.clone(),
                    line_symbol1_color: line.line_symbol1_color.clone(),
                    line_symbol2_color: line.line_symbol2_color.clone(),
                    line_symbol3_color: line.line_symbol3_color.clone(),
                    line_symbol4_color: line.line_symbol4_color.clone(),
                    line_symbol1_shape: line.line_symbol1_shape.clone(),
                    line_symbol2_shape: line.line_symbol2_shape.clone(),
                    line_symbol3_shape: line.line_symbol3_shape.clone(),
                    line_symbol4_shape: line.line_symbol4_shape.clone(),
                    e_status: line.e_status,
                    e_sort: line.e_sort,
                    average_distance: line.average_distance,
                    station: None,
                    train_type: train_types
                        .iter()
                        .find(|tt| tt.type_cd == line.type_cd)
                        .cloned(),
                    line_group_cd: line.line_group_cd,
                    station_cd: line.station_cd,
                    station_g_cd: line.station_g_cd,
                    type_cd: line.type_cd,
                })
                .collect::<Vec<Line>>();
            result.push(train_type);
        }
        Ok(result)
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
        let lines = self
            .line_repository
            .get_by_name(normalize_for_search(&line_name), limit)
            .await?;
        Ok(lines)
    }

    fn dijkstra_with_path(
        &self,
        graph: &Graph<i32, f64, Undirected>,
        start: NodeIndex,
    ) -> (
        HashMap<NodeIndex, f64>,
        HashMap<NodeIndex, Option<NodeIndex>>,
    ) {
        let mut dist_map = HashMap::new();
        let mut prev_map = HashMap::new();

        for node in graph.node_indices() {
            dist_map.insert(node, f64::INFINITY);
            prev_map.insert(node, None);
        }
        *dist_map.get_mut(&start).unwrap() = 0.0;

        let mut heap = BinaryHeap::new();
        heap.push(DijkstraState {
            cost: 0.0,
            node: start,
        });

        while let Some(DijkstraState { cost, node }) = heap.pop() {
            if cost > dist_map[&node] {
                continue;
            }
            for edge in graph.edges(node) {
                let next = edge.target();
                let next_cost = cost + edge.weight();

                if next_cost < dist_map[&next] {
                    *dist_map.get_mut(&next).unwrap() = next_cost;
                    *prev_map.get_mut(&next).unwrap() = Some(node);
                    heap.push(DijkstraState {
                        cost: next_cost,
                        node: next,
                    });
                }
            }
        }

        (dist_map, prev_map)
    }

    fn reconstruct_path(
        &self,
        prev_map: &HashMap<NodeIndex, Option<NodeIndex>>,
        start: NodeIndex,
        goal: NodeIndex,
    ) -> Option<Vec<NodeIndex>> {
        let mut path = Vec::new();
        let mut current = goal;
        while let Some(&Some(prev)) = prev_map.get(&current) {
            path.push(current);
            current = prev;

            if current == start {
                path.push(start);
                path.reverse();
                return Some(path);
            }
        }
        None
    }

    // TODO: SQLite版ではいったん未実装のままにしておく
    async fn get_connected_stations(
        &self,
        _from_station_id: u32,
        _to_station_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        // let conns = self.connection_repository.get_all().await?;

        // let edges = conns.into_iter().map(|conn| {
        //     let start = conn.station_cd1;
        //     let goal = conn.station_cd2;
        //     let weight = conn.distance;
        //     (start, goal, weight)
        // });

        // let graph = UnGraph::<i32, f64>::from_edges(edges);
        // let (_dist_map, prev_map) = self.dijkstra_with_path(&graph, from_station_id.into());

        // if let Some(path) =
        //     self.reconstruct_path(&prev_map, from_station_id.into(), to_station_id.into())
        // {
        //     let node_ids: Vec<u32> = path.to_vec().iter().map(|x| x.index() as u32).collect();
        //     let stations = self.station_repository.get_by_id_vec(&node_ids).await?;
        //     let stations = self
        //         .update_station_vec_with_attributes(stations, None)
        //         .await?;
        //     return Ok(stations);
        // }

        Ok(vec![])
    }
}
