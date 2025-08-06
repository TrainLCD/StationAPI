use async_trait::async_trait;
use sqlx::{PgConnection, Pool, Postgres};
use std::sync::Arc;

use crate::domain::{
    entity::company::Company, error::DomainError, repository::company_repository::CompanyRepository,
};

#[derive(sqlx::FromRow, Clone)]
pub struct CompanyRow {
    pub company_cd: i32,
    pub rr_cd: i32,
    pub company_name: String,
    pub company_name_k: String,
    pub company_name_h: String,
    pub company_name_r: String,
    pub company_name_en: String,
    pub company_name_full_en: String,
    pub company_url: Option<String>,
    pub company_type: i32,
    pub e_status: i32,
    pub e_sort: i32,
}

impl From<CompanyRow> for Company {
    fn from(row: CompanyRow) -> Self {
        Self {
            company_cd: row.company_cd,
            rr_cd: row.rr_cd,
            company_name: row.company_name,
            company_name_k: row.company_name_k,
            company_name_h: row.company_name_h,
            company_name_r: row.company_name_r,
            company_name_en: row.company_name_en,
            company_name_full_en: row.company_name_full_en,
            company_url: row.company_url,
            company_type: row.company_type,
            e_status: row.e_status,
            e_sort: row.e_sort,
        }
    }
}

pub struct MyCompanyRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyCompanyRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CompanyRepository for MyCompanyRepository {
    async fn find_by_id_vec(&self, id_vec: &[u32]) -> Result<Vec<Company>, DomainError> {
        let id_vec: Vec<i32> = id_vec.iter().map(|x| *x as i32).collect();
        let mut conn = self.pool.acquire().await?;
        InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await
    }
}

pub struct InternalCompanyRepository {}

impl InternalCompanyRepository {
    async fn find_by_id_vec(
        id_vec: &[i32],
        conn: &mut PgConnection,
    ) -> Result<Vec<Company>, DomainError> {
        if id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params: Vec<String> = (1..=id_vec.len()).map(|i| format!("${i}")).collect();
        let params_str = params.join(", ");
        let query_str = format!("SELECT company_cd, rr_cd, company_name, company_name_k, company_name_h, company_name_r, company_name_en, company_name_full_en, company_url, company_type, e_status, e_sort FROM companies WHERE company_cd IN ( {params_str} )");

        let mut query = sqlx::query_as::<_, CompanyRow>(&query_str);
        for id in id_vec {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let companies: Vec<Company> = rows.into_iter().map(|row| row.into()).collect();

        Ok(companies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use std::env;

    // テスト用のヘルパー関数
    async fn setup_test_db() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://test:test@localhost/stationapi_test".to_string());

        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    async fn setup_test_data(pool: &PgPool) {
        // テスト用のテーブルとデータを作成
        sqlx::query("DROP TABLE IF EXISTS companies CASCADE")
            .execute(pool)
            .await
            .unwrap();

        // テーブル作成
        sqlx::query(
            "CREATE TABLE companies (
                company_cd INTEGER PRIMARY KEY,
                rr_cd INTEGER NOT NULL,
                company_name VARCHAR(255) NOT NULL,
                company_name_k VARCHAR(255) NOT NULL,
                company_name_h VARCHAR(255) NOT NULL,
                company_name_r VARCHAR(255) NOT NULL,
                company_name_en VARCHAR(255) NOT NULL,
                company_name_full_en VARCHAR(255) NOT NULL,
                company_url VARCHAR(512),
                company_type INTEGER NOT NULL,
                e_status INTEGER NOT NULL DEFAULT 0,
                e_sort INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(pool)
        .await
        .unwrap();

        // テストデータの挿入
        sqlx::query(
            "INSERT INTO companies (company_cd, rr_cd, company_name, company_name_k, company_name_h, company_name_r, company_name_en, company_name_full_en, company_url, company_type, e_status, e_sort) VALUES 
            (1, 1001, 'Test Company 1', 'テスト会社1', 'テスト会社1', 'Test Company 1', 'Test Company 1', 'Test Company 1 Inc.', 'https://test1.com', 1, 0, 1),
            (2, 1002, 'Test Company 2', 'テスト会社2', 'テスト会社2', 'Test Company 2', 'Test Company 2', 'Test Company 2 Ltd.', 'https://test2.com', 2, 0, 2),
            (3, 1003, 'Test Company 3', 'テスト会社3', 'テスト会社3', 'Test Company 3', 'Test Company 3', 'Test Company 3 Corp.', NULL, 1, 0, 3)"
        )
        .execute(pool)
        .await
        .unwrap();
    }

    async fn cleanup_test_data(pool: &PgPool) {
        sqlx::query("DROP TABLE IF EXISTS companies CASCADE")
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_company_row_to_company_conversion() {
        let company_row = CompanyRow {
            company_cd: 1,
            rr_cd: 1001,
            company_name: "Test Company".to_string(),
            company_name_k: "テスト会社".to_string(),
            company_name_h: "テスト会社".to_string(),
            company_name_r: "Test Company".to_string(),
            company_name_en: "Test Company".to_string(),
            company_name_full_en: "Test Company Inc.".to_string(),
            company_url: Some("https://test.com".to_string()),
            company_type: 1,
            e_status: 0,
            e_sort: 1,
        };

        let company: Company = company_row.into();

        assert_eq!(company.company_cd, 1);
        assert_eq!(company.rr_cd, 1001);
        assert_eq!(company.company_name, "Test Company");
        assert_eq!(company.company_name_k, "テスト会社");
        assert_eq!(company.company_name_h, "テスト会社");
        assert_eq!(company.company_name_r, "Test Company");
        assert_eq!(company.company_name_en, "Test Company");
        assert_eq!(company.company_name_full_en, "Test Company Inc.");
        assert_eq!(company.company_url, Some("https://test.com".to_string()));
        assert_eq!(company.company_type, 1);
        assert_eq!(company.e_status, 0);
        assert_eq!(company.e_sort, 1);
    }

    #[tokio::test]
    async fn test_company_row_to_company_conversion_with_null_url() {
        let company_row = CompanyRow {
            company_cd: 1,
            rr_cd: 1001,
            company_name: "Test Company".to_string(),
            company_name_k: "テスト会社".to_string(),
            company_name_h: "テスト会社".to_string(),
            company_name_r: "Test Company".to_string(),
            company_name_en: "Test Company".to_string(),
            company_name_full_en: "Test Company Inc.".to_string(),
            company_url: None,
            company_type: 1,
            e_status: 0,
            e_sort: 1,
        };

        let company: Company = company_row.into();

        assert_eq!(company.company_url, None);
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_find_by_id_vec_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let ids = vec![1, 2];
        let result = InternalCompanyRepository::find_by_id_vec(&ids, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 2);

        // ソートされていることを確認（IDの順序）
        let mut found_ids: Vec<i32> = companies.iter().map(|c| c.company_cd).collect();
        found_ids.sort();
        assert_eq!(found_ids, vec![1, 2]);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_find_by_id_vec_empty() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let ids = vec![];
        let result = InternalCompanyRepository::find_by_id_vec(&ids, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_find_by_id_vec_single_company() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let ids = vec![1];
        let result = InternalCompanyRepository::find_by_id_vec(&ids, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 1);
        assert_eq!(companies[0].company_cd, 1);
        assert_eq!(companies[0].company_name, "Test Company 1");

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_find_by_id_vec_nonexistent_ids() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let ids = vec![999, 1000];
        let result = InternalCompanyRepository::find_by_id_vec(&ids, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_find_by_id_vec_mixed_existing_and_nonexistent() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let ids = vec![1, 999, 2];
        let result = InternalCompanyRepository::find_by_id_vec(&ids, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 2); // 存在する会社のみ

        let mut found_ids: Vec<i32> = companies.iter().map(|c| c.company_cd).collect();
        found_ids.sort();
        assert_eq!(found_ids, vec![1, 2]);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    async fn test_my_company_repository_new() {
        let database_url = "postgres://test:test@localhost/stationapi_test";
        let pool = PgPool::connect(database_url).await;

        if let Ok(pool) = pool {
            let pool = Arc::new(pool);
            let repository = MyCompanyRepository::new(pool.clone());

            // プールが正しく設定されていることを確認
            assert!(Arc::ptr_eq(&repository.pool, &pool));
        }
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_company_repository_find_by_id_vec() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyCompanyRepository::new(pool);

        let ids = vec![1, 2];
        let result = repository.find_by_id_vec(&ids).await;
        assert!(result.is_ok());

        let companies = result.unwrap();
        assert_eq!(companies.len(), 2);

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_company_repository_find_by_id_vec_empty() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyCompanyRepository::new(pool);

        let ids = vec![];
        let result = repository.find_by_id_vec(&ids).await;
        assert!(result.is_ok());

        let companies = result.unwrap();
        assert_eq!(companies.len(), 0);

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_company_repository_type_conversion() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyCompanyRepository::new(pool);

        // u32 から i32 への変換をテスト
        let ids: Vec<u32> = vec![1, 2];
        let result = repository.find_by_id_vec(&ids).await;
        assert!(result.is_ok());

        let companies = result.unwrap();
        assert_eq!(companies.len(), 2);

        cleanup_test_data(&repository.pool).await;
    }
}
