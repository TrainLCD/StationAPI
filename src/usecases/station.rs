use crate::{
    repositories::station::StationRepository,
    service::{MultipleStationResponse, SingleStationResponse},
};

pub async fn find_station_by_id(
    repository: impl StationRepository,
    id: u32,
) -> Option<SingleStationResponse> {
    if let Some(station) = repository.find_one(id).await {
        return Some(SingleStationResponse {
            station: Some(station.into()),
        });
    }
    None
}

pub async fn find_station_by_coordinates(
    repository: impl StationRepository,
    latitude: f64,
    longitude: f64,
    limit: Option<i32>,
) -> MultipleStationResponse {
    if let Some(stations) = repository
        .get_by_coordinates(latitude, longitude, limit)
        .await
    {
        return MultipleStationResponse {
            stations: stations.into_iter().map(|s| s.into()).collect(),
        };
    }
    MultipleStationResponse { stations: vec![] }
}
