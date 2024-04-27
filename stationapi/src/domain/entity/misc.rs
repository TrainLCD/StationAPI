use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StationIdWithDistance {
    pub station_id: u32,
    pub distance: f64,
}

impl StationIdWithDistance {
    pub fn new(station_id: u32, distance: f64) -> Self {
        Self {
            station_id,
            distance,
        }
    }
}
