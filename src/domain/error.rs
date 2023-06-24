use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error(r#"{entity_type} was not found for entity_id "{entity_id}"."#)]
    NotFound {
        entity_type: &'static str,
        entity_id: String,
    },
    #[error(transparent)]
    InfrastructureError(anyhow::Error),
    #[error("{0}")]
    Unexpected(String),
}
