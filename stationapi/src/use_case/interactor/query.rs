use std::{
    collections::hash_map::DefaultHasher,
    env::{self, VarError},
    hash::{Hash, Hasher},
    vec,
};

use async_trait::async_trait;

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

#[derive(Clone)]
pub struct QueryInteractor<SR, LR, TR, CR> {
    pub station_repository: SR,
    pub line_repository: LR,
    pub train_type_repository: TR,
    pub company_repository: CR,
    pub cache_client: Option<memcache::Client>,
}

#[async_trait]
impl<SR, LR, TR, CR> QueryUseCase for QueryInteractor<SR, LR, TR, CR>
where
    SR: StationRepository,
    LR: LineRepository,
    TR: TrainTypeRepository,
    CR: CompanyRepository,
{
    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        match env::var("DISABLE_MEMCACHE") {
            Ok(s) => {
                let is_memcached_disabled: bool = s.parse().unwrap();
                if !is_memcached_disabled {
                    let mut s = DefaultHasher::new();
                    t.hash(&mut s);
                    return s.finish();
                }
                0
            }
            Err(env::VarError::NotPresent) => 0,
            Err(VarError::NotUnicode(_)) => 0,
        }
    }

    async fn find_station_by_id(&self, station_id: u32) -> Result<Option<Station>, UseCaseError> {
        let cache_key = format!("find_station_by_id:{}", Self::calculate_hash(&station_id));

        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(&cache_key) {
                if let Ok(station) = serde_json::from_str::<Station>(&cache_value) {
                    return Ok(Some(station));
                };
            };
        }

        let Some(station) = self.station_repository.find_by_id(station_id).await? else {
            return Ok(None);
        };
        let result_vec = &mut vec![station];
        self.update_station_vec_with_attributes(result_vec, None)
            .await?;
        let station = result_vec.first().cloned();

        if let Some(cache_client) = &self.cache_client {
            if let Ok(station_str) = serde_json::to_string(&station) {
                let _ = cache_client.set(&cache_key, station_str, 0);
            };
        }

        Ok(station)
    }
    async fn get_stations_by_id_vec(
        &self,
        station_ids: Vec<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let cache_key: String = format!(
            "get_stations_by_id_vec:{}",
            Self::calculate_hash(&station_ids)
        );
        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(&cache_key) {
                if let Ok(stations) = serde_json::from_str::<Vec<Station>>(&cache_value) {
                    return Ok(stations);
                };
            };
        }

        let mut stations = self.station_repository.get_by_id_vec(station_ids).await?;
        self.update_station_vec_with_attributes(&mut stations, None)
            .await?;

        if let Some(cache_client) = &self.cache_client {
            if let Ok(stations_str) = serde_json::to_string(&stations) {
                let _ = cache_client.set(&cache_key, stations_str, 0);
            };
        }

        Ok(stations)
    }
    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let cache_key = format!(
            "get_stations_by_group_id:{}",
            Self::calculate_hash(&station_group_id)
        );
        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(&cache_key) {
                if let Ok(stations) = serde_json::from_str::<Vec<Station>>(&cache_value) {
                    return Ok(stations);
                }
            };
        }

        let mut stations = self
            .station_repository
            .get_by_station_group_id(station_group_id)
            .await?;

        self.update_station_vec_with_attributes(&mut stations, Some(station_group_id))
            .await?;

        if let Some(cache_client) = &self.cache_client {
            if let Ok(stations_str) = serde_json::to_string(&stations) {
                let _ = cache_client.set(&cache_key, stations_str, 0);
            };
        }

        Ok(stations)
    }
    async fn get_stations_by_group_id_vec(
        &self,
        station_group_id_vec: Vec<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let cache_key = format!(
            "get_stations_by_group_id_vec:{}",
            Self::calculate_hash(&station_group_id_vec)
        );

        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(&cache_key) {
                if let Ok(stations) = serde_json::from_str::<Vec<Station>>(&cache_value) {
                    return Ok(stations);
                }
            };
        }

        let stations = self
            .station_repository
            .get_by_station_group_id_vec(station_group_id_vec)
            .await?;

        if let Some(cache_client) = &self.cache_client {
            if let Ok(stations_str) = serde_json::to_string(&stations) {
                let _ = cache_client.set(&cache_key, stations_str, 0);
            };
        }

        Ok(stations)
    }
    async fn get_lines_by_station_group_id_vec(
        &self,
        station_group_id_vec: Vec<u32>,
    ) -> Result<Vec<Line>, UseCaseError> {
        let cache_key = format!(
            "get_lines_by_station_group_id_vec:{}",
            Self::calculate_hash(&station_group_id_vec)
        );
        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                if let Ok(lines) = serde_json::from_str::<Vec<Line>>(&cache_value) {
                    return Ok(lines);
                }
            };
        }

        let lines = self
            .line_repository
            .get_by_station_group_id_vec(station_group_id_vec)
            .await?;

        if let Some(cache_client) = &self.cache_client {
            if let Ok(lines_str) = serde_json::to_string(&lines) {
                let _ = cache_client.set(&cache_key, lines_str, 0);
            };
        }

        Ok(lines)
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

        self.update_station_vec_with_attributes(&mut stations, None)
            .await?;

        Ok(stations)
    }
    async fn get_stations_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let cache_key = format!(
            "get_stations_by_line_id:{}",
            Self::calculate_hash(&format!("{}:{}", line_id, station_id.unwrap_or(0)))
        );

        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(&cache_key) {
                if let Ok(stations) = serde_json::from_str::<Vec<Station>>(&cache_value) {
                    return Ok(stations);
                }
            };
        }

        let mut stations = self
            .station_repository
            .get_by_line_id(line_id, station_id)
            .await?;

        self.update_station_vec_with_attributes(&mut stations, None)
            .await?;

        if let Some(cache_client) = &self.cache_client {
            if let Ok(stations_str) = serde_json::to_string(&stations) {
                let _ = cache_client.set(&cache_key, stations_str, 0);
            };
        }

        Ok(stations)
    }
    async fn get_stations_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let cache_key = format!(
            "get_stations_by_name:{}",
            Self::calculate_hash(&format!("{}:{}", station_name, limit.unwrap_or(0)))
        );

        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                if let Ok(stations) = serde_json::from_str::<Vec<Station>>(&cache_value) {
                    return Ok(stations);
                }
            };
        }

        let mut stations = self
            .station_repository
            .get_by_name(station_name, limit)
            .await?;

        self.update_station_vec_with_attributes(&mut stations, None)
            .await?;

        if let Some(cache_client) = &self.cache_client {
            if let Ok(stations_str) = serde_json::to_string(&stations) {
                let _ = cache_client.set(&cache_key, stations_str, 0);
            };
        }

        Ok(stations)
    }
    async fn find_company_by_id_vec(
        &self,
        company_id_vec: Vec<u32>,
    ) -> Result<Vec<Company>, UseCaseError> {
        let cache_key = format!(
            "find_company_by_id_vec:{}",
            Self::calculate_hash(&company_id_vec)
        );

        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                if let Ok(companies) = serde_json::from_str::<Vec<Company>>(&cache_value) {
                    return Ok(companies);
                }
            };
        }

        let companies = self
            .company_repository
            .find_by_id_vec(company_id_vec)
            .await?;

        if let Some(cache_client) = &self.cache_client {
            if let Ok(companies_str) = serde_json::to_string(&companies) {
                let _ = cache_client.set(&cache_key, companies_str, 0);
            };
        }

        Ok(companies)
    }
    async fn update_station_vec_with_attributes(
        &self,
        stations_ref: &mut Vec<Station>,
        line_group_id: Option<u32>,
    ) -> Result<(), UseCaseError> {
        let station_group_ids = stations_ref
            .iter()
            .map(|station| station.station_g_cd)
            .collect::<Vec<u32>>();

        let stations_by_group_ids = self
            .get_stations_by_group_id_vec(station_group_ids.clone())
            .await?;

        let station_ids = stations_by_group_ids
            .iter()
            .map(|station| station.station_cd)
            .collect::<Vec<u32>>();

        let lines = self
            .get_lines_by_station_group_id_vec(station_group_ids.clone())
            .await?;

        let company_ids = lines
            .iter()
            .map(|station| station.company_cd)
            .collect::<Vec<u32>>();
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
        station_group_id: u32,
    ) -> Result<Vec<Line>, UseCaseError> {
        let cache_key = format!(
            "get_lines_by_station_group_id:{}",
            Self::calculate_hash(&station_group_id)
        );

        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                if let Ok(lines) = serde_json::from_str::<Vec<Line>>(&cache_value) {
                    return Ok(lines);
                }
            };
        }

        let lines = self
            .line_repository
            .get_by_station_group_id(station_group_id)
            .await?;

        if let Some(cache_client) = &self.cache_client {
            if let Ok(lines_str) = serde_json::to_string(&lines) {
                let _ = cache_client.set(&cache_key, lines_str, 0);
            };
        }
        Ok(lines)
    }
    async fn get_stations_by_line_group_id(
        &self,
        line_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let cache_key = format!(
            "get_stations_by_line_group_id:{}",
            Self::calculate_hash(&line_group_id)
        );

        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                if let Ok(stations) = serde_json::from_str::<Vec<Station>>(&cache_value) {
                    return Ok(stations);
                }
            };
        }

        let mut stations = self
            .station_repository
            .get_by_line_group_id(line_group_id)
            .await?;

        self.update_station_vec_with_attributes(&mut stations, Some(line_group_id))
            .await?;

        if let Some(cache_client) = &self.cache_client {
            if let Ok(stations_str) = serde_json::to_string(&stations) {
                let _ = cache_client.set(&cache_key, stations_str, 0);
            };
        }

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
            station_g_cd: None,
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
        station_id: u32,
    ) -> Result<Vec<TrainType>, UseCaseError> {
        let cache_key = format!(
            "get_train_types_by_station_id:{}",
            Self::calculate_hash(&station_id)
        );

        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                if let Ok(train_types) = serde_json::from_str::<Vec<TrainType>>(&cache_value) {
                    return Ok(train_types);
                }
            };
        }

        let mut train_types = self
            .train_type_repository
            .get_by_station_id(station_id)
            .await?;

        let train_type_ids = train_types.iter().map(|tt| tt.line_group_cd).collect();

        let mut lines = self
            .line_repository
            .get_by_line_group_id_vec(train_type_ids)
            .await?;

        let company_ids = lines.iter().map(|l| l.company_cd).collect();
        let companies = self.company_repository.find_by_id_vec(company_ids).await?;

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

            line.train_type = Some(tt.clone());
            line.company = companies
                .iter()
                .find(|c| c.company_cd == line.company_cd)
                .cloned();
            line.line_symbols = self.get_line_symbols(&line);

            tt.lines = lines;
            tt.line = Some(Box::new(line.clone()));
        }

        if let Some(cache_client) = &self.cache_client {
            if let Ok(train_types_str) = serde_json::to_string(&train_types) {
                let _ = cache_client.set(&cache_key, train_types_str, 0);
            };
        }

        Ok(train_types)
    }

    async fn get_train_types_by_station_id_vec(
        &self,
        station_id_vec: Vec<u32>,
        line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, UseCaseError> {
        let station_id_key: String = station_id_vec
            .iter()
            .map(|id| id.to_string())
            .collect::<String>();

        let cache_key = format!(
            "get_train_types_by_station_id_vec:{}",
            Self::calculate_hash(&format!(
                "{}:{}",
                station_id_key,
                line_group_id.unwrap_or(0)
            ))
        );

        if let Some(cache_client) = &self.cache_client {
            if let Ok(Some(cache_value)) = cache_client.get::<String>(cache_key.as_str()) {
                if let Ok(train_types) = serde_json::from_str::<Vec<TrainType>>(&cache_value) {
                    return Ok(train_types);
                }
            };
        }

        let train_types = self
            .train_type_repository
            .get_types_by_station_id_vec(station_id_vec, line_group_id)
            .await?;

        if let Some(cache_client) = &self.cache_client {
            if let Ok(train_types_str) = serde_json::to_string(&train_types) {
                let _ = cache_client.set(&cache_key, train_types_str, 0);
            };
        }

        Ok(train_types)
    }
}
