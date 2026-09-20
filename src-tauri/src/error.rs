use serde::Serialize;
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("credential store error: {0}")]
    Credential(String),
    #[error("invalid post: {0}")]
    Validation(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error("account not found: {0}")]
    AccountNotFound(String),
    #[error("application state is unavailable")]
    StateUnavailable,
}
impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
