use async_trait::async_trait;
use sqlx::{Pool, Sqlite, SqliteConnection};
use std::sync::Arc;

use crate::domain::{
    entity::company::Company, error::DomainError, repository::company_repository::CompanyRepository,
};

#[derive(sqlx::FromRow, Clone)]
pub struct CompanyRow {
    pub company_cd: i64,
    pub rr_cd: i64,
    pub company_name: String,
    pub company_name_k: String,
    pub company_name_h: String,
    pub company_name_r: String,
    pub company_name_en: String,
    pub company_name_full_en: String,
    pub company_url: Option<String>,
    pub company_type: i64,
    pub e_status: i64,
    pub e_sort: i64,
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
    pool: Arc<Pool<Sqlite>>,
}

impl MyCompanyRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CompanyRepository for MyCompanyRepository {
    async fn find_by_id_vec(&self, id_vec: &[u32]) -> Result<Vec<Company>, DomainError> {
        let id_vec: Vec<i64> = id_vec.iter().map(|x| *x as i64).collect();
        let mut conn = self.pool.acquire().await?;
        InternalCompanyRepository::find_by_id_vec(&id_vec, &mut conn).await
    }
}

pub struct InternalCompanyRepository {}

impl InternalCompanyRepository {
    async fn find_by_id_vec(
        id_vec: &[i64],
        conn: &mut SqliteConnection,
    ) -> Result<Vec<Company>, DomainError> {
        if id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = format!("?{}", ", ?".repeat(id_vec.len() - 1));
        let query_str = format!(
            "SELECT * FROM `companies` WHERE company_cd IN ( {} )",
            params
        );

        let mut query = sqlx::query_as::<_, CompanyRow>(&query_str);
        for id in id_vec {
            query = query.bind(id);
        }

        let rows = query.fetch_all(conn).await?;
        let companies: Vec<Company> = rows.into_iter().map(|row| row.into()).collect();

        Ok(companies)
    }
}
