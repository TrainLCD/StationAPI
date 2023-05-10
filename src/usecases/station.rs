use std::vec;

use crate::{
    entities::{
        line::Line,
        station::{Station, StationWithDistance},
    },
    repositories::station::StationRepository,
    service::{
        LineResponse, LineSymbol, MultipleStationResponse, SingleStationResponse, StationResponse,
    },
};
use bigdecimal::Zero;
use futures::future::join_all;

async fn get_minimal_station_responses(
    repository: &impl StationRepository,
    stations: Vec<StationWithDistance>,
) -> Vec<StationResponse> {
    let station_responses: Vec<StationResponse> =
        stations.clone().into_iter().map(|s| s.into()).collect();

    let station_responses_futures = station_responses.into_iter().map(|s| async {
        let mut station_response: StationResponse = s;

        let Ok(lines) = repository
            .get_lines_by_station_id(station_response.id)
            .await else {
            return station_response;
            };

        let line_ids = lines.iter().map(|l| l.line_cd).collect();
        let companies = repository
            .get_companies_by_line_ids(line_ids)
            .await
            .unwrap();

        let lines: Vec<LineResponse> = lines
            .iter()
            .enumerate()
            .into_iter()
            .map(|(i, l)| {
                let mut resp: LineResponse = l.clone().into();
                let line_symbols = get_line_symbols(&mut l.clone());
                resp.line_symbols = line_symbols;
                let Some(company) = companies.get(i) else {
                    return resp;
                };
                resp.company = Some(company.clone().into());
                resp
            })
            .collect();

        let Ok(line) = repository
            .find_one_line_by_station_id(station_response.id)
            .await else {
                return station_response;
            };
        let mut line = line;
        let mut line_response: LineResponse = line.clone().into();
        let Ok(company) = repository.find_one_company_by_line_id(line_response.id).await else {
            return station_response;
        };
        line_response.company = Some(company.into());
        let line_symbols = get_line_symbols(&mut line);
        line_response.line_symbols = line_symbols;
        station_response.line = Some(Box::new(line_response));
        station_response.lines = lines;
        station_response
    });

    join_all(station_responses_futures).await
}

async fn get_station_responses(
    repository: impl StationRepository,
    stations: Vec<Station>,
) -> Vec<StationResponse> {
    let partially_constructed_response =
        get_minimal_station_responses(&repository, stations.into_iter().map(|s| s.into()).collect())
            .await;

    let station_responses_futures = partially_constructed_response.into_iter().map(|s| async {
        let mut station_response: StationResponse = s;
        let Some(mut line_response) =  station_response.clone().line else {
            return station_response;
        };
        let line_ids: Vec<u32> = station_response.lines.iter().map(|l| l.id).collect();
        

        let Ok(transferable_stations) = repository
        .get_transferable_stations(station_response.group_id, line_ids).await else {
            return station_response;
        };
        let Some(transferable_station) = 
            transferable_stations.iter().find_map(|ts| {
                if ts.line_cd == line_response.id {
                    return Some(ts);
                }
                None
            }).cloned() else {
                return station_response;
            };

        line_response.station = Some(Box::new(transferable_station.into()));
        station_response.line = Some(line_response);
        station_response
    });

    join_all(station_responses_futures).await
}

fn get_line_symbols(line: &mut Line) -> Vec<LineSymbol> {
    let mut line_symbols = vec![];

    if !line
        .line_symbol_primary
        .clone()
        .unwrap_or("".to_string())
        .len()
        .is_zero()
    {
        let line_symbol = LineSymbol {
            symbol: line.clone().line_symbol_primary.unwrap_or("".to_string()),
            color: line
                .clone()
                .line_symbol_primary_color
                .unwrap_or("".to_string()),
            shape: line
                .clone()
                .line_symbol_primary_shape
                .unwrap_or("".to_string()),
        };
        line_symbols.push(line_symbol);
    }

    if !line
        .line_symbol_secondary
        .clone()
        .unwrap_or("".to_string())
        .len()
        .is_zero()
    {
        let line_symbol = LineSymbol {
            symbol: line.clone().line_symbol_secondary.unwrap_or("".to_string()),
            color: line
                .clone()
                .line_symbol_secondary_color
                .unwrap_or("".to_string()),
            shape: line
                .clone()
                .line_symbol_secondary_shape
                .unwrap_or("".to_string()),
        };
        line_symbols.push(line_symbol);
    }

    if !line
        .line_symbol_extra
        .clone()
        .unwrap_or("".to_string())
        .len()
        .is_zero()
    {
        let line_symbol = LineSymbol {
            symbol: line.clone().line_symbol_extra.unwrap_or("".to_string()),
            color: line
                .clone()
                .line_symbol_extra_color
                .unwrap_or("".to_string()),
            shape: line
                .clone()
                .line_symbol_extra_shape
                .unwrap_or("".to_string()),
        };
        line_symbols.push(line_symbol);
    }
    line_symbols
}

pub async fn find_station_by_id(
    repository: impl StationRepository,
    id: u32,
) -> Option<SingleStationResponse> {
    let Ok(fetched_station) = repository.find_one(id).await else {
        return None
    };

    let stations = get_station_responses(repository, vec![fetched_station]).await;

    Some(SingleStationResponse {
        station: stations.first().cloned(),
    })
}

pub async fn get_stations_by_group_id(
    repository: impl StationRepository,
    group_id: u32,
) -> MultipleStationResponse {
    let stations = repository.get_by_group_id(group_id).await.unwrap();

    MultipleStationResponse {
        stations: get_station_responses(repository, stations).await,
    }
}

pub async fn get_stations_by_coordinates(
    repository: &impl StationRepository,
    latitude: f64,
    longitude: f64,
    limit: Option<i32>,
) -> MultipleStationResponse {
    let Ok(stations) = repository
        .get_by_coordinates(latitude, longitude, limit)
        .await else {
            return MultipleStationResponse { stations: vec![] };
        };
    MultipleStationResponse {
        stations: get_minimal_station_responses(repository, stations).await,
    }
}

pub async fn get_stations_by_line_id(
    repository: impl StationRepository,
    line_id: u32,
) -> MultipleStationResponse {
    let Ok(stations) = repository.get_by_line_id(line_id).await else {
        return MultipleStationResponse { stations: vec![] };
    };

    MultipleStationResponse {
        stations: get_station_responses(repository, stations).await,
    }
}
