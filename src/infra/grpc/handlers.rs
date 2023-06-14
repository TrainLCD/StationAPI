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
    let lines_response = lines
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
    let station_response: StationResponse = data.into();
    let line_symbols = line_service.get_line_symbols(&mut station_line);
    line_response.line_symbols = line_symbols;
    Ok(SingleStationResponse {
        station: Some(station_response),
        line: Some(line_response),
        lines: lines_response,
    })
}
pub async fn get_station_by_group_id(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByGroupIdRequest>,
) -> Result<MultipleStationResponse> {
    let station_service = StationService::new(ctx.station_repository());
    let line_service = LineService::new(ctx.line_repository());

    let group_id = request.get_ref().group_id;
    let stations = station_service.get_by_group_id(group_id).await?;
    let station_group_ids: Vec<u32> = stations
        .iter()
        .map(|station| station.station_g_cd)
        .collect();

    let mut lines = vec![];
    for station_group_id in station_group_ids {
        let line = line_service
            .get_by_station_group_id(station_group_id)
            .await?;
        lines.push(line);
    }

    Ok(MultipleStationResponse {
        data: stations
            .into_iter()
            .enumerate()
            .map(|(index, station)| {
                let station_response: StationResponse = station.clone().into();
                let station_lines = lines.get(index).unwrap();
                let station_line = station_lines
                    .iter()
                    .find(|line| line.line_cd == station.line_cd)
                    .unwrap();
                let station_lines_response: Vec<LineResponse> = station_lines
                    .iter()
                    .map(|line| {
                        let mut line_response: LineResponse = line.clone().into();
                        let line_symbols = line_service.get_line_symbols(&mut line.clone());
                        line_response.line_symbols = line_symbols;
                        line_response
                    })
                    .collect();

                let mut line_response: LineResponse = station_line.clone().into();

                let line_symbols = line_service.get_line_symbols(&mut station_line.clone());
                line_response.line_symbols = line_symbols;

                SingleStationResponse {
                    station: Some(station_response),
                    lines: station_lines_response,
                    line: Some(line_response),
                }
            })
            .collect(),
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
    let limit = request.get_ref().limit;
    let stations = station_service
        .get_station_by_coordinates(latitude, longitude, limit)
        .await?;
    let station_group_ids: Vec<u32> = stations
        .iter()
        .map(|station| station.station_g_cd)
        .collect();

    let mut lines = vec![];
    for station_group_id in station_group_ids {
        let line = line_service
            .get_by_station_group_id(station_group_id)
            .await?;
        lines.push(line);
    }

    Ok(MultipleStationResponse {
        data: stations
            .into_iter()
            .enumerate()
            .map(|(index, station)| {
                let station_response: StationResponse = station.clone().into();
                let station_lines = lines.get(index).unwrap();
                let station_line = station_lines
                    .iter()
                    .find(|line| line.line_cd == station.line_cd)
                    .unwrap();
                let station_lines_response: Vec<LineResponse> = station_lines
                    .iter()
                    .map(|line| {
                        let mut line_response: LineResponse = line.clone().into();
                        let line_symbols = line_service.get_line_symbols(&mut line.clone());
                        line_response.line_symbols = line_symbols;
                        line_response
                    })
                    .collect();

                let mut line_response: LineResponse = station_line.clone().into();

                let line_symbols = line_service.get_line_symbols(&mut station_line.clone());

                line_response.line_symbols = line_symbols;

                SingleStationResponse {
                    station: Some(station_response),
                    lines: station_lines_response,
                    line: Some(line_response),
                }
            })
            .collect(),
    })
}

pub async fn get_stations_by_line_id(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByLineIdRequest>,
) -> Result<MultipleStationResponse> {
    let station_service = StationService::new(ctx.station_repository());
    let line_service = LineService::new(ctx.line_repository());

    let line_id: u32 = request.get_ref().line_id;
    let stations = station_service.get_stations_by_line_id(line_id).await?;
    let station_group_ids: Vec<u32> = stations
        .iter()
        .map(|station| station.station_g_cd)
        .collect();

    let mut lines = vec![];
    for station_group_id in station_group_ids {
        let line = line_service
            .get_by_station_group_id(station_group_id)
            .await?;
        lines.push(line);
    }

    Ok(MultipleStationResponse {
        data: stations
            .into_iter()
            .enumerate()
            .map(|(index, station)| {
                let station_response: StationResponse = station.clone().into();
                let station_lines = lines.get(index).unwrap();
                let station_line = station_lines
                    .iter()
                    .find(|line| line.line_cd == station.line_cd)
                    .unwrap();
                let station_lines_response: Vec<LineResponse> = station_lines
                    .iter()
                    .map(|line| {
                        let mut line_response: LineResponse = line.clone().into();
                        let line_symbols = line_service.get_line_symbols(&mut line.clone());
                        line_response.line_symbols = line_symbols;
                        line_response
                    })
                    .collect();

                let mut line_response: LineResponse = station_line.clone().into();

                let line_symbols = line_service.get_line_symbols(&mut station_line.clone());
                line_response.line_symbols = line_symbols;

                SingleStationResponse {
                    station: Some(station_response),
                    lines: station_lines_response,
                    line: Some(line_response),
                }
            })
            .collect(),
    })
}

pub async fn get_stations_by_station_name(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByNameRequest>,
) -> Result<MultipleStationResponse> {
    let station_service = StationService::new(ctx.station_repository());
    let line_service = LineService::new(ctx.line_repository());

    let station_name = &request.get_ref().station_name;
    let stations = station_service.get_stations_by_name(station_name).await?;
    let station_group_ids: Vec<u32> = stations
        .iter()
        .map(|station| station.station_g_cd)
        .collect();

    let mut lines = vec![];
    for station_group_id in station_group_ids {
        let line = line_service
            .get_by_station_group_id(station_group_id)
            .await?;
        lines.push(line);
    }

    Ok(MultipleStationResponse {
        data: stations
            .into_iter()
            .enumerate()
            .map(|(index, station)| {
                let station_response: StationResponse = station.clone().into();
                let station_lines = lines.get(index).unwrap();
                let station_line = station_lines
                    .iter()
                    .find(|line| line.line_cd == station.line_cd)
                    .unwrap();
                let station_lines_response: Vec<LineResponse> = station_lines
                    .iter()
                    .map(|line| {
                        let mut line_response: LineResponse = line.clone().into();
                        let line_symbols = line_service.get_line_symbols(&mut line.clone());
                        line_response.line_symbols = line_symbols;
                        line_response
                    })
                    .collect();

                let mut line_response: LineResponse = station_line.clone().into();

                let line_symbols = line_service.get_line_symbols(&mut station_line.clone());
                line_response.line_symbols = line_symbols;

                SingleStationResponse {
                    station: Some(station_response),
                    lines: station_lines_response,
                    line: Some(line_response),
                }
            })
            .collect(),
    })
}
