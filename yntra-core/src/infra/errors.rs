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
