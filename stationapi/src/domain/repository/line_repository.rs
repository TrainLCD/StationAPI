use async_trait::async_trait;

use crate::domain::{entity::line::Line, error::DomainError};

#[async_trait]
pub trait LineRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: u32) -> Result<Option<Line>, DomainError>;
    async fn find_by_station_id(&self, station_id: u32) -> Result<Option<Line>, DomainError>;
    async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<Line>, DomainError>;
    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Line>, DomainError>;
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError>;
    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Line>, DomainError>;
    async fn get_by_line_group_id_vec(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError>;
    // FIXME: もっとマシな命名
    async fn get_by_line_group_id_vec_for_routes(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError>;
    async fn get_by_name(
        &self,
        line_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Line>, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entity::{company::Company, line_symbol::LineSymbol};
    use std::collections::HashMap;

    // テスト用のモック実装
    pub struct MockLineRepository {
        lines: HashMap<u32, Line>,
        lines_by_station_id: HashMap<u32, Line>,
        lines_by_station_group_id: HashMap<u32, Vec<Line>>,
        lines_by_line_group_id: HashMap<u32, Vec<Line>>,
        lines_by_name: HashMap<String, Vec<Line>>,
    }

    impl MockLineRepository {
        pub fn new() -> Self {
            let mut lines = HashMap::new();
            let mut lines_by_station_id = HashMap::new();
            let mut lines_by_station_group_id = HashMap::new();
            let mut lines_by_line_group_id = HashMap::new();
            let mut lines_by_name = HashMap::new();

            // テスト用のダミーデータを準備
            let company_jr_east = Company::new(
                1,
                1,
                "JR東日本".to_string(),
                "ジェイアールヒガシニホン".to_string(),
                "東日本旅客鉄道株式会社".to_string(),
                "JR東日本".to_string(),
                "JR East".to_string(),
                "East Japan Railway Company".to_string(),
                Some("https://www.jreast.co.jp/".to_string()),
                1,
                0,
                1,
            );

            let line_symbols_yamanote = vec![LineSymbol::new(
                "JY".to_string(),
                "#80C241".to_string(),
                "SQUARE".to_string(),
            )];

            let line_symbols_keihin = vec![LineSymbol::new(
                "JK".to_string(),
                "#009639".to_string(),
                "SQUARE".to_string(),
            )];

            // 山手線
            let yamanote_line = Line::new(
                11302,
                1,
                Some(company_jr_east.clone()),
                "山手線".to_string(),
                "ヤマノテセン".to_string(),
                "山手線".to_string(),
                Some("Yamanote Line".to_string()),
                Some("山手线".to_string()),
                Some("야마노테선".to_string()),
                Some("#80C241".to_string()),
                Some(2),
                line_symbols_yamanote,
                Some("JY".to_string()),
                None,
                None,
                None,
                Some("#80C241".to_string()),
                None,
                None,
                None,
                Some("SQUARE".to_string()),
                None,
                None,
                None,
                0,
                11302,
                None,
                None,
                Some(1),
                Some(1001),
                Some(1),
                Some(1075.968412),
                Some(0),
            );

            // 京浜東北線
            let keihin_line = Line::new(
                11303,
                1,
                Some(company_jr_east.clone()),
                "京浜東北線".to_string(),
                "ケイヒントウホクセン".to_string(),
                "京浜東北線".to_string(),
                Some("Keihin-Tohoku Line".to_string()),
                Some("京滨东北线".to_string()),
                Some("게이힌토호쿠선".to_string()),
                Some("#009639".to_string()),
                Some(1),
                line_symbols_keihin,
                Some("JK".to_string()),
                None,
                None,
                None,
                Some("#009639".to_string()),
                None,
                None,
                None,
                Some("SQUARE".to_string()),
                None,
                None,
                None,
                0,
                11303,
                None,
                None,
                Some(2),
                Some(1002),
                Some(2),
                Some(1234.567890),
                Some(1),
            );

            // データを格納
            lines.insert(1, yamanote_line.clone());
            lines.insert(2, keihin_line.clone());

            lines_by_station_id.insert(1001, yamanote_line.clone());
            lines_by_station_id.insert(1002, keihin_line.clone());

            lines_by_station_group_id.insert(1, vec![yamanote_line.clone()]);
            lines_by_station_group_id.insert(2, vec![keihin_line.clone()]);

            lines_by_line_group_id.insert(1, vec![yamanote_line.clone()]);
            lines_by_line_group_id.insert(2, vec![keihin_line.clone()]);

            lines_by_name.insert("山手線".to_string(), vec![yamanote_line.clone()]);
            lines_by_name.insert("京浜東北線".to_string(), vec![keihin_line.clone()]);

            Self {
                lines,
                lines_by_station_id,
                lines_by_station_group_id,
                lines_by_line_group_id,
                lines_by_name,
            }
        }
    }

    #[async_trait]
    impl LineRepository for MockLineRepository {
        async fn find_by_id(&self, id: u32) -> Result<Option<Line>, DomainError> {
            Ok(self.lines.get(&id).cloned())
        }

        async fn find_by_station_id(&self, station_id: u32) -> Result<Option<Line>, DomainError> {
            Ok(self.lines_by_station_id.get(&station_id).cloned())
        }

        async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<Line>, DomainError> {
            let mut result = Vec::new();
            for &id in ids {
                if let Some(line) = self.lines.get(&id) {
                    result.push(line.clone());
                }
            }
            Ok(result)
        }

        async fn get_by_station_group_id(
            &self,
            station_group_id: u32,
        ) -> Result<Vec<Line>, DomainError> {
            Ok(self
                .lines_by_station_group_id
                .get(&station_group_id)
                .cloned()
                .unwrap_or_default())
        }

        async fn get_by_station_group_id_vec(
            &self,
            station_group_id_vec: &[u32],
        ) -> Result<Vec<Line>, DomainError> {
            let mut result = Vec::new();
            for &station_group_id in station_group_id_vec {
                if let Some(lines) = self.lines_by_station_group_id.get(&station_group_id) {
                    result.extend(lines.clone());
                }
            }
            Ok(result)
        }

        async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Line>, DomainError> {
            Ok(self
                .lines_by_line_group_id
                .get(&line_group_id)
                .cloned()
                .unwrap_or_default())
        }

        async fn get_by_line_group_id_vec(
            &self,
            line_group_id_vec: &[u32],
        ) -> Result<Vec<Line>, DomainError> {
            let mut result = Vec::new();
            for &line_group_id in line_group_id_vec {
                if let Some(lines) = self.lines_by_line_group_id.get(&line_group_id) {
                    result.extend(lines.clone());
                }
            }
            Ok(result)
        }

        async fn get_by_line_group_id_vec_for_routes(
            &self,
            line_group_id_vec: &[u32],
        ) -> Result<Vec<Line>, DomainError> {
            // このテストでは通常のget_by_line_group_id_vecと同じ動作とする
            self.get_by_line_group_id_vec(line_group_id_vec).await
        }

        async fn get_by_name(
            &self,
            line_name: String,
            limit: Option<u32>,
        ) -> Result<Vec<Line>, DomainError> {
            let mut result = self
                .lines_by_name
                .iter()
                .filter(|(name, _)| name.contains(&line_name))
                .flat_map(|(_, lines)| lines.clone())
                .collect::<Vec<_>>();

            if let Some(limit) = limit {
                result.truncate(limit as usize);
            }

            Ok(result)
        }
    }

    #[tokio::test]
    async fn test_find_by_id_success() {
        let repository = MockLineRepository::new();

        let result = repository.find_by_id(1).await;

        assert!(result.is_ok());
        let line = result.unwrap();
        assert!(line.is_some());
        let line = line.unwrap();
        assert_eq!(line.line_cd, 11302);
        assert_eq!(line.line_name, "山手線");
        assert_eq!(line.line_name_r, Some("Yamanote Line".to_string()));
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let repository = MockLineRepository::new();

        let result = repository.find_by_id(999).await;

        assert!(result.is_ok());
        let line = result.unwrap();
        assert!(line.is_none());
    }

    #[tokio::test]
    async fn test_find_by_station_id_success() {
        let repository = MockLineRepository::new();

        let result = repository.find_by_station_id(1001).await;

        assert!(result.is_ok());
        let line = result.unwrap();
        assert!(line.is_some());
        let line = line.unwrap();
        assert_eq!(line.line_name, "山手線");
    }

    #[tokio::test]
    async fn test_find_by_station_id_not_found() {
        let repository = MockLineRepository::new();

        let result = repository.find_by_station_id(9999).await;

        assert!(result.is_ok());
        let line = result.unwrap();
        assert!(line.is_none());
    }

    #[tokio::test]
    async fn test_get_by_ids_success() {
        let repository = MockLineRepository::new();
        let ids = vec![1, 2];

        let result = repository.get_by_ids(&ids).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].line_name, "山手線");
        assert_eq!(lines[1].line_name, "京浜東北線");
    }

    #[tokio::test]
    async fn test_get_by_ids_empty_input() {
        let repository = MockLineRepository::new();
        let ids = vec![];

        let result = repository.get_by_ids(&ids).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    #[tokio::test]
    async fn test_get_by_ids_non_existent_ids() {
        let repository = MockLineRepository::new();
        let ids = vec![999, 998];

        let result = repository.get_by_ids(&ids).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    #[tokio::test]
    async fn test_get_by_station_group_id_success() {
        let repository = MockLineRepository::new();

        let result = repository.get_by_station_group_id(1).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].line_name, "山手線");
    }

    #[tokio::test]
    async fn test_get_by_station_group_id_not_found() {
        let repository = MockLineRepository::new();

        let result = repository.get_by_station_group_id(999).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    #[tokio::test]
    async fn test_get_by_station_group_id_vec_success() {
        let repository = MockLineRepository::new();
        let station_group_ids = vec![1, 2];

        let result = repository
            .get_by_station_group_id_vec(&station_group_ids)
            .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_station_group_id_vec_empty_input() {
        let repository = MockLineRepository::new();
        let station_group_ids = vec![];

        let result = repository
            .get_by_station_group_id_vec(&station_group_ids)
            .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    #[tokio::test]
    async fn test_get_by_line_group_id_success() {
        let repository = MockLineRepository::new();

        let result = repository.get_by_line_group_id(1).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].line_name, "山手線");
    }

    #[tokio::test]
    async fn test_get_by_line_group_id_not_found() {
        let repository = MockLineRepository::new();

        let result = repository.get_by_line_group_id(999).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    #[tokio::test]
    async fn test_get_by_line_group_id_vec_success() {
        let repository = MockLineRepository::new();
        let line_group_ids = vec![1, 2];

        let result = repository.get_by_line_group_id_vec(&line_group_ids).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_line_group_id_vec_empty_input() {
        let repository = MockLineRepository::new();
        let line_group_ids = vec![];

        let result = repository.get_by_line_group_id_vec(&line_group_ids).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }

    #[tokio::test]
    async fn test_get_by_line_group_id_vec_for_routes_success() {
        let repository = MockLineRepository::new();
        let line_group_ids = vec![1, 2];

        let result = repository
            .get_by_line_group_id_vec_for_routes(&line_group_ids)
            .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_name_exact_match() {
        let repository = MockLineRepository::new();

        let result = repository.get_by_name("山手線".to_string(), None).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].line_name, "山手線");
    }

    #[tokio::test]
    async fn test_get_by_name_partial_match() {
        let repository = MockLineRepository::new();

        let result = repository.get_by_name("京浜".to_string(), None).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].line_name, "京浜東北線");
    }

    #[tokio::test]
    async fn test_get_by_name_with_limit() {
        let repository = MockLineRepository::new();

        let result = repository.get_by_name("線".to_string(), Some(1)).await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 1);
    }

    #[tokio::test]
    async fn test_get_by_name_not_found() {
        let repository = MockLineRepository::new();

        let result = repository
            .get_by_name("存在しない路線".to_string(), None)
            .await;

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 0);
    }
}
