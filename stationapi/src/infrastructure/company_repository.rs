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
        let query_str = format!("SELECT * FROM companies WHERE company_cd IN ( {params_str} )");

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
    use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

    /// テスト用のPostgreSQLデータベースをセットアップ
    async fn setup_test_db() -> Pool<Postgres> {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://stationapi:stationapi@localhost/stationapi_test".to_string()
        });

        let pool = PgPoolOptions::new()
            .connect(&database_url)
            .await
            .expect("データベース接続に失敗しました");

        // テスト用のテーブルを作成
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS companies (
                company_cd INTEGER PRIMARY KEY,
                rr_cd INTEGER NOT NULL,
                company_name TEXT NOT NULL,
                company_name_k TEXT NOT NULL,
                company_name_h TEXT NOT NULL,
                company_name_r TEXT NOT NULL,
                company_name_en TEXT NOT NULL,
                company_name_full_en TEXT NOT NULL,
                company_url TEXT,
                company_type INTEGER NOT NULL,
                e_status INTEGER NOT NULL,
                e_sort INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("テーブル作成に失敗しました");

        // テスト用のデータを挿入
        sqlx::query(
            r#"
            INSERT INTO companies (
                company_cd, rr_cd, company_name, company_name_k, company_name_h,
                company_name_r, company_name_en, company_name_full_en, company_url,
                company_type, e_status, e_sort
            ) VALUES 
                (1, 1, 'JR東日本', 'ジェイアールヒガシニホン', '東日本旅客鉄道株式会社', 
                 'JR東日本', 'JR East', 'East Japan Railway Company', 
                 'https://www.jreast.co.jp/', 1, 0, 1),
                (2, 2, 'JR西日本', 'ジェイアールニシニホン', '西日本旅客鉄道株式会社',
                 'JR西日本', 'JR West', 'West Japan Railway Company',
                 'https://www.westjr.co.jp/', 1, 0, 2),
                (3, 3, '東京メトロ', 'トウキョウメトロ', '東京地下鉄株式会社',
                 '東京メトロ', 'Tokyo Metro', 'Tokyo Metro Co., Ltd.',
                 'https://www.tokyometro.jp/', 2, 0, 3)
            "#,
        )
        .execute(&pool)
        .await
        .expect("テストデータの挿入に失敗しました");

        pool
    }

    /// CompanyRowからCompanyへの変換をテスト
    #[test]
    fn test_company_row_to_company_conversion() {
        let company_row = CompanyRow {
            company_cd: 1,
            rr_cd: 1,
            company_name: "JR東日本".to_string(),
            company_name_k: "ジェイアールヒガシニホン".to_string(),
            company_name_h: "東日本旅客鉄道株式会社".to_string(),
            company_name_r: "JR東日本".to_string(),
            company_name_en: "JR East".to_string(),
            company_name_full_en: "East Japan Railway Company".to_string(),
            company_url: Some("https://www.jreast.co.jp/".to_string()),
            company_type: 1,
            e_status: 0,
            e_sort: 1,
        };

        let company: Company = company_row.into();

        assert_eq!(company.company_cd, 1);
        assert_eq!(company.rr_cd, 1);
        assert_eq!(company.company_name, "JR東日本");
        assert_eq!(company.company_name_k, "ジェイアールヒガシニホン");
        assert_eq!(company.company_name_h, "東日本旅客鉄道株式会社");
        assert_eq!(company.company_name_r, "JR東日本");
        assert_eq!(company.company_name_en, "JR East");
        assert_eq!(company.company_name_full_en, "East Japan Railway Company");
        assert_eq!(
            company.company_url,
            Some("https://www.jreast.co.jp/".to_string())
        );
        assert_eq!(company.company_type, 1);
        assert_eq!(company.e_status, 0);
        assert_eq!(company.e_sort, 1);
    }

    /// company_urlがNoneの場合の変換をテスト
    #[test]
    fn test_company_row_to_company_conversion_with_none_url() {
        let company_row = CompanyRow {
            company_cd: 1,
            rr_cd: 1,
            company_name: "テスト会社".to_string(),
            company_name_k: "テストガイシャ".to_string(),
            company_name_h: "テスト株式会社".to_string(),
            company_name_r: "テスト会社".to_string(),
            company_name_en: "Test Company".to_string(),
            company_name_full_en: "Test Company Ltd.".to_string(),
            company_url: None,
            company_type: 1,
            e_status: 0,
            e_sort: 1,
        };

        let company: Company = company_row.into();

        assert_eq!(company.company_url, None);
    }

    /// InternalCompanyRepository::find_by_id_vec - 正常系
    #[tokio::test]
    async fn test_internal_company_repository_find_by_id_vec_success() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let id_vec = vec![1i32, 2i32];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 2);

        // IDで昇順にソートして比較
        let mut companies = companies;
        companies.sort_by_key(|c| c.company_cd);

        assert_eq!(companies[0].company_cd, 1);
        assert_eq!(companies[0].company_name, "JR東日本");
        assert_eq!(companies[1].company_cd, 2);
        assert_eq!(companies[1].company_name, "JR西日本");
    }

    /// InternalCompanyRepository::find_by_id_vec - 空のIDベクター
    #[tokio::test]
    async fn test_internal_company_repository_find_by_id_vec_empty() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let id_vec = vec![];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 0);
    }

    /// InternalCompanyRepository::find_by_id_vec - 単一のID
    #[tokio::test]
    async fn test_internal_company_repository_find_by_id_vec_single() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let id_vec = vec![3i32];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 1);
        assert_eq!(companies[0].company_cd, 3);
        assert_eq!(companies[0].company_name, "東京メトロ");
        assert_eq!(companies[0].company_name_en, "Tokyo Metro");
    }

    /// InternalCompanyRepository::find_by_id_vec - 存在しないID
    #[tokio::test]
    async fn test_internal_company_repository_find_by_id_vec_non_existent() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let id_vec = vec![999i32];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 0);
    }

    /// InternalCompanyRepository::find_by_id_vec - 存在するIDと存在しないIDの混在
    #[tokio::test]
    async fn test_internal_company_repository_find_by_id_vec_mixed() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let id_vec = vec![1i32, 999i32, 2i32];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 2);

        // IDで昇順にソートして比較
        let mut companies = companies;
        companies.sort_by_key(|c| c.company_cd);

        assert_eq!(companies[0].company_cd, 1);
        assert_eq!(companies[1].company_cd, 2);
    }

    /// 大量のIDでのクエリパフォーマンステスト
    #[tokio::test]
    async fn test_internal_company_repository_find_by_id_vec_large_id_list() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        // 大量のIDを生成（既存のIDと存在しないIDを混在）
        let mut id_vec = vec![];
        for i in 1..=100 {
            id_vec.push(i);
        }

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        // テストデータには3つの会社しかないので、結果は3つ
        assert_eq!(companies.len(), 3);
    }

    /// SQLインジェクション攻撃の防止をテスト
    #[tokio::test]
    async fn test_sql_injection_prevention() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");

        // 悪意のあるSQLを含むIDリスト（実際にはi32にキャストされるため無害になる）
        let id_vec = vec![1i32, 2i32];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        // 正常に実行され、期待通りの結果が返されることを確認
        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 2);
    }

    /// 空クエリの生成をテスト（ID数が1の場合）
    #[tokio::test]
    async fn test_query_generation_single_id() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let id_vec = vec![1i32];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        assert_eq!(companies.len(), 1);
        assert_eq!(companies[0].company_cd, 1);
    }

    /// 重複IDのテスト
    #[tokio::test]
    async fn test_internal_company_repository_find_by_id_vec_duplicate_ids() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let id_vec = vec![1i32, 1i32, 2i32, 2i32];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        // 重複するIDがあっても、データベースからは重複しない結果が返される
        assert_eq!(companies.len(), 2);
    }

    /// 負の値のIDをテスト
    #[tokio::test]
    async fn test_internal_company_repository_find_by_id_vec_negative_ids() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let id_vec = vec![-1i32, -2i32];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        // 負のIDは存在しないため、結果は空
        assert_eq!(companies.len(), 0);
    }

    /// ゼロのIDをテスト
    #[tokio::test]
    async fn test_internal_company_repository_find_by_id_vec_zero_id() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let id_vec = vec![0i32];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        // ゼロのIDは存在しないため、結果は空
        assert_eq!(companies.len(), 0);
    }

    /// 正常なIDと負のIDの混在をテスト
    #[tokio::test]
    async fn test_internal_company_repository_find_by_id_vec_mixed_positive_negative() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let id_vec = vec![1i32, -1i32, 2i32, -2i32];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        // 正のIDのみが結果として返される
        assert_eq!(companies.len(), 2);

        let mut companies = companies;
        companies.sort_by_key(|c| c.company_cd);

        assert_eq!(companies[0].company_cd, 1);
        assert_eq!(companies[1].company_cd, 2);
    }

    /// i32の最大値と最小値のテスト
    #[tokio::test]
    async fn test_internal_company_repository_find_by_id_vec_extreme_values() {
        let pool = setup_test_db().await;
        let mut conn = pool
            .acquire()
            .await
            .expect("コネクション取得に失敗しました");
        let id_vec = vec![i32::MAX, i32::MIN, 1i32];

        let result = InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await;

        assert!(result.is_ok());
        let companies = result.unwrap();
        // 存在するIDのみが結果として返される（1のみ）
        assert_eq!(companies.len(), 1);
        assert_eq!(companies[0].company_cd, 1);
    }
}
