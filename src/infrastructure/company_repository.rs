use async_trait::async_trait;
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
    async fn find_by_id(&self, id: u32) -> Result<Option<Company>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalCompanyRepository::find_by_id(id, &mut conn).await
    }
}

pub struct InternalCompanyRepository {}

impl InternalCompanyRepository {
    async fn find_by_id(
        id: u32,
        conn: &mut MySqlConnection,
    ) -> Result<Option<Company>, DomainError> {
        let rows: Option<CompanyRow> =
            sqlx::query_as("SELECT * FROM `companies` WHERE company_cd = ?")
                .bind(id)
                .fetch_optional(conn)
                .await?;
        let company: Option<Company> = rows.map(|row| row.into());

        Ok(company)
    }
}
