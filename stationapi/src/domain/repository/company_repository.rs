use async_trait::async_trait;

use crate::domain::{entity::company::Company, error::DomainError};

#[async_trait]
pub trait CompanyRepository: Send + Sync + 'static {
    async fn find_by_id_vec(&self, id_vec: &[u32]) -> Result<Vec<Company>, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // テスト用のモック実装
    pub struct MockCompanyRepository {
        companies: HashMap<u32, Company>,
    }

    impl MockCompanyRepository {
        pub fn new() -> Self {
            let mut companies = HashMap::new();

            // テスト用のダミーデータを準備
            companies.insert(
                1,
                Company::new(
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
                ),
            );

            companies.insert(
                2,
                Company::new(
                    2,
                    2,
                    "JR西日本".to_string(),
                    "ジェイアールニシニホン".to_string(),
                    "西日本旅客鉄道株式会社".to_string(),
                    "JR西日本".to_string(),
                    "JR West".to_string(),
                    "West Japan Railway Company".to_string(),
                    Some("https://www.westjr.co.jp/".to_string()),
                    1,
                    0,
                    2,
                ),
            );

            companies.insert(
                3,
                Company::new(
                    3,
                    3,
                    "東京メトロ".to_string(),
                    "トウキョウメトロ".to_string(),
                    "東京地下鉄株式会社".to_string(),
                    "東京メトロ".to_string(),
                    "Tokyo Metro".to_string(),
                    "Tokyo Metro Co., Ltd.".to_string(),
                    Some("https://www.tokyometro.jp/".to_string()),
                    2,
                    0,
                    3,
                ),
            );

            Self { companies }
        }
    }

    #[async_trait]
    impl CompanyRepository for MockCompanyRepository {
        async fn find_by_id_vec(&self, id_vec: &[u32]) -> Result<Vec<Company>, DomainError> {
            let mut result = Vec::new();

            for &id in id_vec {
                if let Some(company) = self.companies.get(&id) {
                    result.push(company.clone());
                }
            }

            // IDの順序を保持するために、入力の順序でソート
            result.sort_by_key(|company| {
                id_vec
                    .iter()
                    .position(|&id| id == company.company_cd as u32)
                    .unwrap_or(usize::MAX)
            });

            Ok(result)
        }
    }

    #[tokio::test]
    async fn test_find_by_id_vec_success() {
        let repository = MockCompanyRepository::new();
        let id_vec = vec![1, 2];

        let result = repository.find_by_id_vec(&id_vec).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 2);
        assert_eq!(companies[0].company_cd, 1);
        assert_eq!(companies[0].company_name, "JR東日本");
        assert_eq!(companies[1].company_cd, 2);
        assert_eq!(companies[1].company_name, "JR西日本");
    }

    #[tokio::test]
    async fn test_find_by_id_vec_single_company() {
        let repository = MockCompanyRepository::new();
        let id_vec = vec![3];

        let result = repository.find_by_id_vec(&id_vec).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 1);
        assert_eq!(companies[0].company_cd, 3);
        assert_eq!(companies[0].company_name, "東京メトロ");
    }

    #[tokio::test]
    async fn test_find_by_id_vec_empty_input() {
        let repository = MockCompanyRepository::new();
        let id_vec = vec![];

        let result = repository.find_by_id_vec(&id_vec).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 0);
    }

    #[tokio::test]
    async fn test_find_by_id_vec_non_existent_ids() {
        let repository = MockCompanyRepository::new();
        let id_vec = vec![999, 998];

        let result = repository.find_by_id_vec(&id_vec).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 0);
    }

    #[tokio::test]
    async fn test_find_by_id_vec_mixed_existing_and_non_existing() {
        let repository = MockCompanyRepository::new();
        let id_vec = vec![1, 999, 2];

        let result = repository.find_by_id_vec(&id_vec).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 2);
        assert_eq!(companies[0].company_cd, 1);
        assert_eq!(companies[1].company_cd, 2);
    }

    #[tokio::test]
    async fn test_find_by_id_vec_preserves_order() {
        let repository = MockCompanyRepository::new();
        let id_vec = vec![3, 1, 2];

        let result = repository.find_by_id_vec(&id_vec).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 3);
        // 入力の順序通りになっているかを確認
        assert_eq!(companies[0].company_cd, 3); // 東京メトロ
        assert_eq!(companies[1].company_cd, 1); // JR東日本
        assert_eq!(companies[2].company_cd, 2); // JR西日本
    }
}
