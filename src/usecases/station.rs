use crate::{entities::station::Station, repositories::station::StationRepository};

pub async fn find_one_station(repository: impl StationRepository, id: i32) -> Option<Station> {
    repository.find_one(id).await
}
