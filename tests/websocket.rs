use futures_util::{SinkExt, StreamExt};
use gytags_roster::api::{AppState, router};
use gytags_roster::collector::SnapshotSync;
use gytags_roster::domain::RawMember;
use gytags_roster::storage::Repository;
use serde_json::json;
use tokio::sync::broadcast;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn buffers_update_during_websocket_handshake() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    let (events, _) = broadcast::channel(8);
    let sync = SnapshotSync::new(repository.clone(), Vec::new(), events.clone());
    let state = AppState {
        repository,
        events,
        websocket_token: None,
        ingest_token: None,
        sync: sync.clone(),
        source_ttl_seconds: 300,
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });

    let (mut socket, _) = connect_async(format!("ws://{address}/ws")).await.unwrap();
    sync.apply(vec![RawMember {
        discord_id: "1".into(),
        nickname: "Player".into(),
        role_ids: Vec::new(),
    }])
    .await
    .unwrap();

    socket
        .send(Message::Text(
            json!({
                "type": "hello",
                "protocol_version": 1,
                "nicknames": ["Player"]
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let hello_ack = socket.next().await.unwrap().unwrap();
    assert!(hello_ack.into_text().unwrap().contains("hello_ack"));
    let snapshot = socket.next().await.unwrap().unwrap();
    assert!(snapshot.into_text().unwrap().contains("snapshot"));

    let update = timeout(Duration::from_secs(1), socket.next())
        .await
        .expect("update emitted during handshake was lost")
        .unwrap()
        .unwrap();
    assert!(update.into_text().unwrap().contains("update"));

    server.abort();
    let _ = server.await;
}
