use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StationIdWithDistance {
    pub station_id: i32,
    pub distance: f64,
    pub average_distance: f64,
}

impl StationIdWithDistance {
    pub fn new(station_id: i32, distance: f64, average_distance: f64) -> Self {
        Self {
            station_id,
            distance,
            average_distance,
        }
    }
}
