use anyhow::Result;

use crate::{
    application::{line_service::LineService, station_service::StationService},
    pb::{
        GetStationByCoordinatesRequest, GetStationByGroupIdRequest, GetStationByIdRequest,
        GetStationByLineIdRequest, GetStationByNameRequest, LineResponse, MultipleStationResponse,
        SingleStationResponse, StationResponse,
    },
};

use super::router;

pub async fn get_station_by_id(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByIdRequest>,
) -> Result<SingleStationResponse> {
    let station_service = StationService::new(ctx.station_repository());
    let line_service = LineService::new(ctx.line_repository());

    let id = request.get_ref().id;
    let data = station_service.find_by_id(id).await?;
    let lines = line_service
        .get_by_station_group_id(data.station_g_cd)
        .await?;
    let lines_response: Vec<LineResponse> = lines
        .into_iter()
        .map(|mut line| {
            let mut line_response: LineResponse = line.clone().into();
            let line_symbols = line_service.get_line_symbols(&mut line);
            line_response.line_symbols = line_symbols;
            line_response
        })
        .collect();
    let mut station_line = line_service.find_by_station_id(data.station_cd).await?;
    let mut line_response: LineResponse = station_line.clone().into();
    let mut station_response: StationResponse = data.clone().into();
    station_response.lines = lines_response;
    let line_symbols = line_service.get_line_symbols(&mut station_line);
    line_response.line_symbols = line_symbols;
    station_response.line = Some(Box::new(line_response));

    station_response.station_numbers = station_service.get_station_numbers(&data, &station_line);

    Ok(SingleStationResponse {
        station: Some(station_response),
    })
}
pub async fn get_station_by_group_id(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByGroupIdRequest>,
) -> Result<MultipleStationResponse> {
    let station_service = StationService::new(ctx.station_repository());
    let line_service = LineService::new(ctx.line_repository());

    let group_id = request.get_ref().group_id;
    let stations = &station_service.get_by_group_id(group_id).await?;

    let stations_lines = &line_service.get_stations_lines(stations).await?;
    let response_stations = line_service
        .get_response_stations_from_stations(stations, stations_lines)
        .await?;

    Ok(MultipleStationResponse {
        stations: response_stations.to_vec(),
    })
}

pub async fn get_station_by_coordinates(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByCoordinatesRequest>,
) -> Result<MultipleStationResponse> {
    let station_service = StationService::new(ctx.station_repository());
    let line_service = LineService::new(ctx.line_repository());

    let latitude = request.get_ref().latitude;
    let longitude = request.get_ref().longitude;
    let limit = &request.get_ref().limit;
    let stations = &station_service
        .get_station_by_coordinates(latitude, longitude, limit)
        .await?;

    let stations_lines = &line_service.get_stations_lines(stations).await?;
    let response_stations = &line_service
        .get_response_stations_from_stations(stations, stations_lines)
        .await?;

    Ok(MultipleStationResponse {
        stations: response_stations.to_vec(),
    })
}

pub async fn get_stations_by_line_id(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByLineIdRequest>,
) -> Result<MultipleStationResponse> {
    let station_service = StationService::new(ctx.station_repository());
    let line_service = LineService::new(ctx.line_repository());

    let line_id: u32 = request.get_ref().line_id;
    let stations = &station_service.get_stations_by_line_id(line_id).await?;

    let stations_lines = &line_service.get_stations_lines(stations).await?;
    let response_stations = &line_service
        .get_response_stations_from_stations(stations, stations_lines)
        .await?;

    Ok(MultipleStationResponse {
        stations: response_stations.to_vec(),
    })
}

pub async fn get_stations_by_station_name(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByNameRequest>,
) -> Result<MultipleStationResponse> {
    let station_service = StationService::new(ctx.station_repository());
    let line_service = LineService::new(ctx.line_repository());

    let station_name = &request.get_ref().station_name;
    let limit = &request.get_ref().limit;
    let stations = &station_service
        .get_stations_by_name(station_name, limit)
        .await?;

    let stations_lines = &line_service.get_stations_lines(stations).await?;
    let response_stations = &line_service
        .get_response_stations_from_stations(stations, stations_lines)
        .await?;

    Ok(MultipleStationResponse {
        stations: response_stations.to_vec(),
    })
}
