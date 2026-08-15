use axum::Router;
use axum::extract::{Json, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use gytags_roster::collector::{CollectorClient, CollectorClientConfig};
use gytags_roster::domain::RawMember;
use gytags_roster::protocol::{IngestRequest, IngestResponse};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone)]
struct TestState {
    ingest_attempts: Arc<AtomicUsize>,
}

#[tokio::test]
async fn sends_authenticated_ingest_and_retries() {
    let state = TestState {
        ingest_attempts: Arc::new(AtomicUsize::new(0)),
    };
    let attempts = state.ingest_attempts.clone();
    let app = Router::new()
        .route("/internal/v1/ingest", post(ingest))
        .with_state(TestState {
            ingest_attempts: attempts,
        });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::task::yield_now().await;

    let client = CollectorClient::new(CollectorClientConfig {
        base_url: format!("http://{address}"),
        ingest_token: Some("secret".into()),
    })
    .unwrap();

    let response = client
        .snapshot(vec![RawMember {
            discord_id: "1".into(),
            nickname: "Player".into(),
            role_ids: Vec::new(),
        }])
        .await
        .unwrap();
    assert_eq!(response.revision, 7);
    assert_eq!(state.ingest_attempts.load(Ordering::SeqCst), 2);

    server.abort();
    let _ = server.await;
}

async fn ingest(
    State(state): State<TestState>,
    headers: HeaderMap,
    Json(_request): Json<IngestRequest>,
) -> impl IntoResponse {
    assert_eq!(headers.get("x-gytags-ingest-token").unwrap(), "secret");
    let attempt = state.ingest_attempts.fetch_add(1, Ordering::SeqCst);
    if attempt == 0 {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    Json(IngestResponse {
        status: "ok".into(),
        revision: 7,
    })
    .into_response()
}
