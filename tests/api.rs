use gutags_roster::api::{AppState, router};
use gutags_roster::collector::SnapshotSync;
use gutags_roster::domain::RawMember;
use gutags_roster::storage::Repository;
use tokio::sync::broadcast;

#[tokio::test]
async fn exposes_source_freshness() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    let (events, _) = broadcast::channel(8);
    let sync = SnapshotSync::new(repository.clone(), Vec::new(), Vec::new(), events.clone());
    sync.apply(vec![RawMember {
        discord_id: "1".into(),
        nickname: "Player".into(),
        role_ids: Vec::new(),
    }])
    .await
    .unwrap();
    let state = AppState {
        repository,
        events,
        source_ttl_seconds: 1,
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    let client = reqwest::Client::new();
    let base_url = format!("http://{address}");

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
