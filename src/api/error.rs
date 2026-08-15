use crate::collector::CollectorError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("storage error: {0}")]
    Storage(#[from] crate::storage::StorageError),
    #[error("collector error: {0}")]
    Collector(#[from] CollectorError),
    #[error("unauthorized")]
    Unauthorized,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::Storage(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
            Self::Collector(CollectorError::Domain(error)) => {
                (StatusCode::BAD_REQUEST, error.to_string())
            }
            Self::Collector(error) => (StatusCode::BAD_GATEWAY, error.to_string()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".into()),
        };
        (status, axum::Json(serde_json::json!({ "error": message }))).into_response()
    }
}
