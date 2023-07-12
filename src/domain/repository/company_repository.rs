use async_trait::async_trait;
use mockall::automock;

use crate::domain::{entity::company::Company, error::DomainError};

#[automock]
#[async_trait]
pub trait CompanyRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: u32) -> Result<Option<Company>, DomainError>;
}
