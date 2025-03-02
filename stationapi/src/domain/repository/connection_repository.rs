use async_trait::async_trait;

use crate::domain::{entity::connection::Connection, error::DomainError};

#[async_trait]
pub trait ConnectionRepository: Send + Sync + 'static {
    async fn get_all(&self) -> Result<Vec<Connection>, DomainError>;
}
