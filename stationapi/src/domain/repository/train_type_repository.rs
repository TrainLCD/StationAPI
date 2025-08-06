use async_trait::async_trait;

use crate::domain::{entity::train_type::TrainType, error::DomainError};

#[async_trait]
pub trait TrainTypeRepository: Send + Sync + 'static {
    async fn get_by_line_group_id(&self, line_group_id: u32)
        -> Result<Vec<TrainType>, DomainError>;
    async fn get_by_station_id(&self, station_id: u32) -> Result<Vec<TrainType>, DomainError>;
    async fn find_by_line_group_id_and_line_id(
        &self,
        line_group_id: u32,
        line_id: u32,
    ) -> Result<Option<TrainType>, DomainError>;
    async fn get_by_station_id_vec(
        &self,
        station_id_vec: &[u32],
        line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, DomainError>;
    async fn get_types_by_station_id_vec(
        &self,
        station_id_vec: &[u32],
        line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, DomainError>;
    async fn get_by_line_group_id_vec(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<TrainType>, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // モック実装
    pub struct MockTrainTypeRepository {
        train_types: HashMap<i32, TrainType>,
    }

    impl MockTrainTypeRepository {
        pub fn new() -> Self {
            let mut train_types = HashMap::new();

            // テストデータを作成
            let train_type1 = create_test_train_type(
                1,
                100201,
                1,
                1,
                "のぞみ",
                "ノゾミ",
                Some("Nozomi"),
                "#FFD400",
                4,
            );
            let train_type2 = create_test_train_type(
                2,
                100202,
                2,
                1,
                "ひかり",
                "ヒカリ",
                Some("Hikari"),
                "#0070F0",
                4,
            );
            let train_type3 = create_test_train_type(
                3,
                100203,
                3,
                2,
                "急行",
                "キュウコウ",
                Some("Express"),
                "#FF6600",
                2,
            );
            let train_type4 = create_test_train_type(
                4,
                100201,
                4,
                1,
                "こだま",
                "コダマ",
                Some("Kodama"),
                "#00AA00",
                4,
            );

            train_types.insert(1, train_type1);
            train_types.insert(2, train_type2);
            train_types.insert(3, train_type3);
            train_types.insert(4, train_type4);

            Self { train_types }
        }
    }

    #[async_trait]
    impl TrainTypeRepository for MockTrainTypeRepository {
        async fn get_by_line_group_id(
            &self,
            line_group_id: u32,
        ) -> Result<Vec<TrainType>, DomainError> {
            let result: Vec<TrainType> = self
                .train_types
                .values()
                .filter(|train_type| train_type.line_group_cd == Some(line_group_id as i32))
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_by_station_id(&self, station_id: u32) -> Result<Vec<TrainType>, DomainError> {
            let result: Vec<TrainType> = self
                .train_types
                .values()
                .filter(|train_type| train_type.station_cd == Some(station_id as i32))
                .cloned()
                .collect();
            Ok(result)
        }

        async fn find_by_line_group_id_and_line_id(
            &self,
            line_group_id: u32,
            line_id: u32,
        ) -> Result<Option<TrainType>, DomainError> {
            let result = self
                .train_types
                .values()
                .find(|train_type| {
                    train_type.line_group_cd == Some(line_group_id as i32)
                        && train_type.type_cd == Some(line_id as i32)
                })
                .cloned();
            Ok(result)
        }

        async fn get_by_station_id_vec(
            &self,
            station_id_vec: &[u32],
            line_group_id: Option<u32>,
        ) -> Result<Vec<TrainType>, DomainError> {
            let result: Vec<TrainType> = self
                .train_types
                .values()
                .filter(|train_type| {
                    let station_match = train_type
                        .station_cd
                        .map(|cd| station_id_vec.contains(&(cd as u32)))
                        .unwrap_or(false);

                    let line_group_match = match line_group_id {
                        Some(group_id) => train_type.line_group_cd == Some(group_id as i32),
                        None => true,
                    };

                    station_match && line_group_match
                })
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_types_by_station_id_vec(
            &self,
            station_id_vec: &[u32],
            line_group_id: Option<u32>,
        ) -> Result<Vec<TrainType>, DomainError> {
            // この実装では get_by_station_id_vec と同じ動作とする
            self.get_by_station_id_vec(station_id_vec, line_group_id)
                .await
        }

        async fn get_by_line_group_id_vec(
            &self,
            line_group_id_vec: &[u32],
        ) -> Result<Vec<TrainType>, DomainError> {
            let result: Vec<TrainType> = self
                .train_types
                .values()
                .filter(|train_type| {
                    train_type
                        .line_group_cd
                        .map(|cd| line_group_id_vec.contains(&(cd as u32)))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
            Ok(result)
        }
    }

    // テスト用のTrainType作成ヘルパー関数
    fn create_test_train_type(
        id: i32,
        station_cd: i32,
        type_cd: i32,
        line_group_cd: i32,
        type_name: &str,
        type_name_k: &str,
        type_name_r: Option<&str>,
        color: &str,
        kind: i32,
    ) -> TrainType {
        #![allow(clippy::too_many_arguments)]
        TrainType::new(
            Some(id),
            Some(station_cd),
            Some(type_cd),
            Some(line_group_cd),
            Some(0),
            type_name.to_string(),
            type_name_k.to_string(),
            type_name_r.map(|s| s.to_string()),
            Some(format!("{type_name}_zh")),
            Some(format!("{type_name}_ko")),
            color.to_string(),
            Some(0),
            Some(kind),
        )
    }

    #[tokio::test]
    async fn test_get_by_line_group_id() {
        let repository = MockTrainTypeRepository::new();
        let result = repository.get_by_line_group_id(1).await.unwrap();

        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|t| t.line_group_cd == Some(1)));
    }

    #[tokio::test]
    async fn test_get_by_line_group_id_empty() {
        let repository = MockTrainTypeRepository::new();
        let result = repository.get_by_line_group_id(999).await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_by_station_id() {
        let repository = MockTrainTypeRepository::new();
        let result = repository.get_by_station_id(100201).await.unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|t| t.station_cd == Some(100201)));
    }

    #[tokio::test]
    async fn test_get_by_station_id_empty() {
        let repository = MockTrainTypeRepository::new();
        let result = repository.get_by_station_id(999999).await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_find_by_line_group_id_and_line_id_found() {
        let repository = MockTrainTypeRepository::new();
        let result = repository
            .find_by_line_group_id_and_line_id(1, 1)
            .await
            .unwrap();

        assert!(result.is_some());
        let train_type = result.unwrap();
        assert_eq!(train_type.line_group_cd, Some(1));
        assert_eq!(train_type.type_cd, Some(1));
        assert_eq!(train_type.type_name, "のぞみ");
    }

    #[tokio::test]
    async fn test_find_by_line_group_id_and_line_id_not_found() {
        let repository = MockTrainTypeRepository::new();
        let result = repository
            .find_by_line_group_id_and_line_id(999, 999)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_station_id_vec() {
        let repository = MockTrainTypeRepository::new();
        let station_ids = vec![100201, 100202];
        let result = repository
            .get_by_station_id_vec(&station_ids, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 3);
        assert!(result
            .iter()
            .all(|t| { t.station_cd == Some(100201) || t.station_cd == Some(100202) }));
    }

    #[tokio::test]
    async fn test_get_by_station_id_vec_with_line_group_filter() {
        let repository = MockTrainTypeRepository::new();
        let station_ids = vec![100201, 100202];
        let result = repository
            .get_by_station_id_vec(&station_ids, Some(1))
            .await
            .unwrap();

        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|t| t.line_group_cd == Some(1)));
    }

    #[tokio::test]
    async fn test_get_by_station_id_vec_empty() {
        let repository = MockTrainTypeRepository::new();
        let station_ids = vec![999999];
        let result = repository
            .get_by_station_id_vec(&station_ids, None)
            .await
            .unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_types_by_station_id_vec() {
        let repository = MockTrainTypeRepository::new();
        let station_ids = vec![100201, 100203];
        let result = repository
            .get_types_by_station_id_vec(&station_ids, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 3);
    }

    #[tokio::test]
    async fn test_get_types_by_station_id_vec_with_line_group_filter() {
        let repository = MockTrainTypeRepository::new();
        let station_ids = vec![100201, 100203];
        let result = repository
            .get_types_by_station_id_vec(&station_ids, Some(2))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line_group_cd, Some(2));
    }

    #[tokio::test]
    async fn test_get_by_line_group_id_vec() {
        let repository = MockTrainTypeRepository::new();
        let line_group_ids = vec![1, 2];
        let result = repository
            .get_by_line_group_id_vec(&line_group_ids)
            .await
            .unwrap();

        assert_eq!(result.len(), 4);
        assert!(result
            .iter()
            .all(|t| { t.line_group_cd == Some(1) || t.line_group_cd == Some(2) }));
    }

    #[tokio::test]
    async fn test_get_by_line_group_id_vec_empty() {
        let repository = MockTrainTypeRepository::new();
        let line_group_ids = vec![999];
        let result = repository
            .get_by_line_group_id_vec(&line_group_ids)
            .await
            .unwrap();

        assert!(result.is_empty());
    }
}
