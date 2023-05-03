use crate::{dao::station::StationDao, entities::station::Station};

pub struct StationService<T: for<'a> StationDao<'a>>(pub T);

impl<T: for<'a> StationDao<'a>> StationService<T> {
    // pub fn new(station_dao: T) -> Self {
    //     Self(station_dao)
    // }
    pub async fn find_one(&self, id: i64) -> Option<Station> {
        self.0.find_one(id).await.ok()
    }
}
