use async_trait::async_trait;

use crate::domain::{
    entity::{gtfs::TransportType, station::Station},
    error::DomainError,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectedRoutePatternStop {
    pub line_group_id: u32,
    pub station_station_type_id: i32,
    pub station_group_id: u32,
    pub pass: Option<i32>,
}

#[async_trait]
pub trait StationRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError>;
    async fn get_by_id_vec(&self, ids: &[u32]) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
        direction_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_id_vec(&self, line_ids: &[u32]) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_id_vec_with_group_stations(
        &self,
        line_ids: &[u32],
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_station_group_id_vec_no_types(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Station>, DomainError>;
    /// 座標の近傍から最大 `limit` 件返す。`transport_type` を指定した場合は
    /// 距離の昇順。指定しない場合は鉄道駅を先、バス停を後に並べ、それぞれの中を
    /// 距離の昇順にする (`stationsNearby` の仕様)。件数の上限は並べた後に掛かる
    /// ので、鉄道駅だけで `limit` 件そろえばバス停は返らない。
    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
        transport_type: Option<TransportType>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
        from_station_group_id: Option<u32>,
        transport_type: Option<TransportType>,
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Station>, DomainError>;
    async fn get_by_line_group_id_vec(
        &self,
        line_group_ids: &[u32],
    ) -> Result<Vec<Station>, DomainError>;
    /// Fetch only the fields needed while exploring connected routes.
    ///
    /// The default keeps lightweight test repositories source-compatible.
    /// Production repositories should override this to avoid materializing full
    /// `Station` entities for every explored line group.
    async fn get_connected_route_pattern_stops(
        &self,
        line_group_ids: &[u32],
    ) -> Result<Vec<ConnectedRoutePatternStop>, DomainError> {
        Ok(self
            .get_by_line_group_id_vec(line_group_ids)
            .await?
            .into_iter()
            .filter_map(|stop| {
                Some(ConnectedRoutePatternStop {
                    line_group_id: stop.line_group_cd? as u32,
                    station_station_type_id: stop.sst_id?,
                    station_group_id: stop.station_g_cd as u32,
                    pass: stop.pass,
                })
            })
            .collect())
    }
    /// 各座標から `radius_meters` 以内のバス停を、近い順に最大
    /// `limit_per_station` 件返す。半径の外は呼び出し側でも採用されないため、
    /// ここで切っておく (全国の最寄り N 件を作ってから捨てると、駅数に比例して
    /// 無駄が積み上がる)。
    ///
    /// 半径が有限でない (`NaN` / 無限大) 場合と負の場合は空を返す。無限大を
    /// 距離の比較にそのまま使うと全件が半径内と判定されるため、実装ごとに
    /// 結果が食い違わないようここで決めておく。
    async fn get_bus_stops_near_stations(
        &self,
        coords: &[(u32, f64, f64)], // (station_g_cd, lat, lon)
        limit_per_station: u32,
        radius_meters: f64,
    ) -> Result<Vec<(u32, Station)>, DomainError>;
    async fn get_route_stops(
        &self,
        from_station_id: u32,
        to_station_id: u32,
        via_line_ids: &[u32],
    ) -> Result<Vec<Station>, DomainError>;
    async fn get_route_stops_by_station_cd(
        &self,
        from_station_cd: u32,
        to_station_cd: u32,
        via_line_ids: &[u32],
        direction_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::StopCondition;
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
            _direction_id: Option<u32>,
        ) -> Result<Vec<Station>, DomainError> {
            let result: Vec<Station> = self
                .stations
                .values()
                .filter(|station| station.line_cd == line_id as i32)
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_by_line_id_vec(&self, line_ids: &[u32]) -> Result<Vec<Station>, DomainError> {
            let result: Vec<Station> = self
                .stations
                .values()
                .filter(|station| line_ids.contains(&(station.line_cd as u32)))
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_by_line_id_vec_with_group_stations(
            &self,
            line_ids: &[u32],
        ) -> Result<Vec<Station>, DomainError> {
            let group_ids: Vec<i32> = self
                .stations
                .values()
                .filter(|s| line_ids.contains(&(s.line_cd as u32)))
                .map(|s| s.station_g_cd)
                .collect();
            let result: Vec<Station> = self
                .stations
                .values()
                .filter(|s| group_ids.contains(&s.station_g_cd))
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

        async fn get_by_station_group_id_vec_no_types(
            &self,
            station_group_id_vec: &[u32],
        ) -> Result<Vec<Station>, DomainError> {
            self.get_by_station_group_id_vec(station_group_id_vec).await
        }

        async fn get_by_coordinates(
            &self,
            latitude: f64,
            longitude: f64,
            limit: Option<u32>,
            transport_type: Option<TransportType>,
        ) -> Result<Vec<Station>, DomainError> {
            let mut result: Vec<Station> = self
                .stations
                .values()
                .filter(|station| {
                    transport_type
                        .as_ref()
                        .is_none_or(|tt| station.transport_type == *tt)
                })
                .map(|station| {
                    let mut s = station.clone();
                    let distance = ((station.lat - latitude).powi(2)
                        + (station.lon - longitude).powi(2))
                    .sqrt();
                    s.distance = Some(distance);
                    s
                })
                .collect();

            // trait の契約どおり、鉄道を先・バスを後にしてから距離でソートする。
            // 種別を指定した場合は第 1 キーが定数になるので距離順になる。
            result.sort_by(|a, b| {
                (a.transport_type as i32)
                    .cmp(&(b.transport_type as i32))
                    .then_with(|| {
                        a.distance
                            .partial_cmp(&b.distance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| a.station_cd.cmp(&b.station_cd))
            });

            // 制限があれば適用
            if let Some(limit) = limit {
                result.truncate(limit as usize);
            }

            Ok(result)
        }

        /// trait の契約どおり、半径で絞ってから件数を切る。
        ///
        /// `get_by_coordinates` が `distance` に入れるのは緯度経度の度で測った
        /// ユークリッド距離なので、メートルの半径とは比較できない。ここでは
        /// 距離を測り直す。件数を先に切ると、半径の外の駅が枠を埋めた分だけ
        /// 返る件数が本来より少なくなる。
        async fn get_bus_stops_near_stations(
            &self,
            coords: &[(u32, f64, f64)],
            limit_per_station: u32,
            radius_meters: f64,
        ) -> Result<Vec<(u32, Station)>, DomainError> {
            // 無限大をそのまま比較に使うと全件が半径内になる
            if !radius_meters.is_finite() || radius_meters < 0.0 {
                return Ok(Vec::new());
            }

            /// 球面距離 (m)。
            fn haversine_meters(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
                const EARTH_RADIUS_M: f64 = 6_371_000.0;
                let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
                let dlat = (lat2 - lat1).to_radians();
                let dlon = (lon2 - lon1).to_radians();
                let a =
                    (dlat / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dlon / 2.0).sin().powi(2);
                2.0 * EARTH_RADIUS_M * a.sqrt().clamp(-1.0, 1.0).asin()
            }

            let mut result = Vec::new();
            for &(source_g_cd, lat, lon) in coords {
                let stops = self
                    .get_by_coordinates(lat, lon, None, Some(TransportType::Bus))
                    .await?;
                // get_by_coordinates の並びは度で測ったユークリッド距離順で、
                // 緯度の高い地点では球面距離順と一致しない。件数を切る前に
                // 測り直した距離で並べ直す。
                let mut within: Vec<Station> = stops
                    .into_iter()
                    .filter_map(|mut stop| {
                        let meters = haversine_meters(lat, lon, stop.lat, stop.lon);
                        (meters <= radius_meters).then(|| {
                            stop.distance = Some(meters);
                            stop
                        })
                    })
                    .collect();
                // 元の並びは HashMap の反復順なので、同距離の順序を距離だけに
                // 任せると件数を切ったときにどのバス停が残るか実行ごとに変わる。
                // 索引側 (by_distance_then_station_cd) と同じく station_cd で決める。
                within.sort_by(|a, b| {
                    a.distance
                        .partial_cmp(&b.distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.station_cd.cmp(&b.station_cd))
                });
                within.truncate(limit_per_station as usize);
                result.extend(within.into_iter().map(|stop| (source_g_cd, stop)));
            }
            Ok(result)
        }

        async fn get_by_name(
            &self,
            station_name: String,
            limit: Option<u32>,
            _from_station_group_id: Option<u32>,
            transport_type: Option<TransportType>,
        ) -> Result<Vec<Station>, DomainError> {
            let mut result: Vec<Station> = self
                .stations
                .values()
                .filter(|station| {
                    station.station_name.contains(&station_name)
                        && transport_type
                            .as_ref()
                            .is_none_or(|tt| station.transport_type == *tt)
                })
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

        async fn get_by_line_group_id_vec(
            &self,
            line_group_ids: &[u32],
        ) -> Result<Vec<Station>, DomainError> {
            let result: Vec<Station> = self
                .stations
                .values()
                .filter(|station| {
                    station
                        .line_group_cd
                        .map(|v| line_group_ids.contains(&(v as u32)))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_route_stops(
            &self,
            from_station_id: u32,
            to_station_id: u32,
            via_line_ids: &[u32],
        ) -> Result<Vec<Station>, DomainError> {
            let mut result = Vec::new();

            let from_station = self.stations.get(&from_station_id);
            let to_station = self.stations.get(&to_station_id);

            if !via_line_ids.is_empty() {
                let line_match = |s: &Station| via_line_ids.contains(&(s.line_cd as u32));
                if !from_station.is_some_and(line_match) || !to_station.is_some_and(line_match) {
                    return Ok(result);
                }
            }

            if let Some(from_station) = from_station {
                result.push(from_station.clone());
            }

            if let Some(to_station) = to_station {
                if from_station_id != to_station_id {
                    result.push(to_station.clone());
                }
            }

            Ok(result)
        }

        async fn get_route_stops_by_station_cd(
            &self,
            from_station_cd: u32,
            to_station_cd: u32,
            via_line_ids: &[u32],
            _direction_id: Option<u32>,
        ) -> Result<Vec<Station>, DomainError> {
            self.get_route_stops(from_station_cd, to_station_cd, via_line_ids)
                .await
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
            TransportType::Rail,
        )
    }

    /// 指定した座標にバス停を置いたモック。半径の扱いを検証するために使う。
    fn bus_stop_repository(stops: &[(i32, f64, f64)]) -> MockStationRepository {
        let mut stations = HashMap::new();
        for &(station_cd, lat, lon) in stops {
            let mut stop =
                create_test_station(station_cd, &format!("バス停{station_cd}"), 500, lat, lon);
            stop.transport_type = TransportType::Bus;
            stations.insert(station_cd as u32, stop);
        }
        MockStationRepository { stations }
    }

    /// 指定した座標に種別つきの駅を置いたモック。並び順の検証に使う。
    /// 経度は東京駅に固定し、緯度だけを動かす。
    fn mixed_repository(stations_spec: &[(i32, TransportType, f64)]) -> MockStationRepository {
        let mut stations = HashMap::new();
        for &(station_cd, transport_type, lat) in stations_spec {
            let mut station =
                create_test_station(station_cd, &format!("駅{station_cd}"), 500, lat, 139.767125);
            station.transport_type = transport_type;
            stations.insert(station_cd as u32, station);
        }
        MockStationRepository { stations }
    }

    /// 東京駅から北へおよそ meters メートルの緯度。
    fn lat_north_of_tokyo(meters: f64) -> f64 {
        35.681236 + meters / 111_195.0
    }

    #[tokio::test]
    async fn test_get_bus_stops_near_stations_excludes_stops_outside_the_radius() {
        let repo = bus_stop_repository(&[
            (901, lat_north_of_tokyo(100.0), 139.767125),
            (902, lat_north_of_tokyo(250.0), 139.767125),
            (903, lat_north_of_tokyo(500.0), 139.767125),
        ]);

        let result = repo
            .get_bus_stops_near_stations(&[(1, 35.681236, 139.767125)], 50, 300.0)
            .await
            .unwrap();

        // 300m を超える 903 は含まれず、近い順に並ぶ
        let ids: Vec<i32> = result.iter().map(|(_, s)| s.station_cd).collect();
        assert_eq!(ids, vec![901, 902]);
        // 距離はメートルで入る
        let distances: Vec<f64> = result.iter().map(|(_, s)| s.distance.unwrap()).collect();
        assert!((distances[0] - 100.0).abs() < 5.0, "{distances:?}");
        assert!((distances[1] - 250.0).abs() < 5.0, "{distances:?}");
        // 呼び出し元の座標に紐づく
        assert!(result.iter().all(|(source_g_cd, _)| *source_g_cd == 1));
    }

    /// 件数の上限は半径で絞ったあとに掛ける。先に切ると、半径の外の駅が枠を
    /// 埋めた分だけ返る件数が本来より少なくなる。
    #[tokio::test]
    async fn test_get_bus_stops_near_stations_applies_the_limit_after_the_radius() {
        let repo = bus_stop_repository(&[
            (901, lat_north_of_tokyo(1000.0), 139.767125),
            (902, lat_north_of_tokyo(2000.0), 139.767125),
            (903, lat_north_of_tokyo(100.0), 139.767125),
            (904, lat_north_of_tokyo(200.0), 139.767125),
        ]);

        let result = repo
            .get_bus_stops_near_stations(&[(1, 35.681236, 139.767125)], 2, 300.0)
            .await
            .unwrap();

        // 半径の外にある 901 / 902 が枠を消費しない
        let ids: Vec<i32> = result.iter().map(|(_, s)| s.station_cd).collect();
        assert_eq!(ids, vec![903, 904]);
    }

    /// 同距離の並びは station_cd の昇順。元の並びは HashMap の反復順なので、
    /// 決め切っていないと件数を切ったときの結果が実行ごとに変わる。
    #[tokio::test]
    async fn test_get_bus_stops_near_stations_breaks_ties_by_station_cd() {
        let lat = lat_north_of_tokyo(100.0);
        let repo = bus_stop_repository(&[(903, lat, 139.767125), (901, lat, 139.767125)]);

        let result = repo
            .get_bus_stops_near_stations(&[(1, 35.681236, 139.767125)], 1, 300.0)
            .await
            .unwrap();

        let ids: Vec<i32> = result.iter().map(|(_, s)| s.station_cd).collect();
        assert_eq!(ids, vec![901]);
    }

    /// 座標ごとにまとまり、その中では距離順。
    #[tokio::test]
    async fn test_get_bus_stops_near_stations_groups_by_source_coordinate() {
        let repo = bus_stop_repository(&[
            (901, lat_north_of_tokyo(100.0), 139.767125),
            (902, lat_north_of_tokyo(200.0), 139.767125),
        ]);

        let result = repo
            .get_bus_stops_near_stations(
                &[
                    (1, 35.681236, 139.767125),
                    (2, lat_north_of_tokyo(200.0), 139.767125),
                ],
                50,
                300.0,
            )
            .await
            .unwrap();

        let pairs: Vec<(u32, i32)> = result
            .iter()
            .map(|(source_g_cd, s)| (*source_g_cd, s.station_cd))
            .collect();
        assert_eq!(pairs, vec![(1, 901), (1, 902), (2, 902), (2, 901)]);
    }

    /// 半径が有限でない場合と負の場合は空を返す。無限大をそのまま比較に使うと
    /// 全件が半径内と判定され、本番実装 (index::within_radius) と食い違う。
    #[tokio::test]
    async fn test_get_bus_stops_near_stations_rejects_an_invalid_radius() {
        let repo = bus_stop_repository(&[
            (901, lat_north_of_tokyo(100.0), 139.767125),
            (902, lat_north_of_tokyo(5000.0), 139.767125),
        ]);
        let coords = [(1u32, 35.681236, 139.767125)];

        for radius in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN, -1.0] {
            let result = repo
                .get_bus_stops_near_stations(&coords, 50, radius)
                .await
                .unwrap();
            assert!(result.is_empty(), "半径 {radius} で空にならない");
        }

        // 有限の半径では従来どおり返る
        let result = repo
            .get_bus_stops_near_stations(&coords, 50, 300.0)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
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
        let result = repo.get_by_line_id(1001, None, None).await.unwrap();
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
            .get_by_coordinates(35.681236, 139.767125, Some(2), None)
            .await
            .unwrap();
        assert!(result.len() <= 2);
        assert!(result[0].distance.is_some());
    }

    /// 種別を指定しない座標検索は鉄道駅が先、バス停が後。10m 先のバス停より
    /// 500m 先の鉄道駅が先に来る (`stationsNearby` の仕様)。
    #[tokio::test]
    async fn test_get_by_coordinates_puts_rail_before_bus() {
        let repo = mixed_repository(&[
            (901, TransportType::Bus, lat_north_of_tokyo(10.0)),
            (902, TransportType::Bus, lat_north_of_tokyo(20.0)),
            (101, TransportType::Rail, lat_north_of_tokyo(500.0)),
            (102, TransportType::Rail, lat_north_of_tokyo(400.0)),
        ]);

        let result = repo
            .get_by_coordinates(35.681236, 139.767125, None, None)
            .await
            .unwrap();

        // 鉄道 2 件が先、その中では近い順。バス停はその後
        let ids: Vec<i32> = result.iter().map(|s| s.station_cd).collect();
        assert_eq!(ids, vec![102, 101, 901, 902]);
    }

    /// 件数の上限は種別ごとではなく、並べた後の全体に掛かる。鉄道駅だけで
    /// 埋まる地点ではバス停は返らない。
    #[tokio::test]
    async fn test_get_by_coordinates_fills_the_limit_with_rail_first() {
        let repo = mixed_repository(&[
            (901, TransportType::Bus, lat_north_of_tokyo(10.0)),
            (101, TransportType::Rail, lat_north_of_tokyo(500.0)),
            (102, TransportType::Rail, lat_north_of_tokyo(400.0)),
        ]);

        let result = repo
            .get_by_coordinates(35.681236, 139.767125, Some(2), None)
            .await
            .unwrap();

        let ids: Vec<i32> = result.iter().map(|s| s.station_cd).collect();
        assert_eq!(ids, vec![102, 101]);
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let repo = MockStationRepository::new();
        let result = repo
            .get_by_name("東京".to_string(), None, None, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].station_name, "東京駅");
    }

    #[tokio::test]
    async fn test_get_by_name_with_limit() {
        let repo = MockStationRepository::new();
        let result = repo
            .get_by_name("駅".to_string(), Some(2), None, None)
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
        let result = repo.get_route_stops(1, 2, &[]).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].station_cd, 1);
        assert_eq!(result[1].station_cd, 2);
    }

    #[tokio::test]
    async fn test_get_route_stops_same_station() {
        let repo = MockStationRepository::new();
        let result = repo.get_route_stops(1, 1, &[]).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].station_cd, 1);
    }

    #[tokio::test]
    async fn test_get_route_stops_not_found() {
        let repo = MockStationRepository::new();
        let result = repo.get_route_stops(999, 1000, &[]).await.unwrap();
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_get_route_stops_with_via_line_ids_match() {
        let repo = MockStationRepository::new();
        // 東京駅(1) と 品川駅(4) は line_cd=1001 で一致する
        let result = repo.get_route_stops(1, 4, &[1001]).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].station_cd, 1);
        assert_eq!(result[1].station_cd, 4);
    }

    #[tokio::test]
    async fn test_get_route_stops_with_via_line_ids_mismatch() {
        let repo = MockStationRepository::new();
        // line_cd が一致しないためルートは返さない
        let result = repo.get_route_stops(1, 2, &[1001]).await.unwrap();
        assert!(result.is_empty());
    }
}
