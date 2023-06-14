use anyhow::Result;

use crate::{
    domain::models::company::{company_model::Company, company_repository::CompanyRepository},
    pb::CompanyResponse,
};

#[derive(Debug)]
pub struct CompanyService<T>
where
    T: CompanyRepository,
{
    company_repository: T,
}

impl From<Company> for CompanyResponse {
    fn from(value: Company) -> Self {
        Self {
            id: value.company_cd,
            railroad_id: value.rr_cd,
            name_general: value.company_name,
            name_katakana: value.company_name_k,
            name_full: value.company_name_h,
            name_short: value.company_name_r,
            name_english_short: value.company_name_en,
            name_english_full: value.company_name_full_en,
            url: value.company_url,
            r#type: value.company_type,
            status: value.e_status as i32,
        }
    }
}

impl<T: CompanyRepository> CompanyService<T> {
    pub fn new(company_repository: T) -> Self {
        Self { company_repository }
    }
    pub async fn find_by_id(&self, id: u32) -> Result<Company> {
        match self.company_repository.find_by_id(id).await {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the company. Provided ID: {:?}",
                id
            )),
        }
    }
    pub async fn get_by_line_ids(&self, line_ids: &Vec<u32>) -> Result<Vec<Company>> {
        match self
            .company_repository
            .get_by_line_ids(line_ids.clone())
            .await
        {
            Ok(values) => Ok(values),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the companies. Provided Line IDs: {:?}",
                line_ids
            )),
        }
    }
}
