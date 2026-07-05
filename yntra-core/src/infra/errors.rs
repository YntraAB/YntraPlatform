#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum YntraError {
    #[error("Database error: {0}")]
    DbError(String),
    #[error("Network sync error: {0}")]
    SyncError(String),
    #[error("Authentication error: {0}")]
    AuthError(String),
    #[error("Constraint violation: {0}")]
    ConstraintError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Not found: {0}")]
    NotFoundError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Invitation error: {0}")]
    InvitationError(String),
}

#[cfg(not(target_arch = "wasm32"))]
impl From<libsql::Error> for YntraError {
    fn from(err: libsql::Error) -> Self {
        match err {
            libsql::Error::SqliteFailure(code, msg) => {
                if code == 19 {
                    YntraError::ConstraintError(msg)
                } else {
                    YntraError::DbError(format!("SQLite error ({}): {}", code, msg))
                }
            }
            libsql::Error::ConnectionFailed(msg) => YntraError::DbError(format!("Connection failed: {}", msg)),
            libsql::Error::Misuse(msg) => YntraError::DbError(format!("API misuse: {}", msg)),
            _ => YntraError::DbError(err.to_string()),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn test_libsql_error_conversion() {
        let constraint_err = libsql::Error::SqliteFailure(19, "UNIQUE constraint failed: table.col".to_string());
        let yntra_constraint = YntraError::from(constraint_err);
        assert!(matches!(yntra_constraint, YntraError::ConstraintError(_)));
        assert_eq!(yntra_constraint.to_string(), "Constraint violation: UNIQUE constraint failed: table.col");

        let db_err = libsql::Error::SqliteFailure(1, "some other sqlite error".to_string());
        let yntra_db = YntraError::from(db_err);
        assert!(matches!(yntra_db, YntraError::DbError(_)));

        let conn_err = libsql::Error::ConnectionFailed("host unreachable".to_string());
        let yntra_conn = YntraError::from(conn_err);
        assert!(matches!(yntra_conn, YntraError::DbError(_)));
        assert!(yntra_conn.to_string().contains("Connection failed"));

        let misuse_err = libsql::Error::Misuse("misuse".to_string());
        let yntra_misuse = YntraError::from(misuse_err);
        assert!(matches!(yntra_misuse, YntraError::DbError(_)));
        assert!(yntra_misuse.to_string().contains("API misuse"));
    }
}

