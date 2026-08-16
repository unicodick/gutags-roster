use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
#[error("storage error: {0}")]
pub struct ApiError(#[from] crate::storage::StorageError);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}
