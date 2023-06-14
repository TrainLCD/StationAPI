use anyhow::Result;
use sqlx::{MySql, Pool};

use crate::domain::models::company::{
    company_model::Company, company_repository::CompanyRepository,
};
#[derive(sqlx::FromRow, Clone)]
pub struct CompanyEntity {
    pub company_cd: u32,
    pub rr_cd: u32,
    pub company_name: String,
    pub company_name_k: String,
    pub company_name_h: String,
    pub company_name_r: String,
    pub company_name_en: String,
    pub company_name_full_en: String,
    pub company_url: String,
    pub company_type: i32,
    pub e_status: u32,
    pub e_sort: u32,
}

impl From<CompanyEntity> for Company {
    fn from(entity: CompanyEntity) -> Company {
        Company {
            company_cd: entity.company_cd,
            rr_cd: entity.rr_cd,
            company_name: entity.company_name,
            company_name_k: entity.company_name_k,
            company_name_h: entity.company_name_h,
            company_name_r: entity.company_name_r,
            company_name_en: entity.company_name_en,
            company_name_full_en: entity.company_name_full_en,
            company_url: entity.company_url,
            company_type: entity.company_type,
            e_status: entity.e_status,
            e_sort: entity.e_sort,
        }
    }
}

pub struct CompanyRepositoryImpl {
    pub pool: Box<Pool<MySql>>,
}

#[async_trait::async_trait]
impl CompanyRepository for CompanyRepositoryImpl {
    async fn find_by_id(&self, id: u32) -> Result<Company> {
        let result = sqlx::query_as!(
            CompanyEntity,
            "SELECT * FROM companies WHERE company_cd = ?",
            id
        )
        .fetch_one(self.pool.as_ref())
        .await;
        match result.map(|entity| entity.into()) {
            Ok(company) => Ok(company),
            Err(err) => Err(err.into()),
        }
    }

    async fn get_by_line_ids(&self, line_ids: Vec<u32>) -> Result<Vec<Company>> {
        let params = format!("?{}", ", ?".repeat(line_ids.len() - 1));
        let query_str = format!(
            "SELECT c.*, l.line_cd
        FROM `lines` as l, `companies` as c
        WHERE l.line_cd IN ({})
        AND l.e_status = 0
        AND c.company_cd = l.company_cd",
            params
        );
        let mut query = sqlx::query_as::<_, CompanyEntity>(&query_str);

        for id in line_ids {
            query = query.bind(id);
        }

        let result = query.fetch_all(self.pool.as_ref()).await;
        match result {
            Ok(companies) => Ok(companies
                .into_iter()
                .map(|company| company.into())
                .collect()),
            Err(err) => Err(err.into()),
        }
    }
}
