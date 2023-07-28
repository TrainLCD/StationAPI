use async_trait::async_trait;
use bigdecimal::Zero;
use sqlx::{MySql, MySqlConnection, Pool};

use crate::domain::{
    entity::company::Company, error::DomainError, repository::company_repository::CompanyRepository,
};

#[derive(sqlx::FromRow, Clone)]
pub struct CompanyRow {
    pub company_cd: u32,
    pub rr_cd: u32,
    pub company_name: String,
    pub company_name_k: String,
    pub company_name_h: String,
    pub company_name_r: String,
    pub company_name_en: String,
    pub company_name_full_en: String,
    pub company_url: String,
    pub company_type: u32,
    pub e_status: u32,
    pub e_sort: u32,
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

#[derive(Debug, Clone)]
pub struct MyCompanyRepository {
    pool: Pool<MySql>,
}

impl MyCompanyRepository {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CompanyRepository for MyCompanyRepository {
    async fn find_by_id_vec(&self, id_vec: Vec<u32>) -> Result<Vec<Company>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalCompanyRepository::find_by_id_vec(id_vec, &mut conn).await
    }
}

pub struct InternalCompanyRepository {}

impl InternalCompanyRepository {
    async fn find_by_id_vec(
        id_vec: Vec<u32>,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<Company>, DomainError> {
        if id_vec.len().is_zero() {
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
