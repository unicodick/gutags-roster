use gytags_roster::api::{AppState, router};
use gytags_roster::collector::SnapshotSync;
use gytags_roster::domain::RawMember;
use gytags_roster::protocol::{IngestRequest, IngestResponse};
use gytags_roster::storage::Repository;
use reqwest::StatusCode;
use tokio::sync::broadcast;

#[tokio::test]
async fn protects_ingest_and_exposes_freshness() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    let (events, _) = broadcast::channel(8);
    let sync = SnapshotSync::new(repository.clone(), Vec::new(), Vec::new(), events.clone());
    let state = AppState {
        repository,
        events,
        websocket_token: Some("ws-secret".into()),
        ingest_token: Some("ingest-secret".into()),
        sync,
        source_ttl_seconds: 1,
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    let client = reqwest::Client::new();
    let base_url = format!("http://{address}");

    let unauthorized = client
        .post(format!("{base_url}/internal/v1/ingest"))
        .json(&IngestRequest::Snapshot {
            members: Vec::new(),
        })
        .send()
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let response = client
        .post(format!("{base_url}/internal/v1/ingest"))
        .header("x-gytags-ingest-token", "ingest-secret")
        .json(&IngestRequest::Snapshot {
            members: vec![RawMember {
                discord_id: "1".into(),
                nickname: "Player".into(),
                role_ids: Vec::new(),
            }],
        })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<IngestResponse>().await.unwrap().revision, 1);

    let health = client
        .get(format!("{base_url}/healthz"))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(health["source_status"], "fresh");
    assert_eq!(health["source_age_seconds"], 0);

    server.abort();
    let _ = server.await;
}
