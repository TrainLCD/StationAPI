//! ODPT API Client
//!
//! Client for fetching train timetable data from the ODPT (Open Data for Public Transportation) API.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{info, warn};

const ODPT_API_BASE_URL: &str = "https://api.odpt.org/api/v4";

/// ODPT API operators supported for stop pattern detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OdptOperator {
    TokyoMetro,
    Toei,
    JREast,
    Tobu,
    Seibu,
    Keio,
    Odakyu,
    Tokyu,
    Keikyu,
    Keisei,
    Sotetsu,
}

impl OdptOperator {
    pub fn id(&self) -> &'static str {
        match self {
            OdptOperator::TokyoMetro => "TokyoMetro",
            OdptOperator::Toei => "Toei",
            OdptOperator::JREast => "JR-East",
            OdptOperator::Tobu => "Tobu",
            OdptOperator::Seibu => "Seibu",
            OdptOperator::Keio => "Keio",
            OdptOperator::Odakyu => "Odakyu",
            OdptOperator::Tokyu => "Tokyu",
            OdptOperator::Keikyu => "Keikyu",
            OdptOperator::Keisei => "Keisei",
            OdptOperator::Sotetsu => "Sotetsu",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            OdptOperator::TokyoMetro => "東京メトロ",
            OdptOperator::Toei => "都営地下鉄",
            OdptOperator::JREast => "JR東日本",
            OdptOperator::Tobu => "東武鉄道",
            OdptOperator::Seibu => "西武鉄道",
            OdptOperator::Keio => "京王電鉄",
            OdptOperator::Odakyu => "小田急電鉄",
            OdptOperator::Tokyu => "東急電鉄",
            OdptOperator::Keikyu => "京急電鉄",
            OdptOperator::Keisei => "京成電鉄",
            OdptOperator::Sotetsu => "相鉄",
        }
    }

    pub fn all() -> Vec<OdptOperator> {
        vec![
            OdptOperator::TokyoMetro,
            OdptOperator::Toei,
            OdptOperator::JREast,
            OdptOperator::Tobu,
            OdptOperator::Seibu,
            OdptOperator::Keio,
            OdptOperator::Odakyu,
            OdptOperator::Tokyu,
            OdptOperator::Keikyu,
            OdptOperator::Keisei,
            OdptOperator::Sotetsu,
        ]
    }
}

/// Train timetable object (stop in a trip)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrainTimetableObject {
    #[serde(rename = "odpt:departureTime")]
    pub departure_time: Option<String>,
    #[serde(rename = "odpt:departureStation")]
    pub departure_station: Option<String>,
    #[serde(rename = "odpt:arrivalTime")]
    pub arrival_time: Option<String>,
    #[serde(rename = "odpt:arrivalStation")]
    pub arrival_station: Option<String>,
}

/// Train timetable from ODPT API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrainTimetable {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "odpt:operator")]
    pub operator: String,
    #[serde(rename = "odpt:railway")]
    pub railway: String,
    #[serde(rename = "odpt:trainNumber")]
    pub train_number: Option<String>,
    #[serde(rename = "odpt:trainType")]
    pub train_type: Option<String>,
    #[serde(rename = "odpt:trainTimetableObject")]
    pub train_timetable_object: Vec<TrainTimetableObject>,
}

/// Railway information from ODPT API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Railway {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "odpt:operator")]
    pub operator: String,
    #[serde(rename = "dc:title")]
    pub title: Option<String>,
    #[serde(rename = "odpt:railwayTitle")]
    pub railway_title: Option<RailwayTitle>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RailwayTitle {
    Simple(String),
    Multilang(HashMap<String, String>),
}

impl Railway {
    pub fn get_name(&self) -> String {
        match &self.railway_title {
            Some(RailwayTitle::Simple(s)) => s.clone(),
            Some(RailwayTitle::Multilang(map)) => {
                map.get("ja").or(map.get("en")).cloned().unwrap_or_default()
            }
            None => self.title.clone().unwrap_or_default(),
        }
    }
}

/// Train type information from ODPT API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrainType {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "odpt:operator")]
    pub operator: String,
    #[serde(rename = "dc:title")]
    pub title: Option<String>,
    #[serde(rename = "odpt:trainTypeTitle")]
    pub train_type_title: Option<TrainTypeTitle>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TrainTypeTitle {
    Simple(String),
    Multilang(HashMap<String, String>),
}

impl TrainType {
    pub fn get_name(&self) -> String {
        match &self.train_type_title {
            Some(TrainTypeTitle::Simple(s)) => s.clone(),
            Some(TrainTypeTitle::Multilang(map)) => {
                map.get("ja").or(map.get("en")).cloned().unwrap_or_default()
            }
            None => self.title.clone().unwrap_or_default(),
        }
    }
}

/// Station information from ODPT API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Station {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "odpt:operator")]
    pub operator: String,
    #[serde(rename = "odpt:railway")]
    pub railway: Option<String>,
    #[serde(rename = "dc:title")]
    pub title: Option<String>,
    #[serde(rename = "odpt:stationTitle")]
    pub station_title: Option<StationTitle>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StationTitle {
    Simple(String),
    Multilang(HashMap<String, String>),
}

impl Station {
    pub fn get_name(&self) -> String {
        match &self.station_title {
            Some(StationTitle::Simple(s)) => s.clone(),
            Some(StationTitle::Multilang(map)) => {
                map.get("ja").or(map.get("en")).cloned().unwrap_or_default()
            }
            None => self.title.clone().unwrap_or_default(),
        }
    }
}

/// Extracted stop pattern for a railway/train type combination
#[derive(Debug, Clone)]
pub struct StopPattern {
    pub operator_id: String,
    pub railway_id: String,
    pub railway_name: String,
    pub train_type_id: String,
    pub train_type_name: String,
    pub station_ids: Vec<String>,
    pub station_names: Vec<String>,
}

/// ODPT API Client
pub struct OdptClient {
    pub(crate) api_key: String,
    client: reqwest::blocking::Client,
}

impl OdptClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Fetch train timetables for an operator
    pub fn fetch_train_timetables(
        &self,
        operator: OdptOperator,
    ) -> Result<Vec<TrainTimetable>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/odpt:TrainTimetable?odpt:operator=odpt.Operator:{}&acl:consumerKey={}",
            ODPT_API_BASE_URL,
            operator.id(),
            self.api_key
        );

        info!(
            "Fetching train timetables for {} ({})...",
            operator.name(),
            operator.id()
        );

        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to fetch timetables for {}: HTTP {}",
                operator.id(),
                response.status()
            )
            .into());
        }

        let timetables: Vec<TrainTimetable> = response.json()?;
        info!(
            "Fetched {} timetables for {}",
            timetables.len(),
            operator.name()
        );

        Ok(timetables)
    }

    /// Fetch railways for an operator
    pub fn fetch_railways(
        &self,
        operator: OdptOperator,
    ) -> Result<Vec<Railway>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/odpt:Railway?odpt:operator=odpt.Operator:{}&acl:consumerKey={}",
            ODPT_API_BASE_URL,
            operator.id(),
            self.api_key
        );

        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to fetch railways for {}: HTTP {}",
                operator.id(),
                response.status()
            )
            .into());
        }

        let railways: Vec<Railway> = response.json()?;
        Ok(railways)
    }

    /// Fetch train types for an operator
    pub fn fetch_train_types(
        &self,
        operator: OdptOperator,
    ) -> Result<Vec<TrainType>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/odpt:TrainType?odpt:operator=odpt.Operator:{}&acl:consumerKey={}",
            ODPT_API_BASE_URL,
            operator.id(),
            self.api_key
        );

        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to fetch train types for {}: HTTP {}",
                operator.id(),
                response.status()
            )
            .into());
        }

        let train_types: Vec<TrainType> = response.json()?;
        Ok(train_types)
    }

    /// Fetch stations for an operator
    pub fn fetch_stations(
        &self,
        operator: OdptOperator,
    ) -> Result<Vec<Station>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/odpt:Station?odpt:operator=odpt.Operator:{}&acl:consumerKey={}",
            ODPT_API_BASE_URL,
            operator.id(),
            self.api_key
        );

        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to fetch stations for {}: HTTP {}",
                operator.id(),
                response.status()
            )
            .into());
        }

        let stations: Vec<Station> = response.json()?;
        Ok(stations)
    }

    /// Extract stop patterns from train timetables
    pub fn extract_stop_patterns(
        &self,
        operator: OdptOperator,
    ) -> Result<Vec<StopPattern>, Box<dyn std::error::Error + Send + Sync>> {
        // Fetch all required data
        let timetables = self.fetch_train_timetables(operator)?;
        let railways = self.fetch_railways(operator)?;
        let train_types = self.fetch_train_types(operator)?;
        let stations = self.fetch_stations(operator)?;

        // Build lookup maps
        let railway_names: HashMap<String, String> = railways
            .iter()
            .map(|r| (r.id.clone(), r.get_name()))
            .collect();

        let train_type_names: HashMap<String, String> = train_types
            .iter()
            .map(|t| (t.id.clone(), t.get_name()))
            .collect();

        let station_names: HashMap<String, String> = stations
            .iter()
            .map(|s| (s.id.clone(), s.get_name()))
            .collect();

        // Group timetables by railway and train type
        let mut patterns: HashMap<(String, String), HashSet<String>> = HashMap::new();

        for timetable in &timetables {
            let train_type_id = match &timetable.train_type {
                Some(t) => t.clone(),
                None => continue, // Skip if no train type
            };

            let key = (timetable.railway.clone(), train_type_id);

            let station_set = patterns.entry(key).or_default();

            for obj in &timetable.train_timetable_object {
                if let Some(station) = &obj.departure_station {
                    station_set.insert(station.clone());
                }
                if let Some(station) = &obj.arrival_station {
                    station_set.insert(station.clone());
                }
            }
        }

        // Convert to StopPattern structs
        let operator_id = format!("odpt.Operator:{}", operator.id());
        let mut result: Vec<StopPattern> = Vec::new();

        for ((railway_id, train_type_id), station_ids) in patterns {
            let railway_name = railway_names
                .get(&railway_id)
                .cloned()
                .unwrap_or_else(|| railway_id.clone());

            let train_type_name = train_type_names
                .get(&train_type_id)
                .cloned()
                .unwrap_or_else(|| train_type_id.clone());

            // Sort station IDs for consistent ordering
            let mut station_ids: Vec<String> = station_ids.into_iter().collect();
            station_ids.sort();

            let station_names_list: Vec<String> = station_ids
                .iter()
                .map(|id| station_names.get(id).cloned().unwrap_or_else(|| id.clone()))
                .collect();

            result.push(StopPattern {
                operator_id: operator_id.clone(),
                railway_id,
                railway_name,
                train_type_id,
                train_type_name,
                station_ids,
                station_names: station_names_list,
            });
        }

        info!(
            "Extracted {} stop patterns for {}",
            result.len(),
            operator.name()
        );

        Ok(result)
    }

    /// Extract stop patterns for multiple operators
    pub fn extract_all_stop_patterns(
        &self,
        operators: &[OdptOperator],
    ) -> Result<Vec<StopPattern>, Box<dyn std::error::Error + Send + Sync>> {
        let mut all_patterns: Vec<StopPattern> = Vec::new();

        for operator in operators {
            match self.extract_stop_patterns(*operator) {
                Ok(patterns) => {
                    all_patterns.extend(patterns);
                }
                Err(e) => {
                    warn!("Failed to extract patterns for {}: {}", operator.name(), e);
                    // Continue with other operators
                }
            }
        }

        Ok(all_patterns)
    }
}
