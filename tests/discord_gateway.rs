use futures_util::{SinkExt, StreamExt};
use gutags_roster::collector::DiscordGateway;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[tokio::test]
async fn collects_sidebar_members_from_gateway() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(
                json!({"op": 10, "d": {"heartbeat_interval": 60_000}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();

        let identify = next_json(&mut socket).await;
        assert_eq!(identify["op"], 2);
        assert_eq!(identify["d"]["token"], "user-token");
        assert_eq!(identify["d"]["capabilities"], 22525);

        socket
            .send(Message::Text(
                json!({
                    "op": 0,
                    "t": "READY",
                    "s": 1,
                    "d": {"guilds": [{"id": "guild", "member_count": 2, "channels": [{"id": "channel", "type": 0}]}]}
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

        let subscription = next_json(&mut socket).await;
        assert_eq!(subscription["op"], 37);
        assert_eq!(
            subscription["d"]["subscriptions"]["guild"]["channels"]["channel"],
            json!([[0, 1]])
        );

        socket
            .send(Message::Text(
                json!({
                    "op": 0,
                    "t": "GUILD_MEMBER_LIST_UPDATE",
                    "s": 2,
                    "d": {
                        "guild_id": "guild",
                        "member_count": 2,
                        "ops": [{
                            "op": "SYNC",
                            "range": [0, 1],
                            "items": [
                                {"member": {"user": {"id": "100"}, "nick": "PlayerOne", "roles": ["guild", "role"]}},
                                {"member": {"user": {"id": "101"}, "nick": null, "roles": []}}
                            ]
                        }]
                    }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
    });

    let gateway = DiscordGateway::new_with_url(
        "user-token".into(),
        "guild".into(),
        format!("ws://{address}"),
    );
    let mut members = gateway.fetch_snapshot().await.unwrap();
    members.sort_by(|left, right| left.discord_id.cmp(&right.discord_id));

    assert_eq!(members.len(), 1);
    assert_eq!(members[0].discord_id, "100");
    assert_eq!(members[0].nickname, "PlayerOne");
    assert_eq!(members[0].role_ids, vec!["role"]);
    server.await.unwrap();
}

async fn next_json<S>(socket: &mut S) -> Value
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        match socket.next().await.unwrap().unwrap() {
            Message::Text(text) => return serde_json::from_str(&text).unwrap(),
            Message::Binary(bytes) => return serde_json::from_slice(&bytes).unwrap(),
            _ => {}
        }
    }
}
