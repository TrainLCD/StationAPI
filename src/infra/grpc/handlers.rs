use anyhow::Result;

use crate::{
    application::station_service::StationService,
    pb::{
        GetStationByCoordinatesRequest, GetStationByGroupIdRequest, GetStationByIdRequest,
        GetStationByLineIdRequest, GetStationByNameRequest, MultipleStationResponse,
        SingleStationResponse,
    },
};

use super::router;

pub async fn get_station_by_id(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByIdRequest>,
) -> Result<SingleStationResponse> {
    let service = StationService::new(ctx.station_repository());
    let id = request.get_ref().id;
    let data = service.find_by_id(id).await?;
    Ok(SingleStationResponse {
        station: Some(data),
    })
}
pub async fn get_station_by_group_id(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByGroupIdRequest>,
) -> Result<MultipleStationResponse> {
    let service = StationService::new(ctx.station_repository());
    let group_id = request.get_ref().group_id;
    let data = service.get_by_group_id(group_id).await?;
    Ok(MultipleStationResponse { stations: data })
}
pub async fn get_station_by_coordinates(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByCoordinatesRequest>,
) -> Result<MultipleStationResponse> {
    let service = StationService::new(ctx.station_repository());
    let latitude = request.get_ref().latitude;
    let longitude = request.get_ref().longitude;
    let limit = request.get_ref().limit;
    let data = service
        .get_station_by_coordinates(latitude, longitude, limit)
        .await?;
    Ok(MultipleStationResponse { stations: data })
}
pub async fn get_stations_by_line_id(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByLineIdRequest>,
) -> Result<MultipleStationResponse> {
    let service = StationService::new(ctx.station_repository());
    let line_id = request.get_ref().line_id;
    let data = service.get_stations_by_line_id(line_id).await?;
    Ok(MultipleStationResponse { stations: data })
}
pub async fn get_stations_by_station_name(
    ctx: &router::ApiContext,
    request: tonic::Request<GetStationByNameRequest>,
) -> Result<MultipleStationResponse> {
    let service = StationService::new(ctx.station_repository());
    let station_name = &request.get_ref().station_name;
    let data = service.get_stations_by_name(station_name).await?;
    Ok(MultipleStationResponse { stations: data })
}
