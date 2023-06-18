use anyhow::Result;
use mockall::automock;

use super::company_model::Company;

#[async_trait::async_trait]
#[automock]
pub trait CompanyRepository {
    async fn find_by_id(&self, id: u32) -> Result<Company>;
    async fn get_by_line_ids(&self, line_ids: Vec<u32>) -> Result<Vec<Company>>;
}
