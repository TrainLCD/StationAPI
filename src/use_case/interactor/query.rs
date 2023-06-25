use anyhow::Result;
use async_trait::async_trait;
use futures::future::try_join_all;

use crate::{
    domain::{
        entity::{line::Line, station::Station},
        repository::{line_repository::LineRepository, station_repository::StationRepository},
    },
    use_case::{error::UseCaseError, traits::query::QueryUseCase},
};

#[derive(Debug, Clone)]
pub struct QueryInteractor<SR, LR> {
    pub station_repository: SR,
    pub line_repository: LR,
}

#[async_trait]
impl<SR, LR> QueryUseCase for QueryInteractor<SR, LR>
where
    SR: StationRepository,
    LR: LineRepository,
{
    async fn find_station_by_id(&self, station_id: u32) -> Result<Option<Station>, UseCaseError> {
        let mut station = match self.station_repository.find_by_id(station_id).await {
            Ok(Some(station)) => station,
            Ok(None) => {
                return Err(UseCaseError::NotFound {
                    entity_type: "Station",
                    entity_id: station_id.to_string(),
                })
            }
            Err(err) => return Err(UseCaseError::Unexpected(err.to_string())),
        };
        let line = match self.find_line_by_id(station.line_cd).await {
            Ok(Some(line)) => line,
            Ok(None) => {
                return Err(UseCaseError::NotFound {
                    entity_type: "Line",
                    entity_id: station_id.to_string(),
                })
            }
            Err(err) => return Err(UseCaseError::Unexpected(err.to_string())),
        };

        let lines = match self
            .get_lines_by_station_group_id(station.station_g_cd)
            .await
        {
            Ok(lines) => lines,
            Err(err) => return Err(UseCaseError::Unexpected(err.to_string())),
        };

        station.set_line(Some(line));
        station.set_lines(lines);

        Ok(Some(station))
    }

    async fn get_stations_by_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, UseCaseError> {
        let stations = self
            .station_repository
            .get_by_station_group_id(station_group_id)
            .await?;

        let line_ids = stations.iter().map(|station| station.line_cd).collect();

        let belong_lines = match self.get_lines_by_ids(line_ids).await {
            Ok(lines) => lines,
            Err(err) => return Err(UseCaseError::Unexpected(err.to_string())),
        };

        let lines = match self.get_lines_by_station_group_id(station_group_id).await {
            Ok(lines) => lines,
            Err(err) => return Err(UseCaseError::Unexpected(err.to_string())),
        };

        let stations = stations
            .into_iter()
            .enumerate()
            .map(|(index, mut station)| {
                station.set_line(belong_lines.get(index).cloned());
                station.set_lines(lines.clone());
                station
            })
            .collect();

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

        let station_group_ids: Vec<u32> = stations
            .iter()
            .map(|station| station.station_g_cd)
            .collect();
        let line_ids: Vec<u32> = stations.iter().map(|station| station.line_cd).collect();
        let stations_belong_lines: Vec<Option<Line>> = try_join_all(
            line_ids
                .iter()
                .map(|line_id| self.find_line_by_id(*line_id)),
        )
        .await?;

        let stations_lines: Vec<Vec<Line>> = try_join_all(
            station_group_ids
                .iter()
                .map(|group_id| self.get_lines_by_station_group_id(*group_id)),
        )
        .await?;

        let stations = stations
            .into_iter()
            .enumerate()
            .map(|(index, mut station)| {
                let line = match stations_belong_lines.get(index) {
                    Some(line) => line.clone(),
                    None => None,
                };
                station.set_line(line);
                station.set_lines(stations_lines.get(index).cloned().unwrap_or(vec![]));
                station
            })
            .collect();

        Ok(stations)
    }
    async fn get_stations_by_line_id(&self, line_id: u32) -> Result<Vec<Station>, UseCaseError> {
        let stations = self.station_repository.get_by_line_id(line_id).await?;

        let station_group_ids: Vec<u32> = stations
            .iter()
            .map(|station| station.station_g_cd)
            .collect();
        let line_ids: Vec<u32> = stations.iter().map(|station| station.line_cd).collect();
        let stations_belong_lines: Vec<Option<Line>> = try_join_all(
            line_ids
                .iter()
                .map(|line_id| self.find_line_by_id(*line_id)),
        )
        .await?;

        let stations_lines: Vec<Vec<Line>> = try_join_all(
            station_group_ids
                .iter()
                .map(|group_id| self.get_lines_by_station_group_id(*group_id)),
        )
        .await?;

        let stations = stations
            .into_iter()
            .enumerate()
            .map(|(index, mut station)| {
                let line = match stations_belong_lines.get(index) {
                    Some(line) => line.clone(),
                    None => None,
                };
                station.set_line(line);
                station.set_lines(stations_lines.get(index).cloned().unwrap_or(vec![]));
                station
            })
            .collect();
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

        let station_group_ids: Vec<u32> = stations
            .iter()
            .map(|station| station.station_g_cd)
            .collect();
        let line_ids: Vec<u32> = stations.iter().map(|station| station.line_cd).collect();
        let stations_belong_lines: Vec<Option<Line>> = try_join_all(
            line_ids
                .iter()
                .map(|line_id| self.find_line_by_id(*line_id)),
        )
        .await?;

        let stations_lines: Vec<Vec<Line>> = try_join_all(
            station_group_ids
                .iter()
                .map(|group_id| self.get_lines_by_station_group_id(*group_id)),
        )
        .await?;

        let stations = stations
            .into_iter()
            .enumerate()
            .map(|(index, mut station)| {
                let line = match stations_belong_lines.get(index) {
                    Some(line) => line.clone(),
                    None => None,
                };
                station.set_line(line);
                station.set_lines(stations_lines.get(index).cloned().unwrap_or(vec![]));
                station
            })
            .collect();
        Ok(stations)
    }
    async fn find_line_by_id(&self, line_id: u32) -> Result<Option<Line>, UseCaseError> {
        let line = self.line_repository.find_by_id(line_id).await?;
        Ok(line)
    }
    async fn get_lines_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Line>, UseCaseError> {
        let lines = self
            .line_repository
            .get_by_station_group_id(station_group_id)
            .await?;
        Ok(lines)
    }
    async fn get_lines_by_ids(&self, line_ids: Vec<u32>) -> Result<Vec<Line>, UseCaseError> {
        let lines = self.line_repository.get_by_ids(line_ids).await?;
        Ok(lines)
    }
}
