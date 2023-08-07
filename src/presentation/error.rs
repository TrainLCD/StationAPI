use std::sync::Arc;

use thiserror::Error;

use crate::use_case::error::UseCaseError;

#[derive(Debug, Clone, Error)]
pub enum PresentationalError {
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    OtherError(Arc<anyhow::Error>),
    #[error("{0}")]
    Unexpected(String),
}

impl From<UseCaseError> for PresentationalError {
    fn from(err: UseCaseError) -> Self {
        match err {
            UseCaseError::NotFound { .. } => PresentationalError::NotFound(err.to_string()),
            UseCaseError::Other(_) => {
                PresentationalError::OtherError(Arc::new(anyhow::Error::new(err)))
            }
            UseCaseError::Unexpected(message) => PresentationalError::Unexpected(message),
        }
    }
}

impl From<PresentationalError> for tonic::Status {
    fn from(err: PresentationalError) -> Self {
        match err {
            PresentationalError::NotFound(message) => tonic::Status::not_found(message),
            PresentationalError::OtherError(err) => tonic::Status::internal(err.to_string()),
            PresentationalError::Unexpected(message) => tonic::Status::internal(message),
        }
    }
}
