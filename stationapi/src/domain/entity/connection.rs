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
        let connection = Connection::new(1, 1130101, 1130102, 1.5);
        assert_eq!(
            connection,
            Connection {
                id: 1,
                station_cd1: 1130101,
                station_cd2: 1130102,
                distance: 1.5
            }
        );
    }

    #[test]
    fn new_with_zero_distance() {
        let connection = Connection::new(2, 2200201, 2200202, 0.0);
        assert_eq!(
            connection,
            Connection {
                id: 2,
                station_cd1: 2200201,
                station_cd2: 2200202,
                distance: 0.0
            }
        );
    }

    #[test]
    fn new_with_large_distance() {
        let connection = Connection::new(3, 3300301, 3300302, 999.99);
        assert_eq!(
            connection,
            Connection {
                id: 3,
                station_cd1: 3300301,
                station_cd2: 3300302,
                distance: 999.99
            }
        );
    }

    #[test]
    fn clone_test() {
        let original = Connection::new(4, 4400401, 4400402, 2.3);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn debug_test() {
        let connection = Connection::new(5, 5500501, 5500502, 3.7);
        let debug_string = format!("{connection:?}");
        assert!(debug_string.contains("Connection"));
        assert!(debug_string.contains("id: 5"));
        assert!(debug_string.contains("station_cd1: 5500501"));
        assert!(debug_string.contains("station_cd2: 5500502"));
        assert!(debug_string.contains("distance: 3.7"));
    }

    #[test]
    fn serialization_test() {
        let connection = Connection::new(6, 6600601, 6600602, 4.2);
        let serialized = serde_json::to_string(&connection).unwrap();
        let deserialized: Connection = serde_json::from_str(&serialized).unwrap();
        assert_eq!(connection, deserialized);
    }
}
