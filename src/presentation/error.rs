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

#[cfg(test)]
mod tests {
    use fake::{Fake, Faker};

    #[test]
    fn from_use_case_error() {
        use super::*;

        let err = UseCaseError::NotFound {
            entity_id: Faker.fake(),
            entity_type: "entity_type",
        };
        let err = PresentationalError::from(err);
        assert!(matches!(err, PresentationalError::NotFound(_)));

        let err = UseCaseError::Other(anyhow::Error::msg(Faker.fake::<String>()));
        let err = PresentationalError::from(err);
        assert!(matches!(err, PresentationalError::OtherError(_)));

        let err = UseCaseError::Unexpected(Faker.fake());
        let err = PresentationalError::from(err);
        assert!(matches!(err, PresentationalError::Unexpected(_)));
    }
}
