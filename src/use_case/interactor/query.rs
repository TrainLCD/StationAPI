use async_trait::async_trait;

use crate::{
    domain::{entity::station::Station, repository::station_repository::StationRepository},
    use_case::{error::UseCaseError, traits::query::QueryUseCase},
};

#[derive(Debug, Clone)]
pub struct QueryInteractor<SR> {
    pub station_repository: SR,
}

#[async_trait]
impl<SR> QueryUseCase for QueryInteractor<SR>
where
    SR: StationRepository,
{
    async fn find_station_by_id(&self, station_id: u32) -> Result<Option<Station>, UseCaseError> {
        let station = self.station_repository.find_by_id(station_id).await?;
        Ok(station)
    }

    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_stations_group_id(station_group_id)
            .await?;
        Ok(stations)
    }
    async fn get_stations_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_stations_by_coordinates(latitude, longitude, limit)
            .await?;
        Ok(stations)
    }
    async fn get_stations_by_line_id(&self, line_id: u32) -> Result<Vec<Station>, UseCaseError> {
        let stations = self.station_repository.get_by_line_id(line_id).await?;
        Ok(stations)
    }
    async fn get_stations_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_stations_by_name(station_name, limit)
            .await?;
        Ok(stations)
    }
}
