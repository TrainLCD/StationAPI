use std::vec;

use crate::{
    entities::line::Line,
    repositories::station::StationRepository,
    service::{
        LineResponse, LineSymbol, MultipleStationResponse, SingleStationResponse, StationResponse,
    },
};
use bigdecimal::Zero;
use futures::future::join_all;

fn get_line_symbols(l: Line) -> Vec<LineSymbol> {
    let mut line_symbols = vec![];

    if !l.line_symbol_primary.len().is_zero() {
        let line_symbol = LineSymbol {
            symbol: l.line_symbol_primary,
            color: l.line_symbol_primary_color,
            shape: l.line_symbol_primary_shape,
        };
        line_symbols.push(line_symbol);
    }

    if !l.line_symbol_secondary.len().is_zero() {
        let line_symbol = LineSymbol {
            symbol: l.line_symbol_secondary,
            color: l.line_symbol_secondary_color,
            shape: l.line_symbol_secondary_shape,
        };
        line_symbols.push(line_symbol);
    }

    if !l.line_symbol_extra.len().is_zero() {
        let line_symbol = LineSymbol {
            symbol: l.line_symbol_extra,
            color: l.line_symbol_extra_color,
            shape: l.line_symbol_extra_shape,
        };
        line_symbols.push(line_symbol);
    }
    line_symbols
}

pub async fn find_station_by_id(
    repository: impl StationRepository,
    id: u32,
) -> Option<SingleStationResponse> {
    if let Some(station) = repository.find_one(id).await {
        let mut station_response: StationResponse = station.into();

        if let Some(lines) = repository
            .get_lines_by_station_id(station_response.id)
            .await
        {
            let line_ids: Vec<u32> = lines.iter().map(|l| l.line_cd).collect::<Vec<u32>>();
            let companies = repository
                .get_companies_by_line_ids(line_ids)
                .await
                .unwrap_or(vec![]);

            let lines = lines
                .into_iter()
                .enumerate()
                .map(|(i, l)| {
                    let mut resp: LineResponse = l.clone().into();
                    let line_symbols = get_line_symbols(l);
                    resp.line_symbols = line_symbols;
                    if let Some(company) = companies.get(i) {
                        resp.company = Some(company.clone().into());
                    }
                    resp
                })
                .collect();
            station_response.lines = lines;
        }

        return Some(SingleStationResponse {
            station: Some(station_response),
        });
    }
    None
}
pub async fn get_stations_by_group_id(
    repository: impl StationRepository,
    group_id: u32,
) -> MultipleStationResponse {
    let stations = repository.get_by_group_id(group_id).await.unwrap_or(vec![]);
    let station_responses: Vec<StationResponse> = stations.into_iter().map(|s| s.into()).collect();

    let station_responses_futures = station_responses.clone().into_iter().map(|s| async {
        let mut station_response: StationResponse = s;

        let lines = repository
            .get_lines_by_station_id(station_response.id)
            .await
            .unwrap_or(vec![]);

        let line_ids = lines.iter().map(|l| l.line_cd).collect();
        let companies = repository
            .get_companies_by_line_ids(line_ids)
            .await
            .unwrap_or(vec![]);

        let lines = lines
            .into_iter()
            .enumerate()
            .map(|(i, l)| {
                let mut resp: LineResponse = l.clone().into();
                let line_symbols = get_line_symbols(l);
                resp.line_symbols = line_symbols;
                if let Some(company) = companies.get(i) {
                    resp.company = Some(company.clone().into());
                }
                resp
            })
            .collect();
        station_response.lines = lines;
        station_response
    });

    let station_responses = join_all(station_responses_futures).await;

    MultipleStationResponse {
        stations: station_responses,
    }
}

pub async fn get_stations_by_coordinates(
    repository: impl StationRepository,
    latitude: f64,
    longitude: f64,
    limit: Option<i32>,
) -> MultipleStationResponse {
    let stations = repository
        .get_by_coordinates(latitude, longitude, limit)
        .await
        .unwrap_or(vec![]);

    let station_responses: Vec<StationResponse> = stations.into_iter().map(|s| s.into()).collect();
    let futures = station_responses
        .into_iter()
        .map(|s: StationResponse| async {
            let mut station_response = s;
            let lines = repository
                .get_lines_by_station_id(station_response.id)
                .await
                .unwrap_or(vec![]);

            let line_ids = lines.iter().map(|l| l.line_cd).collect();
            let companies = repository
                .get_companies_by_line_ids(line_ids)
                .await
                .unwrap_or(vec![]);

            let lines = lines
                .into_iter()
                .enumerate()
                .map(|(i, l)| {
                    let mut resp: LineResponse = l.clone().into();
                    let line_symbols = get_line_symbols(l);
                    resp.line_symbols = line_symbols;
                    if let Some(company) = companies.get(i) {
                        resp.company = Some(company.clone().into());
                    }
                    resp
                })
                .collect();
            station_response.lines = lines;
            station_response
        });

    let station_responses = join_all(futures).await;

    MultipleStationResponse {
        stations: station_responses,
    }
}
