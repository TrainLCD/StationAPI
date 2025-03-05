use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Connection {
    pub id: u32,
    pub station_cd1: u32,
    pub station_cd2: u32,
    pub distance: f64,
}

impl Connection {
    pub fn new(id: u32, station_cd1: u32, station_cd2: u32, distance: f64) -> Self {
        Self {
            id,
            station_cd1,
            station_cd2,
            distance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Connection;

    #[test]
    fn new() {
        let connection = Connection::new(1, 100201, 100202, 6140.152858);
        assert_eq!(
            connection,
            Connection {
                id: 1,
                station_cd1: 100201,
                station_cd2: 100202,
                distance: 6140.152858
            }
        );
    }
}
