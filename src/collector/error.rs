use crate::domain::DomainError;
use crate::storage::StorageError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error("domain error: {0}")]
    Domain(#[from] DomainError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("discord configuration error: {0}")]
    Configuration(String),
    #[error("discord gateway error: {0}")]
    Gateway(String),
    #[error("discord gateway protocol error: {0}")]
    GatewayProtocol(String),
    #[error("discord snapshot has too few nicknamed members: {actual}, minimum {minimum}")]
    SnapshotTooSmall { actual: usize, minimum: usize },
}
