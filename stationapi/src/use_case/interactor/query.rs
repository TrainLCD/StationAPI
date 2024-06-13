use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::{
    domain::{
        entity::{
            company::Company, line::Line, line_symbol::LineSymbol, misc::StationIdWithDistance,
            station::Station, station_number::StationNumber, train_type::TrainType,
        },
        repository::{
            company_repository::CompanyRepository, line_repository::LineRepository,
            routes_repository::RoutesRepository, station_repository::StationRepository,
            train_type_repository::TrainTypeRepository,
        },
    },
    infrastructure::routes_repository::RouteRow,
    station_api::Route as ApiRoute,
    use_case::{error::UseCaseError, traits::query::QueryUseCase},
};

#[derive(Clone)]
pub struct QueryInteractor<SR, LR, TR, CR, RR> {
    pub station_repository: SR,
    pub line_repository: LR,
    pub train_type_repository: TR,
    pub company_repository: CR,
    pub routes_repository: RR,
}

#[async_trait]
impl<SR, LR, TR, CR, RR> QueryUseCase for QueryInteractor<SR, LR, TR, CR, RR>
where
    SR: StationRepository,
    LR: LineRepository,
    TR: TrainTypeRepository,
    CR: CompanyRepository,
    RR: RoutesRepository,
{
    async fn find_station_by_id(&self, station_id: i32) -> Result<Option<Station>, UseCaseError> {
        let Some(station) = self.station_repository.find_by_id(station_id).await? else {
            return Ok(None);
        };
        let result_vec = &mut vec![station];
        self.update_station_vec_with_attributes(result_vec, None)
            .await?;
        let station = result_vec.first().cloned();

        Ok(station)
    }
    async fn get_stations_by_id_vec(
        &self,
        station_ids: Vec<i32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let mut stations = self.station_repository.get_by_id_vec(station_ids).await?;
        self.update_station_vec_with_attributes(&mut stations, None)
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_group_id(
        &self,
        station_group_id: i32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let mut stations = self
            .station_repository
            .get_by_station_group_id(station_group_id)
            .await?;

        self.update_station_vec_with_attributes(&mut stations, Some(station_group_id))
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_group_id_vec(
        &self,
        station_group_id_vec: Vec<i32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_station_group_id_vec(station_group_id_vec)
            .await?;

        Ok(stations)
    }
    async fn get_lines_by_station_group_id_vec(
        &self,
        station_group_id_vec: Vec<i32>,
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
        limit: Option<i32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let mut stations = self
            .station_repository
            .get_by_coordinates(latitude, longitude, limit)
            .await?;

        self.update_station_vec_with_attributes(&mut stations, None)
            .await?;

        Ok(stations)
    }
    async fn get_station_id_and_distance_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        line_id: Option<i32>,
    ) -> Result<StationIdWithDistance, UseCaseError> {
        let station = self
            .station_repository
            .get_station_id_and_distance_by_coordinates(latitude, longitude, line_id)
            .await?;

        Ok(station)
    }
    async fn get_stations_by_line_id(
        &self,
        line_id: i32,
        station_id: Option<i32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let mut stations = self
            .station_repository
            .get_by_line_id(line_id, station_id)
            .await?;

        self.update_station_vec_with_attributes(&mut stations, None)
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_name(
        &self,
        station_name: String,
        limit: Option<i32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let mut stations = self
            .station_repository
            .get_by_name(station_name, limit)
            .await?;

        self.update_station_vec_with_attributes(&mut stations, None)
            .await?;

        Ok(stations)
    }
    async fn find_company_by_id_vec(
        &self,
        company_id_vec: Vec<i32>,
    ) -> Result<Vec<Company>, UseCaseError> {
        let companies = self
            .company_repository
            .find_by_id_vec(company_id_vec)
            .await?;

        Ok(companies)
    }
    async fn update_station_vec_with_attributes(
        &self,
        stations_ref: &mut Vec<Station>,
        line_group_id: Option<i32>,
    ) -> Result<(), UseCaseError> {
        let station_group_ids = stations_ref
            .iter()
            .map(|station| station.station_g_cd)
            .collect::<Vec<i32>>();

        let stations_by_group_ids = self
            .get_stations_by_group_id_vec(station_group_ids.clone())
            .await?;

        let station_ids = stations_by_group_ids
            .iter()
            .map(|station| station.station_cd)
            .collect::<Vec<i32>>();

        let lines = self
            .get_lines_by_station_group_id_vec(station_group_ids.clone())
            .await?;

        let company_ids = lines
            .iter()
            .map(|station| station.company_cd)
            .collect::<Vec<i32>>();
        let companies = self.find_company_by_id_vec(company_ids).await?;

        let train_types = self
            .get_train_types_by_station_id_vec(station_ids, line_group_id)
            .await?;

        for station in stations_ref.iter_mut() {
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
                .filter(|&l| l.station_g_cd.is_some())
                .filter(|&l| l.station_g_cd.unwrap() == station.station_g_cd)
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
        }

        Ok(())
    }
    async fn get_lines_by_station_group_id(
        &self,
        station_group_id: i32,
    ) -> Result<Vec<Line>, UseCaseError> {
        let lines = self
            .line_repository
            .get_by_station_group_id(station_group_id)
            .await?;

        Ok(lines)
    }
    async fn get_stations_by_line_group_id(
        &self,
        line_group_id: i32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let mut stations = self
            .station_repository
            .get_by_line_group_id(line_group_id)
            .await?;

        self.update_station_vec_with_attributes(&mut stations, Some(line_group_id))
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
            station.line_symbol_primary_color.unwrap_or_default(),
            station.line_symbol_secondary_color.unwrap_or_default(),
            station.line_symbol_extra_color.unwrap_or_default(),
        ];

        let station_numbers_raw = vec![
            station.primary_station_number.unwrap_or_default(),
            station.secondary_station_number.unwrap_or_default(),
            station.extra_station_number.unwrap_or_default(),
        ];

        let line_symbols_shape_raw: Vec<String> = vec![
            station.line_symbol_primary_shape.unwrap_or_default(),
            station.line_symbol_secondary_shape.unwrap_or_default(),
            station.line_symbol_extra_shape.unwrap_or_default(),
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
                    line_symbol: sym.to_owned(),
                    line_symbol_color: sym_color,
                    line_symbol_shape: sym_shape,
                    station_number: station_number_string,
                };
                station_numbers.push(station_number);
            } else {
                let station_number = StationNumber {
                    line_symbol: "".to_string(),
                    line_symbol_color: sym_color,
                    line_symbol_shape: sym_shape,
                    station_number,
                };
                station_numbers.push(station_number);
            }
        }

        station_numbers
    }
    fn extract_line_from_station(&self, station: &Station) -> Line {
        let station = station.clone();
        Line {
            line_cd: station.line_cd,
            company_cd: station.company_cd,
            company: None,
            line_name: station.line_name,
            line_name_k: station.line_name_k,
            line_name_h: station.line_name_h,
            line_name_r: station.line_name_r.unwrap_or_default(),
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
            station_g_cd: None,
            average_distance: station.average_distance,
        }
    }
    fn get_line_symbols(&self, line: &Line) -> Vec<LineSymbol> {
        let line_symbol_primary = &line.line_symbol_primary;
        let line_symbol_secondary = &line.line_symbol_secondary;
        let line_symbol_extra = &line.line_symbol_extra;
        let line_symbols_raw = [
            line_symbol_primary,
            line_symbol_secondary,
            line_symbol_extra,
        ];

        let line_symbol_primary_color = match line.line_symbol_primary_color {
            Some(ref color) => color.to_string(),
            None => line.line_color_c.to_string(),
        };
        let line_symbol_secondary_color =
            line.line_symbol_secondary_color.clone().unwrap_or_default();
        let line_symbol_extra_color = line.line_symbol_extra_color.clone().unwrap_or_default();

        let line_symbol_colors_raw: Vec<String> = vec![
            line_symbol_primary_color,
            line_symbol_secondary_color,
            line_symbol_extra_color,
        ];

        let line_symbol_primary_shape = line.line_symbol_primary_shape.clone().unwrap_or_default();
        let line_symbol_secondary_shape =
            line.line_symbol_secondary_shape.clone().unwrap_or_default();
        let line_symbol_extra_shape = line.line_symbol_extra_shape.clone().unwrap_or_default();

        let line_symbols_shape_raw = [
            line_symbol_primary_shape,
            line_symbol_secondary_shape,
            line_symbol_extra_shape,
        ];

        if line_symbols_raw.is_empty() {
            return vec![];
        }

        let mut line_symbols: Vec<LineSymbol> = Vec::with_capacity(line_symbols_raw.len());

        (0..line_symbols_raw.len()).for_each(|index| {
            let Some(symbol) = line_symbols_raw[index] else {
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
        station_id: i32,
    ) -> Result<Vec<TrainType>, UseCaseError> {
        let mut train_types: Vec<TrainType> = self
            .train_type_repository
            .get_by_station_id(station_id)
            .await?;

        let train_type_ids: Vec<i32> = train_types.iter().map(|tt| tt.line_group_cd).collect();

        let mut lines = self
            .line_repository
            .get_by_line_group_id_vec(train_type_ids.clone())
            .await?;

        let company_ids = lines.iter().map(|l| l.company_cd).collect();
        let companies = self.company_repository.find_by_id_vec(company_ids).await?;

        let inner_train_types = self
            .train_type_repository
            .get_by_line_group_id_vec(train_type_ids)
            .await?;

        for tt in train_types.iter_mut() {
            let mut lines: Vec<Line> = lines
                .iter_mut()
                .map(|l| l.clone())
                .filter(|l| l.line_group_cd == Some(tt.line_group_cd))
                .collect::<Vec<Line>>();

            let line_train_types: Vec<TrainType> = inner_train_types
                .clone()
                .into_iter()
                .filter(|itt| itt.line_group_cd == tt.line_group_cd)
                .collect();

            for line in lines.iter_mut() {
                line.company = companies
                    .iter()
                    .find(|c| c.company_cd == line.company_cd)
                    .cloned();
                line.line_symbols = self.get_line_symbols(line);

                let train_type = line_train_types
                    .iter()
                    .find(|ltt| ltt.line_cd == line.line_cd);

                line.train_type = train_type.cloned();
            }

            tt.lines = lines;
        }

        Ok(train_types)
    }

    async fn get_train_types_by_station_id_vec(
        &self,
        station_id_vec: Vec<i32>,
        line_group_id: Option<i32>,
    ) -> Result<Vec<TrainType>, UseCaseError> {
        let train_types = self
            .train_type_repository
            .get_types_by_station_id_vec(station_id_vec, line_group_id)
            .await?;

        Ok(train_types)
    }

    async fn get_routes(
        &self,
        from_station_id: i32,
        to_station_id: i32,
    ) -> Result<Vec<ApiRoute>, UseCaseError> {
        let rows = self
            .routes_repository
            .get_routes(from_station_id, to_station_id)
            .await?;

        let route_row_tree_map: &BTreeMap<i32, Vec<RouteRow>> = &rows.clone().into_iter().fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<i32, Vec<RouteRow>>, value| {
                if let Some(line_group_cd) = value.line_group_cd {
                    acc.entry(line_group_cd).or_default().push(value);
                } else {
                    acc.entry(value.line_cd.unwrap_or_default())
                        .or_default()
                        .push(value);
                };
                acc
            },
        );

        let mut routes = vec![];

        for (id, stops) in route_row_tree_map {
            let stops_with_line = stops
                .iter()
                .map(|row| {
                    let mut stop =
                        std::convert::Into::<crate::station_api::Station>::into(row.clone());
                    stop.line = Some(Box::new(crate::station_api::Line {
                        id: row.line_cd.unwrap_or_default(),
                        name_short: row.clone().line_name.unwrap_or_default(),
                        name_katakana: row.clone().line_name_k.unwrap_or_default(),
                        name_full: row.clone().line_name_h.unwrap_or_default(),
                        name_roman: row.line_name_r.clone(),
                        name_chinese: row.line_name_zh.clone(),
                        name_korean: row.line_name_ko.clone(),
                        color: row.clone().line_color_c.unwrap_or("#ffffff".to_string()),
                        line_type: row.line_type.unwrap_or_default(),
                        line_symbols: vec![],
                        status: row.e_status.unwrap_or_default(),
                        station: None,
                        company: None,
                        train_type: None,
                        average_distance: 0.0,
                    }));
                    if row.has_train_types.unwrap_or(false) {
                        stop.train_type = Some(Box::new(crate::station_api::TrainType {
                            id: row.type_id.unwrap_or_default(),
                            type_id: row.type_cd.unwrap_or_default(),
                            group_id: row.line_group_cd.unwrap_or_default(),
                            name: row.type_name.to_owned().unwrap_or_default(),
                            name_katakana: row.type_name_k.to_owned().unwrap_or_default(),
                            name_roman: row.type_name_r.to_owned(),
                            name_chinese: row.type_name_zh.to_owned(),
                            name_korean: row.type_name_ko.to_owned(),
                            color: row.color.to_owned().unwrap_or_default(),
                            lines: vec![],
                            line: None,
                            direction: row.direction.unwrap_or(0),
                            kind: row.kind.unwrap_or(0),
                        }));
                    }
                    stop
                })
                .collect();
            routes.push(ApiRoute {
                id: *id,
                stops: stops_with_line,
            });
        }
        Ok(routes)
    }
}
