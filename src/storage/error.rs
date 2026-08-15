use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("invalid database URL: {0}")]
    InvalidDatabaseUrl(String),
    #[error("invalid stored revision: {0}")]
    InvalidRevision(String),
    #[error("invalid stored timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("invalid stored member JSON: {0}")]
    InvalidMemberJson(#[from] serde_json::Error),
}
