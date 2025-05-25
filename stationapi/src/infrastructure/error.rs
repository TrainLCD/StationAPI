use crate::domain::error::DomainError;

impl From<sqlx::Error> for DomainError {
    fn from(error: sqlx::Error) -> Self {
        DomainError::InfrastructureError(anyhow::Error::new(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Error as SqlxError;

    #[test]
    fn test_from_row_not_found() {
        let error = SqlxError::RowNotFound;
        let domain_error = DomainError::from(error);

        assert!(matches!(domain_error, DomainError::InfrastructureError(_)));
    }

    #[test]
    fn test_from_database_error() {
        // データベース接続エラーのテスト
        let error = SqlxError::Configuration("Invalid connection string".into());
        let domain_error = DomainError::from(error);

        assert!(matches!(domain_error, DomainError::InfrastructureError(_)));
    }

    #[test]
    fn test_from_io_error() {
        // IO エラーのテスト
        let io_error =
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection refused");
        let error = SqlxError::Io(io_error);
        let domain_error = DomainError::from(error);

        assert!(matches!(domain_error, DomainError::InfrastructureError(_)));
    }

    #[test]
    fn test_from_protocol_error() {
        // プロトコルエラーのテスト
        let error = SqlxError::Protocol("Invalid protocol message".into());
        let domain_error = DomainError::from(error);

        assert!(matches!(domain_error, DomainError::InfrastructureError(_)));
    }

    #[test]
    fn test_from_type_not_found() {
        // 型が見つからないエラーのテスト
        let error = SqlxError::TypeNotFound {
            type_name: "unknown_type".into(),
        };
        let domain_error = DomainError::from(error);

        assert!(matches!(domain_error, DomainError::InfrastructureError(_)));
    }

    #[test]
    fn test_from_column_index_out_of_bounds() {
        // カラムインデックスが範囲外のエラーのテスト
        let error = SqlxError::ColumnIndexOutOfBounds { index: 5, len: 3 };
        let domain_error = DomainError::from(error);

        assert!(matches!(domain_error, DomainError::InfrastructureError(_)));
    }

    #[test]
    fn test_from_column_not_found() {
        // カラムが見つからないエラーのテスト
        let error = SqlxError::ColumnNotFound("non_existent_column".into());
        let domain_error = DomainError::from(error);

        assert!(matches!(domain_error, DomainError::InfrastructureError(_)));
    }

    #[test]
    fn test_error_conversion_preserves_information() {
        // エラー変換時に情報が保持されることをテスト
        let original_message = "Test error message";
        let error = SqlxError::Configuration(original_message.into());
        let domain_error = DomainError::from(error);

        if let DomainError::InfrastructureError(anyhow_error) = domain_error {
            let error_string = anyhow_error.to_string();
            assert!(error_string.contains(original_message));
        } else {
            panic!("Expected InfrastructureError variant");
        }
    }

    #[test]
    fn test_error_chain_preservation() {
        // エラーチェーンが保持されることをテスト
        let io_error =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
        let sqlx_error = SqlxError::Io(io_error);
        let domain_error = DomainError::from(sqlx_error);

        if let DomainError::InfrastructureError(anyhow_error) = domain_error {
            // anyhow::Error が元のエラーの情報を保持していることを確認
            let error_string = anyhow_error.to_string();
            assert!(error_string.contains("Permission denied"));
        } else {
            panic!("Expected InfrastructureError variant");
        }
    }

    #[test]
    fn test_multiple_error_conversions() {
        // 複数のエラー変換が独立して動作することをテスト
        let errors = vec![
            SqlxError::RowNotFound,
            SqlxError::Configuration("Config error".into()),
            SqlxError::Protocol("Protocol error".into()),
        ];

        for error in errors {
            let domain_error = DomainError::from(error);
            assert!(matches!(domain_error, DomainError::InfrastructureError(_)));
        }
    }
}
