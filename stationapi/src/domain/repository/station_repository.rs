use async_trait::async_trait;

use crate::domain::{entity::station::Station, error::DomainError};

#[async_trait]
pub trait StationRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError>;
    async fn get_by_id_vec(&self, ids: &[u32]) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
        from_station_group_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Station>, DomainError>;
    async fn get_route_stops(
        &self,
        from_station_id: u32,
        to_station_id: u32,
    ) -> Result<Vec<Station>, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::StopCondition;
    use std::collections::HashMap;

    // モック実装
    pub struct MockStationRepository {
        stations: HashMap<u32, Station>,
    }

    impl MockStationRepository {
        pub fn new() -> Self {
            let mut stations = HashMap::new();

            // テストデータを作成
            let station1 = create_test_station(1, "東京駅", 1001, 35.681236, 139.767125);
            let station2 = create_test_station(2, "新宿駅", 1002, 35.690921, 139.700258);
            let station3 = create_test_station(3, "渋谷駅", 1003, 35.659518, 139.700464);
            let station4 = create_test_station(4, "品川駅", 1001, 35.630152, 139.740570);

            stations.insert(1, station1);
            stations.insert(2, station2);
            stations.insert(3, station3);
            stations.insert(4, station4);

            Self { stations }
        }
    }

    #[async_trait]
    impl StationRepository for MockStationRepository {
        async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError> {
            Ok(self.stations.get(&id).cloned())
        }

        async fn get_by_id_vec(&self, ids: &[u32]) -> Result<Vec<Station>, DomainError> {
            let mut result = Vec::new();
            for id in ids {
                if let Some(station) = self.stations.get(id) {
                    result.push(station.clone());
                }
            }
            Ok(result)
        }

        async fn get_by_line_id(
            &self,
            line_id: u32,
            _station_id: Option<u32>,
        ) -> Result<Vec<Station>, DomainError> {
            let result: Vec<Station> = self
                .stations
                .values()
                .filter(|station| station.line_cd == line_id as i32)
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_by_station_group_id(
            &self,
            station_group_id: u32,
        ) -> Result<Vec<Station>, DomainError> {
            let result: Vec<Station> = self
                .stations
                .values()
                .filter(|station| station.station_g_cd == station_group_id as i32)
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_by_station_group_id_vec(
            &self,
            station_group_id_vec: &[u32],
        ) -> Result<Vec<Station>, DomainError> {
            let result: Vec<Station> = self
                .stations
                .values()
                .filter(|station| station_group_id_vec.contains(&(station.station_g_cd as u32)))
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_by_coordinates(
            &self,
            latitude: f64,
            longitude: f64,
            limit: Option<u32>,
        ) -> Result<Vec<Station>, DomainError> {
            let mut result: Vec<Station> = self
                .stations
                .values()
                .map(|station| {
                    let mut s = station.clone();
                    let distance = ((station.lat - latitude).powi(2)
                        + (station.lon - longitude).powi(2))
                    .sqrt();
                    s.distance = Some(distance);
                    s
                })
                .collect();

            // 距離でソート
            result.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

            // 制限があれば適用
            if let Some(limit) = limit {
                result.truncate(limit as usize);
            }

            Ok(result)
        }

        async fn get_by_name(
            &self,
            station_name: String,
            limit: Option<u32>,
            _from_station_group_id: Option<u32>,
        ) -> Result<Vec<Station>, DomainError> {
            let mut result: Vec<Station> = self
                .stations
                .values()
                .filter(|station| station.station_name.contains(&station_name))
                .cloned()
                .collect();

            if let Some(limit) = limit {
                result.truncate(limit as usize);
            }

            Ok(result)
        }

        async fn get_by_line_group_id(
            &self,
            line_group_id: u32,
        ) -> Result<Vec<Station>, DomainError> {
            let result: Vec<Station> = self
                .stations
                .values()
                .filter(|station| station.line_group_cd == Some(line_group_id as i32))
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_route_stops(
            &self,
            from_station_id: u32,
            to_station_id: u32,
        ) -> Result<Vec<Station>, DomainError> {
            // 簡単なルート検索のモック実装
            let mut result = Vec::new();

            if let Some(from_station) = self.stations.get(&from_station_id) {
                result.push(from_station.clone());
            }

            if let Some(to_station) = self.stations.get(&to_station_id) {
                if from_station_id != to_station_id {
                    result.push(to_station.clone());
                }
            }

            Ok(result)
        }
    }

    // テスト用のStation作成ヘルパー関数
    fn create_test_station(
        station_cd: i32,
        station_name: &str,
        line_cd: i32,
        lat: f64,
        lon: f64,
    ) -> Station {
        Station::new(
            station_cd,
            station_cd, // station_g_cd
            station_name.to_string(),
            format!("{station_name}_k"),
            Some(format!("{station_name}_r")),
            Some(format!("{station_name}_zh")),
            Some(format!("{station_name}_ko")),
            vec![],
            None,
            None,
            None,
            None,
            None,
            line_cd,
            None,
            vec![],
            13, // 東京都
            "100-0000".to_string(),
            "東京都".to_string(),
            lon,
            lat,
            "20000101".to_string(),
            "99991231".to_string(),
            0,
            0,
            StopCondition::All,
            None,
            false,
            None,
            Some(1),
            Some("山手線".to_string()),
            Some("やまのてせん".to_string()),
            Some("Yamanote Line".to_string()),
            Some("Yamanote Line".to_string()),
            Some("山手线".to_string()),
            Some("야마노테선".to_string()),
            Some("#00AC9A".to_string()),
            Some(1),
            Some("JY".to_string()),
            None,
            None,
            None,
            Some("#00AC9A".to_string()),
            None,
            None,
            None,
            Some("circle".to_string()),
            None,
            None,
            None,
            Some(1000),
            Some(5.5),
            Some(0),
            Some(1),
            Some(1),
            Some(1),
            Some("普通".to_string()),
            Some("ふつう".to_string()),
            Some("Local".to_string()),
            Some("普通".to_string()),
            Some("보통".to_string()),
            Some("#000000".to_string()),
            Some(0),
            Some(1),
        )
    }

    #[tokio::test]
    async fn test_find_by_id_existing() {
        let repo = MockStationRepository::new();
        let result = repo.find_by_id(1).await.unwrap();
        assert!(result.is_some());
        let station = result.unwrap();
        assert_eq!(station.station_cd, 1);
        assert_eq!(station.station_name, "東京駅");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let repo = MockStationRepository::new();
        let result = repo.find_by_id(999).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_vec() {
        let repo = MockStationRepository::new();
        let ids = vec![1, 3, 999]; // 999は存在しない
        let result = repo.get_by_id_vec(&ids).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|s| s.station_cd == 1));
        assert!(result.iter().any(|s| s.station_cd == 3));
    }

    #[tokio::test]
    async fn test_get_by_line_id() {
        let repo = MockStationRepository::new();
        let result = repo.get_by_line_id(1001, None).await.unwrap();
        assert_eq!(result.len(), 2); // 東京駅と品川駅
        assert!(result.iter().all(|s| s.line_cd == 1001));
    }

    #[tokio::test]
    async fn test_get_by_station_group_id() {
        let repo = MockStationRepository::new();
        let result = repo.get_by_station_group_id(1).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].station_cd, 1);
    }

    #[tokio::test]
    async fn test_get_by_station_group_id_vec() {
        let repo = MockStationRepository::new();
        let group_ids = vec![1, 2];
        let result = repo.get_by_station_group_id_vec(&group_ids).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_coordinates() {
        let repo = MockStationRepository::new();
        // 東京駅付近の座標
        let result = repo
            .get_by_coordinates(35.681236, 139.767125, Some(2))
            .await
            .unwrap();
        assert!(result.len() <= 2);
        assert!(result[0].distance.is_some());
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let repo = MockStationRepository::new();
        let result = repo
            .get_by_name("東京".to_string(), None, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].station_name, "東京駅");
    }

    #[tokio::test]
    async fn test_get_by_name_with_limit() {
        let repo = MockStationRepository::new();
        let result = repo
            .get_by_name("駅".to_string(), Some(2), None)
            .await
            .unwrap();
        assert!(result.len() <= 2);
    }

    #[tokio::test]
    async fn test_get_by_line_group_id() {
        let repo = MockStationRepository::new();
        let result = repo.get_by_line_group_id(1000).await.unwrap();
        assert_eq!(result.len(), 4); // すべての駅がline_group_cd = 1000に設定されている
    }

    #[tokio::test]
    async fn test_get_route_stops() {
        let repo = MockStationRepository::new();
        let result = repo.get_route_stops(1, 2).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].station_cd, 1);
        assert_eq!(result[1].station_cd, 2);
    }

    #[tokio::test]
    async fn test_get_route_stops_same_station() {
        let repo = MockStationRepository::new();
        let result = repo.get_route_stops(1, 1).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].station_cd, 1);
    }

    #[tokio::test]
    async fn test_get_route_stops_not_found() {
        let repo = MockStationRepository::new();
        let result = repo.get_route_stops(999, 1000).await.unwrap();
        assert_eq!(result.len(), 0);
    }
}
