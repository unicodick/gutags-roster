use super::error::ApiError;
use super::state::AppState;
use crate::collector::CollectorError;
use crate::protocol::{IngestRequest, IngestResponse};
use crate::storage::{age_seconds, now_unix};
use crate::websocket::websocket;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, post};
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
        .route("/internal/v1/ingest", post(ingest))
        .route("/ws", get(websocket))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
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

async fn ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(request): axum::Json<IngestRequest>,
) -> Result<axum::Json<IngestResponse>, ApiError> {
    if !ingest_authorized(&state, headers.get("x-gytags-ingest-token")) {
        return Err(ApiError::Unauthorized);
    }

    let revision = match request {
        IngestRequest::Snapshot { members } => state
            .sync
            .apply(members)
            .await
            .map_err(api_collector_error)?,
    };

    Ok(axum::Json(IngestResponse {
        status: "ok".into(),
        revision,
    }))
}

fn api_collector_error(error: CollectorError) -> ApiError {
    match error {
        CollectorError::Domain(error) => ApiError::BadRequest(error.to_string()),
        error => ApiError::Collector(error),
    }
}

fn ingest_authorized(state: &AppState, token: Option<&axum::http::HeaderValue>) -> bool {
    token_authorized(&state.ingest_token, token)
}

fn token_authorized(expected: &Option<String>, provided: Option<&axum::http::HeaderValue>) -> bool {
    match expected {
        Some(expected) => provided.and_then(|value| value.to_str().ok()) == Some(expected),
        None => true,
    }
}
