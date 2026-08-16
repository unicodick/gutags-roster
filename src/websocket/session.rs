use crate::api::{AppState, public_members};
use crate::collector::ChangeEvent;
use crate::domain::normalize_nicknames;
use crate::protocol::{ClientMessage, PROTOCOL_VERSION, ServerMessage};
use crate::storage::{age_seconds, now_unix};
use axum::extract::{
    State,
    ws::{Message, WebSocket, WebSocketUpgrade},
};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::broadcast;
use tokio::time::{Duration, timeout};

pub async fn websocket(
    upgrade: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let events = state.events.subscribe();
    upgrade.on_upgrade(move |socket| async move {
        if let Err(error) = handle_socket(socket, state, events).await {
            tracing::debug!(%error, "websocket session ended");
        }
    })
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    mut events: broadcast::Receiver<ChangeEvent>,
) -> Result<(), anyhow::Error> {
    let (mut sender, mut receiver) = socket.split();
    let first_message = timeout(Duration::from_secs(10), receiver.next())
        .await
        .map_err(|_| anyhow::anyhow!("hello timeout"))?
        .ok_or_else(|| anyhow::anyhow!("client disconnected before hello"))??;
    let hello = match first_message {
        Message::Text(text) => serde_json::from_str::<ClientMessage>(text.as_str())?,
        _ => anyhow::bail!("first websocket message must be hello"),
    };

    let mut subscribed_keys = match hello {
        ClientMessage::Hello {
            protocol_version,
            nicknames,
        } => {
            if protocol_version != PROTOCOL_VERSION {
                send_json(
                    &mut sender,
                    &ServerMessage::Error {
                        code: "unsupported_protocol".into(),
                        message: format!("supported protocol version is {PROTOCOL_VERSION}"),
                    },
                )
                .await?;
                return Ok(());
            }
            normalize_nicknames(nicknames)?
        }
        _ => anyhow::bail!("first websocket message must be hello"),
    };

    let status = state.repository.status().await?;
    let now = now_unix();
    send_json(
        &mut sender,
        &ServerMessage::HelloAck {
            protocol_version: PROTOCOL_VERSION,
            revision: status.revision,
            source_status: status.effective_source_status(state.source_ttl_seconds, now),
            source_age_seconds: age_seconds(status.last_source_sync_at, now),
        },
    )
    .await?;
    send_snapshot(&state, &mut sender, &subscribed_keys).await?;

    loop {
        tokio::select! {
            incoming = receiver.next() => handle_incoming(
                incoming,
                &state,
                &mut sender,
                &mut subscribed_keys,
            ).await?,
            event = events.recv() => handle_event(
                event,
                &state,
                &mut sender,
                &subscribed_keys,
            ).await?,
        }
    }
}

async fn handle_incoming(
    incoming: Option<Result<Message, axum::Error>>,
    state: &AppState,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    subscribed_keys: &mut Vec<String>,
) -> Result<(), anyhow::Error> {
    match incoming {
        Some(Ok(Message::Text(text))) => match serde_json::from_str::<ClientMessage>(text.as_str())
        {
            Ok(ClientMessage::Subscribe { nicknames }) => {
                *subscribed_keys = normalize_nicknames(nicknames)?;
                send_snapshot(state, sender, subscribed_keys).await?;
            }
            Ok(ClientMessage::Ping) => send_json(sender, &ServerMessage::Pong).await?,
            Ok(ClientMessage::Hello { .. }) => {
                send_json(
                    sender,
                    &ServerMessage::Error {
                        code: "hello_already_received".into(),
                        message: "hello may only be sent once".into(),
                    },
                )
                .await?
            }
            Err(error) => {
                send_json(
                    sender,
                    &ServerMessage::Error {
                        code: "invalid_message".into(),
                        message: error.to_string(),
                    },
                )
                .await?
            }
        },
        Some(Ok(Message::Ping(payload))) => sender.send(Message::Pong(payload)).await?,
        Some(Ok(Message::Close(_))) | None => anyhow::bail!("client disconnected"),
        Some(Ok(Message::Binary(_))) | Some(Ok(Message::Pong(_))) => {}
        Some(Err(error)) => return Err(error.into()),
    }
    Ok(())
}

async fn handle_event(
    event: Result<ChangeEvent, broadcast::error::RecvError>,
    state: &AppState,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    subscribed_keys: &[String],
) -> Result<(), anyhow::Error> {
    match event {
        Ok(ChangeEvent { revision }) => {
            let records = state.repository.members_by_keys(subscribed_keys).await?;
            send_json(
                sender,
                &ServerMessage::Update {
                    revision,
                    members: public_members(subscribed_keys, records),
                },
            )
            .await?;
        }
        Err(broadcast::error::RecvError::Lagged(_)) => {
            send_snapshot(state, sender, subscribed_keys).await?;
        }
        Err(broadcast::error::RecvError::Closed) => anyhow::bail!("event bus closed"),
    }
    Ok(())
}

async fn send_snapshot(
    state: &AppState,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    keys: &[String],
) -> Result<(), anyhow::Error> {
    let status = state.repository.status().await?;
    let now = now_unix();
    let records = state.repository.members_by_keys(keys).await?;
    send_json(
        sender,
        &ServerMessage::Snapshot {
            revision: status.revision,
            source_status: status.effective_source_status(state.source_ttl_seconds, now),
            source_age_seconds: age_seconds(status.last_source_sync_at, now),
            members: public_members(keys, records),
        },
    )
    .await
}

async fn send_json<T: Serialize>(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    value: &T,
) -> Result<(), anyhow::Error> {
    sender
        .send(Message::Text(serde_json::to_string(value)?.into()))
        .await?;
    Ok(())
}
