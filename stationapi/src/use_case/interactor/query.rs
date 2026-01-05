use std::collections::{BTreeMap, HashSet};

/// Maximum distance in meters to search for nearby bus stops from a rail station
const NEARBY_BUS_STOP_RADIUS_METERS: f64 = 300.0;

/// Check if a station's transport type matches the filter
fn matches_transport_filter(station_type: TransportType, filter: TransportTypeFilter) -> bool {
    match filter {
        TransportTypeFilter::Rail => station_type == TransportType::Rail,
        TransportTypeFilter::Bus => station_type == TransportType::Bus,
        TransportTypeFilter::RailAndBus => true,
    }
}

/// Convert TransportTypeFilter to Option<TransportType> for repository queries
fn filter_to_db_type(filter: TransportTypeFilter) -> Option<TransportType> {
    match filter {
        TransportTypeFilter::Rail => Some(TransportType::Rail),
        TransportTypeFilter::Bus => Some(TransportType::Bus),
        TransportTypeFilter::RailAndBus => None, // No filter - return all
    }
}

use crate::{
    domain::{
        entity::{
            company::Company,
            gtfs::{TransportType, TransportTypeFilter},
            line::Line,
            line_symbol::LineSymbol,
            station::Station,
            station_number::StationNumber,
            train_type::TrainType,
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
    async fn find_station_by_id(
        &self,
        station_id: u32,
        transport_type: TransportTypeFilter,
    ) -> Result<Option<Station>, UseCaseError> {
        let Some(station) = self.station_repository.find_by_id(station_id).await? else {
            return Ok(None);
        };
        // Filter by transport_type
        if !matches_transport_filter(station.transport_type, transport_type) {
            return Ok(None);
        }
        let stations = self
            .update_station_vec_with_attributes(vec![station], None, transport_type)
            .await?;

        Ok(stations.into_iter().next())
    }
    async fn get_stations_by_id_vec(
        &self,
        station_ids: &[u32],
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self.station_repository.get_by_id_vec(station_ids).await?;
        // Filter by transport_type
        let stations: Vec<Station> = stations
            .into_iter()
            .filter(|s| matches_transport_filter(s.transport_type, transport_type))
            .collect();
        let stations = self
            .update_station_vec_with_attributes(stations, None, transport_type)
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_station_group_id(station_group_id)
            .await?;

        // Filter by transport_type
        let stations: Vec<Station> = stations
            .into_iter()
            .filter(|s| matches_transport_filter(s.transport_type, transport_type))
            .collect();

        let stations = self
            .update_station_vec_with_attributes(stations, Some(station_group_id), transport_type)
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
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_coordinates(
                latitude,
                longitude,
                limit,
                filter_to_db_type(transport_type),
            )
            .await?;

        let stations = self
            .update_station_vec_with_attributes(stations, None, transport_type)
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
        direction_id: Option<u32>,
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_line_id(line_id, station_id, direction_id)
            .await?;

        let line_group_id = if let Some(sta) = stations
            .iter()
            .find(|sta| sta.station_cd == station_id.unwrap_or(0) as i32)
        {
            sta.line_group_cd
        } else {
            None
        };

        let stations = self
            .update_station_vec_with_attributes(
                stations,
                line_group_id.map(|id| id as u32),
                transport_type,
            )
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
        from_station_group_id: Option<u32>,
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_name(
                normalize_for_search(&station_name),
                limit,
                from_station_group_id,
                filter_to_db_type(transport_type),
            )
            .await?;

        let stations = self
            .update_station_vec_with_attributes(stations, None, transport_type)
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
        mut stations: Vec<Station>,
        line_group_id: Option<u32>,
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError> {
        let station_group_ids = stations
            .iter()
            .map(|station| station.station_g_cd as u32)
            .collect::<Vec<u32>>();

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

        // Build HashMap for O(1) company lookup instead of O(n) linear search
        let company_map: std::collections::HashMap<i32, &Company> =
            companies.iter().map(|c| (c.company_cd, c)).collect();

        let train_types = self
            .get_train_types_by_station_id_vec(&station_ids, line_group_id)
            .await?;

        // Build HashMap for O(1) train_type lookup by station_cd
        let train_type_map: std::collections::HashMap<i32, &TrainType> = train_types
            .iter()
            .filter_map(|tt| tt.station_cd.map(|cd| (cd, tt)))
            .collect();

        // Build lookup map for stations_by_group_ids: (line_cd, station_g_cd) -> Station
        let station_lookup: std::collections::HashMap<(i32, i32), &Station> = stations_by_group_ids
            .iter()
            .map(|s| ((s.line_cd, s.station_g_cd), s))
            .collect();

        for station in stations.iter_mut() {
            let mut line = self.extract_line_from_station(station);
            line.line_symbols = self.get_line_symbols(&line);
            line.company = company_map.get(&line.company_cd).cloned().cloned();
            line.station = Some(station.clone());

            let station_numbers: Vec<StationNumber> = self.get_station_numbers(station);
            station.station_numbers = station_numbers;
            station.line = Some(Box::new(line));
            if let Some(tt) = train_type_map
                .get(&station.station_cd)
                .cloned()
                .cloned()
                .map(Box::new)
            {
                station.train_type = Some(tt);
            };

            let mut seen_line_cds = std::collections::HashSet::new();
            let mut lines: Vec<Line> = lines
                .iter()
                .filter(|&l| {
                    l.station_g_cd.unwrap_or(0) == station.station_g_cd
                        && seen_line_cds.insert(l.line_cd)
                        && matches_transport_filter(l.transport_type, transport_type)
                })
                .cloned()
                .collect();

            // For rail stations, add nearby bus routes to lines array
            // Only add bus routes if transport_type is RailAndBus
            let should_include_bus_routes = transport_type == TransportTypeFilter::RailAndBus;
            if station.transport_type == TransportType::Rail && should_include_bus_routes {
                let nearby_bus_lines = self.get_nearby_bus_lines(station.lat, station.lon).await?;
                for bus_line in nearby_bus_lines {
                    if seen_line_cds.insert(bus_line.line_cd) {
                        lines.push(bus_line);
                    }
                }
            }

            for line in lines.iter_mut() {
                line.company = company_map.get(&line.company_cd).cloned().cloned();
                line.line_symbols = self.get_line_symbols(line);
                if let Some(station_ref) = station_lookup.get(&(line.line_cd, station.station_g_cd))
                {
                    let mut station_copy = (*station_ref).clone();
                    let station_numbers: Vec<StationNumber> =
                        self.get_station_numbers(&station_copy);
                    station_copy.station_numbers = station_numbers;
                    if let Some(tt) = train_type_map
                        .get(&station_copy.station_cd)
                        .cloned()
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
        }

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
        transport_type: TransportTypeFilter,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_line_group_id(line_group_id)
            .await?;

        let stations = self
            .update_station_vec_with_attributes(
                stations,
                Some(line_group_id),
                transport_type,
            )
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

        let line_symbol_colors_raw: [&str; 4] = [
            station.line_symbol1_color.as_deref().unwrap_or_default(),
            station.line_symbol2_color.as_deref().unwrap_or_default(),
            station.line_symbol3_color.as_deref().unwrap_or_default(),
            station.line_symbol4_color.as_deref().unwrap_or_default(),
        ];

        let station_numbers_raw = [
            station.station_number1.as_deref().unwrap_or_default(),
            station.station_number2.as_deref().unwrap_or_default(),
            station.station_number3.as_deref().unwrap_or_default(),
            station.station_number4.as_deref().unwrap_or_default(),
        ];

        let line_symbols_shape_raw: [&str; 4] = [
            station.line_symbol1_shape.as_deref().unwrap_or_default(),
            station.line_symbol2_shape.as_deref().unwrap_or_default(),
            station.line_symbol3_shape.as_deref().unwrap_or_default(),
            station.line_symbol4_shape.as_deref().unwrap_or_default(),
        ];

        station_numbers_raw
            .into_iter()
            .enumerate()
            .filter_map(|(index, station_number)| {
                let sym_color = line_symbol_colors_raw[index];
                let sym_shape = line_symbols_shape_raw[index];

                if station_number.is_empty() {
                    return None;
                }

                if let Some(sym) = line_symbols_raw[index] {
                    let station_number_string = format!("{sym}-{station_number}");

                    let station_number = StationNumber {
                        line_symbol: sym.to_string(),
                        line_symbol_color: sym_color.to_string(),
                        line_symbol_shape: sym_shape.to_string(),
                        station_number: station_number_string,
                    };
                    return Some(station_number);
                }
                let station_number = StationNumber {
                    line_symbol: "".to_string(),
                    line_symbol_color: sym_color.to_string(),
                    line_symbol_shape: sym_shape.to_string(),
                    station_number: station_number.to_string(),
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
            line_name: station.line_name.as_deref().unwrap_or_default().to_string(),
            line_name_k: station
                .line_name_k
                .as_deref()
                .unwrap_or_default()
                .to_string(),
            line_name_h: station
                .line_name_h
                .as_deref()
                .unwrap_or_default()
                .to_string(),
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
            transport_type: station.transport_type,
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
                    symbol: symbol.to_string(),
                    color,
                    shape: shape.to_string(),
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

        let lines = self
            .line_repository
            .get_by_line_group_id_vec(&train_type_ids)
            .await?;

        let company_ids = lines
            .iter()
            .map(|l| l.company_cd as u32)
            .collect::<Vec<u32>>();

        let companies = self.company_repository.find_by_id_vec(&company_ids).await?;

        // Build HashMap for O(1) company lookup
        let company_map: std::collections::HashMap<i32, &Company> =
            companies.iter().map(|c| (c.company_cd, c)).collect();

        let line = self.line_repository.find_by_station_id(station_id).await?;
        let Some(mut line) = line else {
            return Ok(vec![]);
        };

        for tt in train_types.iter_mut() {
            if let Some(line_group_cd) = tt.line_group_cd {
                let mut lines: Vec<Line> = lines
                    .iter()
                    .filter(|l| l.line_group_cd == Some(line_group_cd))
                    .cloned()
                    .collect::<Vec<Line>>();

                for line in lines.iter_mut() {
                    line.company = company_map.get(&line.company_cd).cloned().cloned();
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

                line.company = company_map.get(&line.company_cd).cloned().cloned();
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
        via_line_id: Option<u32>,
    ) -> Result<Vec<Route>, UseCaseError> {
        let stops = self
            .station_repository
            .get_route_stops(from_station_id, to_station_id, via_line_id)
            .await?;

        let route_row_tree_map = self.build_route_tree_map(&stops);

        let mut routes: Vec<Route> = Vec::new();

        for (id, stops) in route_row_tree_map.iter() {
            let line_group_id_vec = stops
                .iter()
                .filter_map(|row| row.line_group_cd.map(|id| id as u32))
                .collect::<Vec<u32>>();

            let mut tt_lines = self
                .line_repository
                .get_by_line_group_id_vec_for_routes(&line_group_id_vec)
                .await?;

            // Add line_symbols to all lines first
            for line in tt_lines.iter_mut() {
                line.line_symbols = self.get_line_symbols(line);
            }

            // Build HashMap for O(1) line lookup by line_cd instead of O(n) linear search
            let tt_line_map: std::collections::HashMap<i32, &Line> =
                tt_lines.iter().map(|line| (line.line_cd, line)).collect();

            let stops = stops
                .iter()
                .map(|row| {
                    let extracted_line = self.extract_line_from_station(row);

                    if let Some(tt_line) = tt_line_map.get(&row.line_cd).copied() {
                        let train_type = match row.type_id.is_some() {
                            true => {
                                // Filter lines to only include those with matching line_group_cd
                                // and remove duplicates by line_cd
                                let mut seen_line_cds = std::collections::HashSet::new();
                                let filtered_lines: Vec<Line> = tt_lines
                                    .iter()
                                    .filter(|line| {
                                        row.line_group_cd.is_some()
                                            && line.line_group_cd == row.line_group_cd
                                            && seen_line_cds.insert(line.line_cd)
                                    })
                                    .cloned()
                                    .collect();

                                Some(Box::new(TrainType {
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
                                    lines: filtered_lines,
                                }))
                            }
                            false => None,
                        };

                        let stop = self.build_station_from_row(row, &extracted_line, train_type);

                        return stop.into();
                    }

                    let stop = self.build_station_from_row(row, &extracted_line, None);

                    stop.into()
                })
                .collect::<Vec<proto::Station>>();

            // TODO: SQLで同等の処理を行う
            let includes_requested_station = stops
                .iter()
                .any(|stop| stop.group_id == from_station_id || stop.group_id == to_station_id);
            if !includes_requested_station {
                continue;
            }

            routes.push(Route {
                id: *id as u32,
                stops,
            });
        }
        Ok(routes)
    }

    async fn get_routes_minimal(
        &self,
        from_station_id: u32,
        to_station_id: u32,
        via_line_id: Option<u32>,
    ) -> Result<proto::RouteMinimalResponse, UseCaseError> {
        let stops = self
            .station_repository
            .get_route_stops(from_station_id, to_station_id, via_line_id)
            .await?;

        let route_row_tree_map = self.build_route_tree_map(&stops);

        let mut routes: Vec<proto::RouteMinimal> = Vec::new();
        let mut all_lines: std::collections::HashMap<u32, proto::LineMinimal> =
            std::collections::HashMap::new();

        for (id, stops) in route_row_tree_map.iter() {
            let stops_minimal = stops
                .iter()
                .map(|row| {
                    let extracted_line = self.extract_line_from_station(row);

                    // Add line to the lines collection
                    let line_symbols = self
                        .get_line_symbols(&extracted_line)
                        .into_iter()
                        .map(|ls| proto::LineSymbol {
                            symbol: ls.symbol,
                            color: ls.color,
                            shape: ls.shape,
                        })
                        .collect();

                    let line_minimal = proto::LineMinimal {
                        id: extracted_line.line_cd as u32,
                        name_short: extracted_line.line_name,
                        color: extracted_line.line_color_c.unwrap_or_default(),
                        line_type: extracted_line.line_type.unwrap_or(0),
                        line_symbols,
                    };

                    // Update line: prefer entries with non-empty line_symbols
                    all_lines
                        .entry(line_minimal.id)
                        .and_modify(|existing| {
                            // Update if new line has symbols and existing doesn't
                            if !line_minimal.line_symbols.is_empty()
                                && existing.line_symbols.is_empty()
                            {
                                *existing = line_minimal.clone();
                            }
                        })
                        .or_insert(line_minimal);

                    // Create station minimal
                    let station_numbers = self
                        .get_station_numbers(row)
                        .into_iter()
                        .map(|sn| proto::StationNumber {
                            line_symbol: sn.line_symbol,
                            line_symbol_color: sn.line_symbol_color,
                            line_symbol_shape: sn.line_symbol_shape,
                            station_number: sn.station_number,
                        })
                        .collect();

                    proto::StationMinimal {
                        id: row.station_cd as u32,
                        group_id: row.station_g_cd as u32,
                        name: row.station_name.clone(),
                        name_katakana: row.station_name_k.clone(),
                        name_roman: row.station_name_r.clone(),
                        line_ids: vec![extracted_line.line_cd as u32],
                        station_numbers,
                        stop_condition: row.pass.unwrap_or(0),
                        has_train_types: Some(row.type_id.is_some()),
                        train_type_id: row.type_id.map(|id| id as u32),
                    }
                })
                .collect::<Vec<proto::StationMinimal>>();

            // TODO: SQLで同等の処理を行う
            let includes_requested_station = stops_minimal
                .iter()
                .any(|stop| stop.group_id == from_station_id || stop.group_id == to_station_id);
            if !includes_requested_station {
                continue;
            }

            routes.push(proto::RouteMinimal {
                id: *id as u32,
                stops: stops_minimal,
            });
        }

        Ok(proto::RouteMinimalResponse {
            routes,
            lines: all_lines.into_values().collect(),
            next_page_token: "".to_string(),
        })
    }

    async fn get_train_types(
        &self,
        from_station_id: u32,
        to_station_id: u32,
        via_line_id: Option<u32>,
    ) -> Result<Vec<TrainType>, UseCaseError> {
        let stops = self
            .station_repository
            .get_route_stops(from_station_id, to_station_id, via_line_id)
            .await?;

        let line_group_id_vec: Vec<u32> = stops
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

        let mut tt_lines = self
            .line_repository
            .get_by_line_group_id_vec(&line_group_id_vec)
            .await?;

        // Add line_symbols to all lines first
        for line in tt_lines.iter_mut() {
            line.line_symbols = self.get_line_symbols(line);
        }

        // Pre-build a map from type_cd to TrainType for O(1) lookup
        // This clones only the TrainTypes needed for embedding in Lines
        let train_type_by_type_cd: std::collections::HashMap<i32, TrainType> = train_types
            .iter()
            .filter_map(|tt| tt.type_cd.map(|cd| (cd, tt.clone())))
            .collect();

        // Track seen line_group_cds to avoid duplicates
        let mut seen_line_group_cds = HashSet::new();

        for mut train_type in train_types {
            if let Some(lgc) = train_type.line_group_cd {
                if !seen_line_group_cds.insert(lgc) {
                    continue;
                }
            } else {
                continue;
            }

            let mut seen_line_cds = HashSet::new();
            train_type.lines = tt_lines
                .iter()
                .filter(|line| {
                    line.line_group_cd == train_type.line_group_cd
                        && seen_line_cds.insert(line.line_cd)
                })
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
                    train_type: line
                        .type_cd
                        .and_then(|cd| train_type_by_type_cd.get(&cd).cloned()),
                    line_group_cd: line.line_group_cd,
                    station_cd: line.station_cd,
                    station_g_cd: line.station_g_cd,
                    type_cd: line.type_cd,
                    transport_type: line.transport_type,
                })
                .collect::<Vec<Line>>();

            // Set the line field to the first line in the lines vector
            train_type.line = train_type.lines.first().cloned().map(Box::new);

            result.push(train_type);
        }
        Ok(result)
    }

    async fn find_line_by_id(&self, line_id: u32) -> Result<Option<Line>, UseCaseError> {
        let line = self.line_repository.find_by_id(line_id).await?;
        Ok(line)
    }

    async fn get_lines_by_id_vec(&self, line_ids: &[u32]) -> Result<Vec<Line>, UseCaseError> {
        let lines = self.line_repository.get_by_ids(line_ids).await?;
        Ok(lines)
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

    // TODO: 未実装
    async fn get_connected_stations(
        &self,
        _from_station_id: u32,
        _to_station_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        Ok(vec![])
    }
}

impl<SR, LR, TR, CR> QueryInteractor<SR, LR, TR, CR>
where
    SR: StationRepository,
    LR: LineRepository,
    TR: TrainTypeRepository,
    CR: CompanyRepository,
{
    /// Get bus lines (routes) within 300m radius of the given coordinates
    async fn get_nearby_bus_lines(
        &self,
        ref_lat: f64,
        ref_lon: f64,
    ) -> Result<Vec<Line>, crate::use_case::error::UseCaseError> {
        let nearby_candidates = self
            .station_repository
            .get_by_coordinates(ref_lat, ref_lon, Some(50), Some(TransportType::Bus))
            .await?;

        let nearby_bus_stops: Vec<Station> = nearby_candidates
            .into_iter()
            .filter(|bus_stop| {
                let distance = haversine_distance(ref_lat, ref_lon, bus_stop.lat, bus_stop.lon);
                distance <= NEARBY_BUS_STOP_RADIUS_METERS
            })
            .collect();

        if nearby_bus_stops.is_empty() {
            return Ok(vec![]);
        }

        // Get bus lines for nearby bus stops
        let bus_station_group_ids: Vec<u32> = nearby_bus_stops
            .iter()
            .map(|s| s.station_g_cd as u32)
            .collect();

        let mut bus_lines = self
            .line_repository
            .get_by_station_group_id_vec(&bus_station_group_ids)
            .await?;

        // Add line symbols and filter to only bus lines
        let mut seen_line_cds = std::collections::HashSet::new();
        bus_lines.retain(|line| {
            line.transport_type == TransportType::Bus && seen_line_cds.insert(line.line_cd)
        });

        // Build HashMap for O(1) bus stop lookup by line_cd
        // Deduplicate by line_cd, keeping the first (closest) bus stop for each line
        let mut seen_bus_line_cds = std::collections::HashSet::new();
        let bus_stop_by_line_cd: std::collections::HashMap<i32, &Station> = nearby_bus_stops
            .iter()
            .filter(|s| seen_bus_line_cds.insert(s.line_cd))
            .map(|s| (s.line_cd, s))
            .collect();

        for line in bus_lines.iter_mut() {
            line.line_symbols = self.get_line_symbols(line);

            // Find the matching bus stop for this line and embed it
            if let Some(&bus_stop) = bus_stop_by_line_cd.get(&line.line_cd) {
                let mut station_copy = bus_stop.clone();
                station_copy.station_numbers = self.get_station_numbers(&station_copy);
                line.station = Some(station_copy);
            }
        }

        Ok(bus_lines)
    }

    fn build_route_tree_map<'a>(&self, stops: &'a [Station]) -> BTreeMap<i32, Vec<&'a Station>> {
        stops.iter().fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<i32, Vec<&'a Station>>, value| {
                if let Some(line_group_cd) = value.line_group_cd {
                    acc.entry(line_group_cd).or_default().push(value);
                } else {
                    acc.entry(value.line_cd).or_default().push(value);
                };
                acc
            },
        )
    }

    fn build_station_from_row(
        &self,
        row: &Station,
        extracted_line: &Line,
        train_type: Option<Box<TrainType>>,
    ) -> Station {
        Station {
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
            transport_type: row.transport_type,
        }
    }
}

/// Calculate the distance between two points on Earth using the Haversine formula.
/// Returns the distance in meters.
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_METERS * c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entity::gtfs::TransportType;
    use crate::proto::StopCondition;

    /// Helper to create a minimal Station for testing
    fn create_test_station(
        station_cd: i32,
        station_g_cd: i32,
        line_cd: i32,
        line_group_cd: Option<i32>,
    ) -> Station {
        Station {
            station_cd,
            station_g_cd,
            station_name: format!("テスト駅{}", station_cd),
            station_name_k: format!("テストエキ{}", station_cd),
            station_name_r: Some(format!("Test Station {}", station_cd)),
            station_name_zh: None,
            station_name_ko: None,
            station_numbers: vec![],
            station_number1: Some("01".to_string()),
            station_number2: None,
            station_number3: None,
            station_number4: None,
            three_letter_code: None,
            line_cd,
            line: None,
            lines: vec![],
            pref_cd: 13,
            post: "100-0001".to_string(),
            address: "東京都千代田区".to_string(),
            lon: 139.7671,
            lat: 35.6812,
            open_ymd: "19000101".to_string(),
            close_ymd: "99991231".to_string(),
            e_status: 0,
            e_sort: 1,
            stop_condition: StopCondition::All,
            distance: None,
            has_train_types: false,
            train_type: None,
            company_cd: Some(1),
            line_name: Some("テスト線".to_string()),
            line_name_k: Some("テストセン".to_string()),
            line_name_h: Some("てすとせん".to_string()),
            line_name_r: Some("Test Line".to_string()),
            line_name_zh: None,
            line_name_ko: None,
            line_color_c: Some("FF0000".to_string()),
            line_type: Some(1),
            line_symbol1: Some("T".to_string()),
            line_symbol2: None,
            line_symbol3: None,
            line_symbol4: None,
            line_symbol1_color: Some("FF0000".to_string()),
            line_symbol2_color: None,
            line_symbol3_color: None,
            line_symbol4_color: None,
            line_symbol1_shape: Some("round".to_string()),
            line_symbol2_shape: None,
            line_symbol3_shape: None,
            line_symbol4_shape: None,
            average_distance: None,
            type_id: None,
            sst_id: None,
            type_cd: None,
            line_group_cd,
            pass: None,
            type_name: None,
            type_name_k: None,
            type_name_r: None,
            type_name_zh: None,
            type_name_ko: None,
            color: None,
            direction: None,
            kind: None,
            transport_type: TransportType::Rail,
        }
    }

    /// Helper to create a minimal Line for testing
    fn create_test_line(line_cd: i32) -> Line {
        Line {
            line_cd,
            company_cd: 1,
            company: None,
            line_name: format!("テスト線{}", line_cd),
            line_name_k: format!("テストセン{}", line_cd),
            line_name_h: format!("てすとせん{}", line_cd),
            line_name_r: Some(format!("Test Line {}", line_cd)),
            line_name_zh: None,
            line_name_ko: None,
            line_color_c: Some("0000FF".to_string()),
            line_type: Some(1),
            line_symbols: vec![],
            line_symbol1: Some("L".to_string()),
            line_symbol2: Some("M".to_string()),
            line_symbol3: None,
            line_symbol4: None,
            line_symbol1_color: Some("0000FF".to_string()),
            line_symbol2_color: Some("00FF00".to_string()),
            line_symbol3_color: None,
            line_symbol4_color: None,
            line_symbol1_shape: Some("square".to_string()),
            line_symbol2_shape: Some("circle".to_string()),
            line_symbol3_shape: None,
            line_symbol4_shape: None,
            e_status: 0,
            e_sort: 1,
            average_distance: None,
            station: None,
            train_type: None,
            line_group_cd: None,
            station_cd: None,
            station_g_cd: None,
            type_cd: None,
            transport_type: TransportType::Rail,
        }
    }

    // ========================================
    // haversine_distance tests
    // ========================================

    #[test]
    fn test_haversine_distance_same_point() {
        let distance = haversine_distance(35.6812, 139.7671, 35.6812, 139.7671);
        assert!((distance - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_haversine_distance_tokyo_to_osaka() {
        // Tokyo Station: 35.6812, 139.7671
        // Osaka Station: 34.7024, 135.4959
        let distance = haversine_distance(35.6812, 139.7671, 34.7024, 135.4959);
        // Expected: approximately 400km
        assert!(distance > 390_000.0 && distance < 410_000.0);
    }

    #[test]
    fn test_haversine_distance_short_distance() {
        // Two points ~100m apart
        let distance = haversine_distance(35.6812, 139.7671, 35.6820, 139.7671);
        assert!(distance > 80.0 && distance < 120.0);
    }

    #[test]
    fn test_haversine_distance_within_bus_stop_radius() {
        // Points within 300m radius
        let distance = haversine_distance(35.6812, 139.7671, 35.6835, 139.7671);
        assert!(distance <= NEARBY_BUS_STOP_RADIUS_METERS);
    }

    #[test]
    fn test_haversine_distance_outside_bus_stop_radius() {
        // Points outside 300m radius
        let distance = haversine_distance(35.6812, 139.7671, 35.6900, 139.7671);
        assert!(distance > NEARBY_BUS_STOP_RADIUS_METERS);
    }

    // ========================================
    // build_route_tree_map tests
    // ========================================

    mod build_route_tree_map_tests {
        use super::*;
        use crate::domain::{
            error::DomainError,
            repository::{
                company_repository::CompanyRepository, line_repository::LineRepository,
                station_repository::StationRepository, train_type_repository::TrainTypeRepository,
            },
        };

        // Mock repositories for testing
        struct MockStationRepository;
        struct MockLineRepository;
        struct MockTrainTypeRepository;
        struct MockCompanyRepository;

        #[async_trait::async_trait]
        impl StationRepository for MockStationRepository {
            async fn find_by_id(&self, _: u32) -> Result<Option<Station>, DomainError> {
                Ok(None)
            }
            async fn get_by_id_vec(&self, _: &[u32]) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_line_id(
                &self,
                _: u32,
                _: Option<u32>,
                _: Option<u32>,
            ) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_group_id(&self, _: u32) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_group_id_vec(
                &self,
                _: &[u32],
            ) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_coordinates(
                &self,
                _: f64,
                _: f64,
                _: Option<u32>,
                _: Option<TransportType>,
            ) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_name(
                &self,
                _: String,
                _: Option<u32>,
                _: Option<u32>,
                _: Option<TransportType>,
            ) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_line_group_id(&self, _: u32) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_route_stops(
                &self,
                _: u32,
                _: u32,
                _: Option<u32>,
            ) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
        }

        #[async_trait::async_trait]
        impl LineRepository for MockLineRepository {
            async fn find_by_id(&self, _: u32) -> Result<Option<Line>, DomainError> {
                Ok(None)
            }
            async fn find_by_station_id(&self, _: u32) -> Result<Option<Line>, DomainError> {
                Ok(None)
            }
            async fn get_by_ids(&self, _: &[u32]) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_name(
                &self,
                _: String,
                _: Option<u32>,
            ) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_line_group_id(&self, _: u32) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_line_group_id_vec(&self, _: &[u32]) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_line_group_id_vec_for_routes(
                &self,
                _: &[u32],
            ) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_group_id(&self, _: u32) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_group_id_vec(
                &self,
                _: &[u32],
            ) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
        }

        #[async_trait::async_trait]
        impl TrainTypeRepository for MockTrainTypeRepository {
            async fn find_by_line_group_id_and_line_id(
                &self,
                _: u32,
                _: u32,
            ) -> Result<Option<TrainType>, DomainError> {
                Ok(None)
            }
            async fn get_by_line_group_id(&self, _: u32) -> Result<Vec<TrainType>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_id(&self, _: u32) -> Result<Vec<TrainType>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_id_vec(
                &self,
                _: &[u32],
                _: Option<u32>,
            ) -> Result<Vec<TrainType>, DomainError> {
                Ok(vec![])
            }
            async fn get_types_by_station_id_vec(
                &self,
                _: &[u32],
                _: Option<u32>,
            ) -> Result<Vec<TrainType>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_line_group_id_vec(
                &self,
                _: &[u32],
            ) -> Result<Vec<TrainType>, DomainError> {
                Ok(vec![])
            }
        }

        #[async_trait::async_trait]
        impl CompanyRepository for MockCompanyRepository {
            async fn find_by_id_vec(&self, _: &[u32]) -> Result<Vec<Company>, DomainError> {
                Ok(vec![])
            }
        }

        fn create_interactor() -> QueryInteractor<
            MockStationRepository,
            MockLineRepository,
            MockTrainTypeRepository,
            MockCompanyRepository,
        > {
            QueryInteractor {
                station_repository: MockStationRepository,
                line_repository: MockLineRepository,
                train_type_repository: MockTrainTypeRepository,
                company_repository: MockCompanyRepository,
            }
        }

        #[test]
        fn test_build_route_tree_map_empty() {
            let interactor = create_interactor();
            let stops: Vec<Station> = vec![];
            let result = interactor.build_route_tree_map(&stops);
            assert!(result.is_empty());
        }

        #[test]
        fn test_build_route_tree_map_groups_by_line_group_cd() {
            let interactor = create_interactor();
            let stops = vec![
                create_test_station(1, 1, 100, Some(1000)),
                create_test_station(2, 2, 100, Some(1000)),
                create_test_station(3, 3, 200, Some(2000)),
            ];
            let result = interactor.build_route_tree_map(&stops);

            assert_eq!(result.len(), 2);
            assert_eq!(result.get(&1000).unwrap().len(), 2);
            assert_eq!(result.get(&2000).unwrap().len(), 1);
        }

        #[test]
        fn test_build_route_tree_map_groups_by_line_cd_when_no_line_group() {
            let interactor = create_interactor();
            let stops = vec![
                create_test_station(1, 1, 100, None),
                create_test_station(2, 2, 100, None),
                create_test_station(3, 3, 200, None),
            ];
            let result = interactor.build_route_tree_map(&stops);

            assert_eq!(result.len(), 2);
            assert_eq!(result.get(&100).unwrap().len(), 2);
            assert_eq!(result.get(&200).unwrap().len(), 1);
        }

        #[test]
        fn test_build_route_tree_map_returns_references() {
            let interactor = create_interactor();
            let stops = vec![create_test_station(1, 1, 100, Some(1000))];
            let result = interactor.build_route_tree_map(&stops);

            // Verify that the result contains references to the original stations
            let grouped_stops = result.get(&1000).unwrap();
            assert_eq!(grouped_stops[0].station_cd, 1);
            // Verify it's a reference by checking pointer equality
            assert!(std::ptr::eq(grouped_stops[0], &stops[0]));
        }

        // ========================================
        // get_station_numbers tests
        // ========================================

        #[test]
        fn test_get_station_numbers_single_number() {
            let interactor = create_interactor();
            let station = create_test_station(1, 1, 100, None);
            let numbers = interactor.get_station_numbers(&station);

            assert_eq!(numbers.len(), 1);
            assert_eq!(numbers[0].station_number, "T-01");
            assert_eq!(numbers[0].line_symbol, "T");
            assert_eq!(numbers[0].line_symbol_color, "FF0000");
            assert_eq!(numbers[0].line_symbol_shape, "round");
        }

        #[test]
        fn test_get_station_numbers_multiple_numbers() {
            let interactor = create_interactor();
            let mut station = create_test_station(1, 1, 100, None);
            station.station_number2 = Some("02".to_string());
            station.line_symbol2 = Some("M".to_string());
            station.line_symbol2_color = Some("00FF00".to_string());
            station.line_symbol2_shape = Some("square".to_string());

            let numbers = interactor.get_station_numbers(&station);

            assert_eq!(numbers.len(), 2);
            assert_eq!(numbers[0].station_number, "T-01");
            assert_eq!(numbers[1].station_number, "M-02");
        }

        #[test]
        fn test_get_station_numbers_empty_when_no_numbers() {
            let interactor = create_interactor();
            let mut station = create_test_station(1, 1, 100, None);
            station.station_number1 = None;

            let numbers = interactor.get_station_numbers(&station);
            assert!(numbers.is_empty());
        }

        #[test]
        fn test_get_station_numbers_without_symbol() {
            let interactor = create_interactor();
            let mut station = create_test_station(1, 1, 100, None);
            station.line_symbol1 = None;

            let numbers = interactor.get_station_numbers(&station);

            assert_eq!(numbers.len(), 1);
            assert_eq!(numbers[0].station_number, "01");
            assert_eq!(numbers[0].line_symbol, "");
        }

        // ========================================
        // get_line_symbols tests
        // ========================================

        #[test]
        fn test_get_line_symbols_multiple_symbols() {
            let interactor = create_interactor();
            let line = create_test_line(100);
            let symbols = interactor.get_line_symbols(&line);

            assert_eq!(symbols.len(), 2);
            assert_eq!(symbols[0].symbol, "L");
            assert_eq!(symbols[0].color, "0000FF");
            assert_eq!(symbols[0].shape, "square");
            assert_eq!(symbols[1].symbol, "M");
            assert_eq!(symbols[1].color, "00FF00");
            assert_eq!(symbols[1].shape, "circle");
        }

        #[test]
        fn test_get_line_symbols_uses_line_color_as_fallback() {
            let interactor = create_interactor();
            let mut line = create_test_line(100);
            line.line_symbol1_color = None;

            let symbols = interactor.get_line_symbols(&line);

            // First symbol should use line_color_c as fallback
            assert_eq!(symbols[0].color, "0000FF");
        }

        #[test]
        fn test_get_line_symbols_empty_when_no_symbols() {
            let interactor = create_interactor();
            let mut line = create_test_line(100);
            line.line_symbol1 = None;
            line.line_symbol2 = None;
            line.line_symbol3 = None;

            let symbols = interactor.get_line_symbols(&line);
            assert!(symbols.is_empty());
        }

        // ========================================
        // extract_line_from_station tests
        // ========================================

        #[test]
        fn test_extract_line_from_station() {
            let interactor = create_interactor();
            let station = create_test_station(1, 1, 100, Some(1000));
            let line = interactor.extract_line_from_station(&station);

            assert_eq!(line.line_cd, 100);
            assert_eq!(line.company_cd, 1);
            assert_eq!(line.line_name, "テスト線");
            assert_eq!(line.line_name_k, "テストセン");
            assert_eq!(line.line_color_c, Some("FF0000".to_string()));
            assert_eq!(line.line_symbol1, Some("T".to_string()));
            assert!(line.line_symbols.is_empty()); // symbols are added separately
            assert!(line.company.is_none());
            assert!(line.station.is_none());
        }

        #[test]
        fn test_extract_line_from_station_preserves_station_cd() {
            let interactor = create_interactor();
            let station = create_test_station(42, 42, 100, None);
            let line = interactor.extract_line_from_station(&station);

            assert_eq!(line.station_cd, Some(42));
            assert_eq!(line.station_g_cd, Some(42));
        }

        #[test]
        fn test_extract_line_from_station_handles_missing_optional_fields() {
            let interactor = create_interactor();
            let mut station = create_test_station(1, 1, 100, None);
            station.line_name = None;
            station.line_name_k = None;
            station.line_name_h = None;
            station.company_cd = None;

            let line = interactor.extract_line_from_station(&station);

            assert_eq!(line.line_name, "");
            assert_eq!(line.line_name_k, "");
            assert_eq!(line.line_name_h, "");
            assert_eq!(line.company_cd, 0);
        }
    }

    // ========================================
    // update_station_vec_with_attributes tests
    // ========================================

    mod update_station_vec_with_attributes_tests {
        use super::*;
        use crate::domain::{
            entity::company::Company,
            error::DomainError,
            repository::{
                company_repository::CompanyRepository, line_repository::LineRepository,
                station_repository::StationRepository, train_type_repository::TrainTypeRepository,
            },
        };

        /// Configurable mock station repository for testing
        struct ConfigurableMockStationRepository {
            stations_by_group: Vec<Station>,
            bus_stops: Vec<Station>,
        }

        impl ConfigurableMockStationRepository {
            fn new(stations_by_group: Vec<Station>, bus_stops: Vec<Station>) -> Self {
                Self {
                    stations_by_group,
                    bus_stops,
                }
            }
        }

        #[async_trait::async_trait]
        impl StationRepository for ConfigurableMockStationRepository {
            async fn find_by_id(&self, _: u32) -> Result<Option<Station>, DomainError> {
                Ok(None)
            }
            async fn get_by_id_vec(&self, _: &[u32]) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_line_id(
                &self,
                _: u32,
                _: Option<u32>,
                _: Option<u32>,
            ) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_group_id(&self, _: u32) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_group_id_vec(
                &self,
                _: &[u32],
            ) -> Result<Vec<Station>, DomainError> {
                Ok(self.stations_by_group.clone())
            }
            async fn get_by_coordinates(
                &self,
                _: f64,
                _: f64,
                _: Option<u32>,
                transport_type: Option<TransportType>,
            ) -> Result<Vec<Station>, DomainError> {
                // Return bus stops only when searching for Bus transport type
                if transport_type == Some(TransportType::Bus) {
                    Ok(self.bus_stops.clone())
                } else {
                    Ok(vec![])
                }
            }
            async fn get_by_name(
                &self,
                _: String,
                _: Option<u32>,
                _: Option<u32>,
                _: Option<TransportType>,
            ) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_line_group_id(&self, _: u32) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
            async fn get_route_stops(
                &self,
                _: u32,
                _: u32,
                _: Option<u32>,
            ) -> Result<Vec<Station>, DomainError> {
                Ok(vec![])
            }
        }

        /// Configurable mock line repository for testing
        struct ConfigurableMockLineRepository {
            lines_by_station_group: Vec<Line>,
        }

        impl ConfigurableMockLineRepository {
            fn new(lines_by_station_group: Vec<Line>) -> Self {
                Self {
                    lines_by_station_group,
                }
            }
        }

        #[async_trait::async_trait]
        impl LineRepository for ConfigurableMockLineRepository {
            async fn find_by_id(&self, _: u32) -> Result<Option<Line>, DomainError> {
                Ok(None)
            }
            async fn find_by_station_id(&self, _: u32) -> Result<Option<Line>, DomainError> {
                Ok(None)
            }
            async fn get_by_ids(&self, _: &[u32]) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_name(
                &self,
                _: String,
                _: Option<u32>,
            ) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_line_group_id(&self, _: u32) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_line_group_id_vec(&self, _: &[u32]) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_line_group_id_vec_for_routes(
                &self,
                _: &[u32],
            ) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_group_id(&self, _: u32) -> Result<Vec<Line>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_group_id_vec(
                &self,
                _: &[u32],
            ) -> Result<Vec<Line>, DomainError> {
                Ok(self.lines_by_station_group.clone())
            }
        }

        /// Configurable mock train type repository for testing
        struct ConfigurableMockTrainTypeRepository {
            train_types: Vec<TrainType>,
        }

        impl ConfigurableMockTrainTypeRepository {
            fn new(train_types: Vec<TrainType>) -> Self {
                Self { train_types }
            }
        }

        #[async_trait::async_trait]
        impl TrainTypeRepository for ConfigurableMockTrainTypeRepository {
            async fn find_by_line_group_id_and_line_id(
                &self,
                _: u32,
                _: u32,
            ) -> Result<Option<TrainType>, DomainError> {
                Ok(None)
            }
            async fn get_by_line_group_id(&self, _: u32) -> Result<Vec<TrainType>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_id(&self, _: u32) -> Result<Vec<TrainType>, DomainError> {
                Ok(vec![])
            }
            async fn get_by_station_id_vec(
                &self,
                _: &[u32],
                _: Option<u32>,
            ) -> Result<Vec<TrainType>, DomainError> {
                Ok(vec![])
            }
            async fn get_types_by_station_id_vec(
                &self,
                _: &[u32],
                _: Option<u32>,
            ) -> Result<Vec<TrainType>, DomainError> {
                Ok(self.train_types.clone())
            }
            async fn get_by_line_group_id_vec(
                &self,
                _: &[u32],
            ) -> Result<Vec<TrainType>, DomainError> {
                Ok(vec![])
            }
        }

        /// Configurable mock company repository for testing
        struct ConfigurableMockCompanyRepository {
            companies: Vec<Company>,
        }

        impl ConfigurableMockCompanyRepository {
            fn new(companies: Vec<Company>) -> Self {
                Self { companies }
            }
        }

        #[async_trait::async_trait]
        impl CompanyRepository for ConfigurableMockCompanyRepository {
            async fn find_by_id_vec(&self, _: &[u32]) -> Result<Vec<Company>, DomainError> {
                Ok(self.companies.clone())
            }
        }

        /// Helper to create a test company
        fn create_test_company(company_cd: i32, name: &str) -> Company {
            Company::new(
                company_cd,
                1,
                name.to_string(),
                format!("{name}カナ"),
                format!("{name}ひらがな"),
                format!("{name} Romaji"),
                format!("{name} EN"),
                format!("{name} Full EN"),
                Some(format!("https://{}.example.com", name.to_lowercase())),
                1,
                0,
                1,
            )
        }

        /// Helper to create a test train type
        fn create_test_train_type_for_station(station_cd: i32, type_name: &str) -> TrainType {
            TrainType::new(
                Some(station_cd * 10), // id
                Some(station_cd),      // station_cd
                Some(1),               // type_cd
                Some(1000),            // line_group_cd
                Some(0),               // pass (not passing)
                type_name.to_string(),
                format!("{type_name}カナ"),
                Some(format!("{type_name} EN")),
                None,
                None,
                "#FF0000".to_string(),
                Some(0),
                Some(1),
            )
        }

        /// Helper to create a test line for station group
        fn create_test_line_for_station_group(line_cd: i32, station_g_cd: i32) -> Line {
            let mut line = create_test_line(line_cd);
            line.station_g_cd = Some(station_g_cd);
            line
        }

        /// Helper to create a bus stop station
        fn create_bus_stop(station_cd: i32, lat: f64, lon: f64, line_cd: i32) -> Station {
            let mut station = create_test_station(station_cd, station_cd, line_cd, None);
            station.lat = lat;
            station.lon = lon;
            station.transport_type = TransportType::Bus;
            station.line_name = Some("テストバス路線".to_string());
            station.line_type = Some(3); // bus type
            station
        }

        fn create_configurable_interactor(
            stations_by_group: Vec<Station>,
            bus_stops: Vec<Station>,
            lines: Vec<Line>,
            train_types: Vec<TrainType>,
            companies: Vec<Company>,
        ) -> QueryInteractor<
            ConfigurableMockStationRepository,
            ConfigurableMockLineRepository,
            ConfigurableMockTrainTypeRepository,
            ConfigurableMockCompanyRepository,
        > {
            QueryInteractor {
                station_repository: ConfigurableMockStationRepository::new(
                    stations_by_group,
                    bus_stops,
                ),
                line_repository: ConfigurableMockLineRepository::new(lines),
                train_type_repository: ConfigurableMockTrainTypeRepository::new(train_types),
                company_repository: ConfigurableMockCompanyRepository::new(companies),
            }
        }

        #[tokio::test]
        async fn test_update_station_vec_with_attributes_enriches_company_info() {
            // Create test data
            let company1 = create_test_company(1, "JR東日本");
            let company2 = create_test_company(2, "東京メトロ");

            let mut station1 = create_test_station(101, 1001, 100, Some(1000));
            station1.company_cd = Some(1);

            let mut station2 = create_test_station(102, 1002, 200, Some(2000));
            station2.company_cd = Some(2);

            let line1 = create_test_line_for_station_group(100, 1001);
            let line2 = create_test_line_for_station_group(200, 1002);

            let interactor = create_configurable_interactor(
                vec![station1.clone(), station2.clone()],
                vec![],
                vec![line1, line2],
                vec![],
                vec![company1.clone(), company2.clone()],
            );

            let stations = vec![station1, station2];
            let result = interactor
                .update_station_vec_with_attributes(stations, None, TransportTypeFilter::Rail)
                .await
                .expect("Should succeed");

            assert_eq!(result.len(), 2);

            // Check that station1's line has the correct company
            let station1_result = &result[0];
            assert!(station1_result.line.is_some());
            let line1_result = station1_result.line.as_ref().unwrap();
            assert!(line1_result.company.is_some());
            assert_eq!(line1_result.company.as_ref().unwrap().company_cd, 1);
            assert_eq!(
                line1_result.company.as_ref().unwrap().company_name,
                "JR東日本"
            );

            // Check that station2's line has the correct company
            let station2_result = &result[1];
            assert!(station2_result.line.is_some());
            let line2_result = station2_result.line.as_ref().unwrap();
            assert!(line2_result.company.is_some());
            assert_eq!(line2_result.company.as_ref().unwrap().company_cd, 2);
            assert_eq!(
                line2_result.company.as_ref().unwrap().company_name,
                "東京メトロ"
            );
        }

        #[tokio::test]
        async fn test_update_station_vec_with_attributes_enriches_train_type_info() {
            let company = create_test_company(1, "JR東日本");
            let train_type = create_test_train_type_for_station(101, "快速");

            let mut station = create_test_station(101, 1001, 100, Some(1000));
            station.company_cd = Some(1);

            let line = create_test_line_for_station_group(100, 1001);

            let interactor = create_configurable_interactor(
                vec![station.clone()],
                vec![],
                vec![line],
                vec![train_type.clone()],
                vec![company],
            );

            let stations = vec![station];
            let result = interactor
                .update_station_vec_with_attributes(stations, None, TransportTypeFilter::Rail)
                .await
                .expect("Should succeed");

            assert_eq!(result.len(), 1);

            let station_result = &result[0];
            assert!(station_result.train_type.is_some());
            let train_type_result = station_result.train_type.as_ref().unwrap();
            assert_eq!(train_type_result.type_name, "快速");
            assert_eq!(train_type_result.station_cd, Some(101));
        }

        #[tokio::test]
        async fn test_update_station_vec_with_attributes_adds_line_symbols() {
            let company = create_test_company(1, "JR東日本");

            let mut station = create_test_station(101, 1001, 100, Some(1000));
            station.company_cd = Some(1);

            let line = create_test_line_for_station_group(100, 1001);

            let interactor = create_configurable_interactor(
                vec![station.clone()],
                vec![],
                vec![line],
                vec![],
                vec![company],
            );

            let stations = vec![station];
            let result = interactor
                .update_station_vec_with_attributes(stations, None, TransportTypeFilter::Rail)
                .await
                .expect("Should succeed");

            assert_eq!(result.len(), 1);

            let station_result = &result[0];
            assert!(station_result.line.is_some());
            let line_result = station_result.line.as_ref().unwrap();

            // Line symbols should be populated via get_line_symbols
            assert!(!line_result.line_symbols.is_empty());
            assert_eq!(line_result.line_symbols[0].symbol, "T");
            assert_eq!(line_result.line_symbols[0].color, "FF0000");
            assert_eq!(line_result.line_symbols[0].shape, "round");
        }

        #[tokio::test]
        async fn test_update_station_vec_with_attributes_adds_nearby_bus_routes_for_rail() {
            let company = create_test_company(1, "JR東日本");
            let bus_company = create_test_company(100, "都営バス");

            // Rail station at Tokyo Station coordinates
            let mut rail_station = create_test_station(101, 1001, 100, Some(1000));
            rail_station.company_cd = Some(1);
            rail_station.lat = 35.6812;
            rail_station.lon = 139.7671;
            rail_station.transport_type = TransportType::Rail;

            // Bus stop within 300m radius (approximately 200m north)
            let bus_stop = create_bus_stop(
                201, 35.6830, // ~200m north of rail station
                139.7671, 500,
            );

            let rail_line = create_test_line_for_station_group(100, 1001);

            // Create a bus line that matches the bus stop's station_g_cd
            let mut bus_line = create_test_line(500);
            bus_line.station_g_cd = Some(201); // Matches bus_stop.station_g_cd
            bus_line.transport_type = TransportType::Bus;
            bus_line.line_name = "テストバス路線".to_string();

            let interactor = create_configurable_interactor(
                vec![rail_station.clone()],
                vec![bus_stop],
                vec![rail_line, bus_line], // Include both rail and bus lines
                vec![],
                vec![company, bus_company],
            );

            let stations = vec![rail_station];
            // Bus routes are added only when transport_type is RailAndBus
            let result = interactor
                .update_station_vec_with_attributes(stations, None, TransportTypeFilter::RailAndBus)
                .await
                .expect("Should succeed");

            assert_eq!(result.len(), 1);

            let station_result = &result[0];
            // Should have the rail line plus nearby bus routes in lines array
            assert!(!station_result.lines.is_empty());

            // Check that bus line was added (line_cd 500 from the bus stop)
            let has_bus_line = station_result
                .lines
                .iter()
                .any(|l| l.transport_type == TransportType::Bus);
            assert!(
                has_bus_line,
                "Rail station should have nearby bus routes added when transport_type is RailAndBus"
            );
        }

        #[tokio::test]
        async fn test_update_station_vec_with_attributes_no_bus_routes_when_transport_type_is_rail()
        {
            let company = create_test_company(1, "JR東日本");

            // Rail station
            let mut rail_station = create_test_station(101, 1001, 100, Some(1000));
            rail_station.company_cd = Some(1);
            rail_station.lat = 35.6812;
            rail_station.lon = 139.7671;
            rail_station.transport_type = TransportType::Rail;

            // Bus stop nearby
            let bus_stop = create_bus_stop(201, 35.6830, 139.7671, 500);

            let rail_line = create_test_line_for_station_group(100, 1001);

            let mut bus_line = create_test_line(500);
            bus_line.station_g_cd = Some(201);
            bus_line.transport_type = TransportType::Bus;

            let interactor = create_configurable_interactor(
                vec![rail_station.clone()],
                vec![bus_stop],
                vec![rail_line, bus_line],
                vec![],
                vec![company],
            );

            // When transport_type is Rail, bus routes should NOT be added
            let stations = vec![rail_station];
            let result = interactor
                .update_station_vec_with_attributes(stations, None, TransportTypeFilter::Rail)
                .await
                .expect("Should succeed");

            assert_eq!(result.len(), 1);

            let station_result = &result[0];
            // Should NOT have bus lines when transport_type is Rail
            let has_bus_line = station_result
                .lines
                .iter()
                .any(|l| l.transport_type == TransportType::Bus);
            assert!(
                !has_bus_line,
                "Should not add bus routes when transport_type is Rail"
            );
        }

        #[tokio::test]
        async fn test_update_station_vec_with_attributes_missing_company_key_no_panic() {
            // Station references company_cd that doesn't exist in the map
            let mut station = create_test_station(101, 1001, 100, Some(1000));
            station.company_cd = Some(999); // Company ID that doesn't exist

            let line = create_test_line_for_station_group(100, 1001);

            // Empty companies list - company_cd 999 won't be found
            let interactor = create_configurable_interactor(
                vec![station.clone()],
                vec![],
                vec![line],
                vec![],
                vec![], // No companies
            );

            let stations = vec![station];
            let result = interactor
                .update_station_vec_with_attributes(stations, None, TransportTypeFilter::Rail)
                .await
                .expect("Should succeed even with missing company");

            assert_eq!(result.len(), 1);

            // Company should be None (default) when not found in map
            let station_result = &result[0];
            assert!(station_result.line.is_some());
            let line_result = station_result.line.as_ref().unwrap();
            assert!(
                line_result.company.is_none(),
                "Company should be None when not found in map"
            );
        }

        #[tokio::test]
        async fn test_update_station_vec_with_attributes_missing_train_type_key_no_panic() {
            let company = create_test_company(1, "JR東日本");

            let mut station = create_test_station(101, 1001, 100, Some(1000));
            station.company_cd = Some(1);

            let line = create_test_line_for_station_group(100, 1001);

            // Train type for a different station_cd
            let train_type = create_test_train_type_for_station(999, "快速"); // Different station_cd

            let interactor = create_configurable_interactor(
                vec![station.clone()],
                vec![],
                vec![line],
                vec![train_type],
                vec![company],
            );

            let stations = vec![station];
            let result = interactor
                .update_station_vec_with_attributes(stations, None, TransportTypeFilter::Rail)
                .await
                .expect("Should succeed even with missing train type");

            assert_eq!(result.len(), 1);

            // Train type should be None (default) when station_cd doesn't match
            let station_result = &result[0];
            assert!(
                station_result.train_type.is_none(),
                "Train type should be None when station_cd doesn't match"
            );
        }

        #[tokio::test]
        async fn test_update_station_vec_with_attributes_empty_input() {
            let interactor = create_configurable_interactor(vec![], vec![], vec![], vec![], vec![]);

            let stations: Vec<Station> = vec![];
            let result = interactor
                .update_station_vec_with_attributes(stations, None, TransportTypeFilter::Rail)
                .await
                .expect("Should succeed with empty input");

            assert!(result.is_empty());
        }

        #[tokio::test]
        async fn test_update_station_vec_with_attributes_multiple_stations_same_line() {
            let company = create_test_company(1, "JR東日本");
            let train_type1 = create_test_train_type_for_station(101, "快速");
            let train_type2 = create_test_train_type_for_station(102, "各停");

            let mut station1 = create_test_station(101, 1001, 100, Some(1000));
            station1.company_cd = Some(1);

            let mut station2 = create_test_station(102, 1001, 100, Some(1000)); // Same station_g_cd
            station2.company_cd = Some(1);

            let line = create_test_line_for_station_group(100, 1001);

            let interactor = create_configurable_interactor(
                vec![station1.clone(), station2.clone()],
                vec![],
                vec![line],
                vec![train_type1, train_type2],
                vec![company.clone()],
            );

            let stations = vec![station1, station2];
            let result = interactor
                .update_station_vec_with_attributes(stations, None, TransportTypeFilter::Rail)
                .await
                .expect("Should succeed");

            assert_eq!(result.len(), 2);

            // Both stations should have the same company
            for station_result in &result {
                assert!(station_result.line.is_some());
                let line_result = station_result.line.as_ref().unwrap();
                assert!(line_result.company.is_some());
                assert_eq!(line_result.company.as_ref().unwrap().company_cd, 1);
            }

            // Each station should have its own train type
            assert!(result[0].train_type.is_some());
            assert_eq!(result[0].train_type.as_ref().unwrap().type_name, "快速");
            assert!(result[1].train_type.is_some());
            assert_eq!(result[1].train_type.as_ref().unwrap().type_name, "各停");
        }

        #[tokio::test]
        async fn test_update_station_vec_with_attributes_station_numbers_populated() {
            let company = create_test_company(1, "JR東日本");

            let mut station = create_test_station(101, 1001, 100, Some(1000));
            station.company_cd = Some(1);
            station.station_number1 = Some("01".to_string());
            station.station_number2 = Some("02".to_string());
            station.line_symbol1 = Some("JY".to_string());
            station.line_symbol2 = Some("JK".to_string());
            station.line_symbol1_color = Some("00B261".to_string());
            station.line_symbol2_color = Some("0078BA".to_string());
            station.line_symbol1_shape = Some("round".to_string());
            station.line_symbol2_shape = Some("round".to_string());

            let line = create_test_line_for_station_group(100, 1001);

            let interactor = create_configurable_interactor(
                vec![station.clone()],
                vec![],
                vec![line],
                vec![],
                vec![company],
            );

            let stations = vec![station];
            let result = interactor
                .update_station_vec_with_attributes(stations, None, TransportTypeFilter::Rail)
                .await
                .expect("Should succeed");

            assert_eq!(result.len(), 1);

            let station_result = &result[0];
            assert_eq!(station_result.station_numbers.len(), 2);
            assert_eq!(station_result.station_numbers[0].station_number, "JY-01");
            assert_eq!(station_result.station_numbers[0].line_symbol, "JY");
            assert_eq!(station_result.station_numbers[1].station_number, "JK-02");
            assert_eq!(station_result.station_numbers[1].line_symbol, "JK");
        }

        #[tokio::test]
        async fn test_update_station_vec_with_attributes_lines_array_populated() {
            let company = create_test_company(1, "JR東日本");

            let mut station = create_test_station(101, 1001, 100, Some(1000));
            station.company_cd = Some(1);

            // Multiple lines associated with the same station group
            let mut line1 = create_test_line_for_station_group(100, 1001);
            line1.line_name = "山手線".to_string();
            let mut line2 = create_test_line_for_station_group(200, 1001);
            line2.line_name = "京浜東北線".to_string();

            let interactor = create_configurable_interactor(
                vec![station.clone()],
                vec![],
                vec![line1, line2],
                vec![],
                vec![company],
            );

            let stations = vec![station];
            let result = interactor
                .update_station_vec_with_attributes(stations, None, TransportTypeFilter::Rail)
                .await
                .expect("Should succeed");

            assert_eq!(result.len(), 1);

            let station_result = &result[0];
            // Should have both lines in the lines array
            assert_eq!(station_result.lines.len(), 2);

            let line_names: Vec<&str> = station_result
                .lines
                .iter()
                .map(|l| l.line_name.as_str())
                .collect();
            assert!(line_names.contains(&"山手線"));
            assert!(line_names.contains(&"京浜東北線"));

            // Each line in the array should have company and line_symbols populated
            for line in &station_result.lines {
                assert!(line.company.is_some());
                assert!(!line.line_symbols.is_empty());
            }
        }
    }

    // ========================================
    // matches_transport_filter tests
    // ========================================

    mod matches_transport_filter_tests {
        use super::*;

        #[test]
        fn test_rail_filter_matches_rail_station() {
            assert!(matches_transport_filter(
                TransportType::Rail,
                TransportTypeFilter::Rail
            ));
        }

        #[test]
        fn test_rail_filter_rejects_bus_station() {
            assert!(!matches_transport_filter(
                TransportType::Bus,
                TransportTypeFilter::Rail
            ));
        }

        #[test]
        fn test_bus_filter_matches_bus_station() {
            assert!(matches_transport_filter(
                TransportType::Bus,
                TransportTypeFilter::Bus
            ));
        }

        #[test]
        fn test_bus_filter_rejects_rail_station() {
            assert!(!matches_transport_filter(
                TransportType::Rail,
                TransportTypeFilter::Bus
            ));
        }

        #[test]
        fn test_rail_and_bus_filter_matches_rail_station() {
            assert!(matches_transport_filter(
                TransportType::Rail,
                TransportTypeFilter::RailAndBus
            ));
        }

        #[test]
        fn test_rail_and_bus_filter_matches_bus_station() {
            assert!(matches_transport_filter(
                TransportType::Bus,
                TransportTypeFilter::RailAndBus
            ));
        }
    }

    // ========================================
    // filter_to_db_type tests
    // ========================================

    mod filter_to_db_type_tests {
        use super::*;

        #[test]
        fn test_rail_filter_returns_some_rail() {
            assert_eq!(
                filter_to_db_type(TransportTypeFilter::Rail),
                Some(TransportType::Rail)
            );
        }

        #[test]
        fn test_bus_filter_returns_some_bus() {
            assert_eq!(
                filter_to_db_type(TransportTypeFilter::Bus),
                Some(TransportType::Bus)
            );
        }

        #[test]
        fn test_rail_and_bus_filter_returns_none() {
            assert_eq!(filter_to_db_type(TransportTypeFilter::RailAndBus), None);
        }
    }
}
