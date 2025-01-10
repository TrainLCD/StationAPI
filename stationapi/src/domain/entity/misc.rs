use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StationIdWithDistance {
    pub station_id: u32,
    pub distance: f64,
    pub average_distance: f64,
}

impl StationIdWithDistance {
    pub fn new(station_id: u32, distance: f64, average_distance: f64) -> Self {
        Self {
            station_id,
            distance,
            average_distance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StationIdWithDistance;

    #[test]
    fn new() {
        let station_with_distance = StationIdWithDistance::new(1001, 2.5, 3.0);
        assert_eq!(
            station_with_distance,
            StationIdWithDistance {
                station_id: 1001,
                distance: 2.5,
                average_distance: 3.0
            }
        );
    }
}
