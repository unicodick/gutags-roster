use super::error::ApiError;
use super::state::AppState;
use crate::storage::{age_seconds, now_unix};
use crate::websocket::websocket;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    revision: i64,
    source_status: String,
    source_age_seconds: Option<i64>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/ws", get(websocket))
        .with_state(state)
}

async fn healthz(State(state): State<AppState>) -> Result<axum::Json<HealthResponse>, ApiError> {
    let status = state.repository.status().await?;
    let now = now_unix();
    Ok(axum::Json(HealthResponse {
        status: "ok",
        revision: status.revision,
        source_status: status.effective_source_status(state.source_ttl_seconds, now),
        source_age_seconds: age_seconds(status.last_source_sync_at, now),
    }))
}
