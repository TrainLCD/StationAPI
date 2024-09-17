use async_trait::async_trait;

use crate::domain::{entity::company::Company, error::DomainError};

#[async_trait]
pub trait CompanyRepository: Send + Sync + 'static {
    async fn find_by_id_vec(&self, id_vec: &[u32]) -> Result<Vec<Company>, DomainError>;
}
