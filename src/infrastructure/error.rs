use crate::domain::error::DomainError;

impl From<sqlx::Error> for DomainError {
    fn from(error: sqlx::Error) -> Self {
        DomainError::InfrastructureError(anyhow::Error::new(error))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn from() {
        use super::DomainError;

        let error = sqlx::Error::RowNotFound;
        let domain_error = DomainError::from(error);

        assert!(matches!(domain_error, DomainError::InfrastructureError(_)));
    }
}
