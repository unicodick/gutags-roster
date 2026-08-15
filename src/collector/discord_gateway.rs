use super::CollectorError;
use super::discord_sidebar::{MemberListUpdate, RANGES_PER_REQUEST, parse_update, plan_ranges};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const OP_DISPATCH: u64 = 0;
const OP_HEARTBEAT: u64 = 1;
const OP_IDENTIFY: u64 = 2;
const OP_RECONNECT: u64 = 7;
const OP_INVALID_SESSION: u64 = 9;
const OP_HELLO: u64 = 10;
const OP_HEARTBEAT_ACK: u64 = 11;
const OP_BULK_GUILD_SUBSCRIBE: u64 = 37;
const OP_QOS_HEARTBEAT: u64 = 40;
const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=9&encoding=json";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const SCRAPE_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_MEMBERS: usize = 100_000;
const MIN_NICKED_MEMBERS: usize = 1;

pub struct DiscordGateway {
    token: String,
    guild_id: String,
    gateway_url: String,
}

impl DiscordGateway {
    pub fn new(token: String, guild_id: String) -> Self {
        Self {
            token,
            guild_id,
            gateway_url: GATEWAY_URL.into(),
        }
    }

    pub fn new_with_url(token: String, guild_id: String, gateway_url: String) -> Self {
        Self {
            token,
            guild_id,
            gateway_url,
        }
    }

    pub async fn fetch_snapshot(&self) -> Result<Vec<crate::domain::RawMember>, CollectorError> {
        if self.token.trim().is_empty() || self.guild_id.trim().is_empty() {
            return Err(CollectorError::Configuration(
                "Discord token and guild ID are required".into(),
            ));
        }

        let members = tokio::time::timeout(SCRAPE_TIMEOUT, self.fetch_snapshot_inner())
            .await
            .map_err(|_| {
                CollectorError::Gateway(format!(
                    "scrape timed out after {} seconds",
                    SCRAPE_TIMEOUT.as_secs()
                ))
            })??;
        if members.len() < MIN_NICKED_MEMBERS {
            return Err(CollectorError::SnapshotTooSmall {
                actual: members.len(),
                minimum: MIN_NICKED_MEMBERS,
            });
        }
        Ok(members)
    }

    async fn fetch_snapshot_inner(&self) -> Result<Vec<crate::domain::RawMember>, CollectorError> {
        tracing::info!(guild_id = %self.guild_id, "starting Discord snapshot scrape");
        let connect = tokio::time::timeout(REQUEST_TIMEOUT, connect_async(&self.gateway_url))
            .await
            .map_err(|_| CollectorError::Gateway("gateway connection timed out".into()))?
            .map_err(|error| CollectorError::Gateway(error.to_string()))?;
        let (mut socket, _) = connect;
        tracing::debug!("Discord Gateway websocket connected");
        let mut heartbeat: Option<tokio::time::Interval> = None;
        let mut sequence = None;
        let mut identified = false;
        let mut ready = false;
        let mut ranges = VecDeque::new();
        let mut channel_ids = Vec::new();
        let mut requested = Vec::new();
        let mut received = Vec::new();
        let mut members = HashMap::new();
        let mut heartbeat_ack = true;

        loop {
            let message = if let Some(interval) = heartbeat.as_mut() {
                tokio::select! {
                    _ = interval.tick() => {
                        if !heartbeat_ack {
                            return Err(CollectorError::Gateway("heartbeat was not acknowledged".into()));
                        }
                        send_heartbeat(&mut socket, sequence).await?;
                        heartbeat_ack = false;
                        continue;
                    }
                    message = socket.next() => message,
                }
            } else {
                socket.next().await
            };

            let Some(message) = message else {
                return Err(CollectorError::Gateway(
                    "gateway closed the connection".into(),
                ));
            };
            let message = message.map_err(|error| CollectorError::Gateway(error.to_string()))?;
            let Some(payload) = decode_message(message)? else {
                continue;
            };
            let opcode = payload.get("op").and_then(Value::as_u64).ok_or_else(|| {
                CollectorError::GatewayProtocol("gateway message has no opcode".into())
            })?;
            if let Some(value) = payload.get("s").and_then(Value::as_u64) {
                sequence = Some(value);
            }

            match opcode {
                OP_HELLO => {
                    let interval_ms = payload
                        .get("d")
                        .and_then(|value| value.get("heartbeat_interval"))
                        .and_then(Value::as_u64)
                        .ok_or_else(|| {
                            CollectorError::GatewayProtocol(
                                "gateway HELLO has no heartbeat interval".into(),
                            )
                        })?;
                    let mut interval =
                        tokio::time::interval(Duration::from_millis(interval_ms.max(1)));
                    interval.tick().await;
                    heartbeat = Some(interval);
                    tracing::debug!(
                        heartbeat_interval_ms = interval_ms,
                        "received Discord Gateway HELLO"
                    );
                    if !identified {
                        send_json(&mut socket, identify_payload(&self.token)).await?;
                        identified = true;
                        tracing::debug!("sent Discord Gateway IDENTIFY");
                    }
                }
                OP_HEARTBEAT => {
                    send_heartbeat(&mut socket, sequence).await?;
                }
                OP_HEARTBEAT_ACK => heartbeat_ack = true,
                OP_RECONNECT | OP_INVALID_SESSION => {
                    return Err(CollectorError::Gateway(format!(
                        "gateway requested reconnect (opcode {opcode})"
                    )));
                }
                OP_DISPATCH => {
                    let event_name = payload.get("t").and_then(Value::as_str).unwrap_or_default();
                    match event_name {
                        "READY" if !ready => {
                            let guild = find_guild(&payload, &self.guild_id)?;
                            let member_count = guild
                                .get("member_count")
                                .and_then(Value::as_u64)
                                .ok_or_else(|| {
                                    CollectorError::GatewayProtocol(
                                        "READY guild has no member_count".into(),
                                    )
                                })? as usize;
                            if member_count > MAX_MEMBERS {
                                return Err(CollectorError::Configuration(format!(
                                    "guild has {member_count} members, limit is {MAX_MEMBERS}"
                                )));
                            }
                            tracing::debug!(member_count, "received Discord Gateway READY");
                            channel_ids = channels(guild)?;
                            ranges = plan_ranges(member_count).into();
                            ready = true;
                            if ranges.is_empty() {
                                return Ok(Vec::new());
                            }
                            requested = take_ranges(&mut ranges);
                            tracing::debug!(
                                ?requested,
                                channels = channel_ids.len(),
                                "requesting Discord member-list ranges"
                            );
                            send_subscription(
                                &mut socket,
                                &self.guild_id,
                                &channel_ids,
                                &requested,
                            )
                            .await?;
                        }
                        "GUILD_MEMBER_LIST_UPDATE" if ready => {
                            let update = parse_update(&payload, &self.guild_id)?;
                            tracing::debug!(members = update.members.len(), sync_ranges = ?update.sync_ranges, "received Discord member-list update");
                            merge_update(&mut members, &update);
                            received.extend(update.sync_ranges);
                            if ranges_are_covered(&requested, &received) {
                                if ranges.is_empty() {
                                    tracing::info!(
                                        members = members.len(),
                                        "completed Discord member-list scrape"
                                    );
                                    return Ok(members.into_values().collect());
                                }
                                requested = take_ranges(&mut ranges);
                                received.clear();
                                tracing::debug!(
                                    ?requested,
                                    "requesting next Discord member-list ranges"
                                );
                                send_subscription(
                                    &mut socket,
                                    &self.guild_id,
                                    &channel_ids,
                                    &requested,
                                )
                                .await?;
                            }
                        }
                        _ => {}
                    }
                }
                OP_BULK_GUILD_SUBSCRIBE => {}
                _ => {}
            }
        }
    }
}

fn decode_message(message: Message) -> Result<Option<Value>, CollectorError> {
    let text = match message {
        Message::Text(text) => text.to_string(),
        Message::Binary(bytes) => String::from_utf8(bytes.to_vec())
            .map_err(|error| CollectorError::GatewayProtocol(error.to_string()))?,
        Message::Ping(_) | Message::Pong(_) => return Ok(None),
        Message::Close(frame) => {
            let details = frame
                .map(|frame| format!("code={} reason={}", u16::from(frame.code), frame.reason))
                .unwrap_or_else(|| "without a close frame".into());
            return Err(CollectorError::Gateway(format!(
                "gateway closed the connection ({details})"
            )));
        }
        Message::Frame(_) => return Ok(None),
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| CollectorError::GatewayProtocol(format!("invalid gateway JSON: {error}")))
}

async fn send_json<S>(socket: &mut S, payload: Value) -> Result<(), CollectorError>
where
    S: futures_util::Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    socket
        .send(Message::Text(payload.to_string().into()))
        .await
        .map_err(|error| CollectorError::Gateway(error.to_string()))
}

async fn send_heartbeat<S>(socket: &mut S, sequence: Option<u64>) -> Result<(), CollectorError>
where
    S: futures_util::Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    send_json(
        socket,
        json!({
            "op": OP_QOS_HEARTBEAT,
            "d": {
                "qos": {"ver": 27, "active": true, "reasons": ["foregrounded"]},
                "seq": sequence
            }
        }),
    )
    .await
}

fn identify_payload(token: &str) -> Value {
    json!({
        "op": OP_IDENTIFY,
        "d": {
            "token": token,
            "capabilities": 22525,
            "properties": {"os": "linux", "browser": "Discord Client", "device": "Discord Client"},
            "presence": {"status": "online", "since": 0, "activities": [], "afk": false},
            "compress": false,
            "client_state": {"guild_versions": {}}
        }
    })
}

async fn send_subscription<S>(
    socket: &mut S,
    guild_id: &str,
    channel_ids: &[String],
    ranges: &[(u64, u64)],
) -> Result<(), CollectorError>
where
    S: futures_util::Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    let channels = channel_ids
        .iter()
        .map(|channel_id| (channel_id.clone(), json!(ranges)))
        .collect::<serde_json::Map<_, _>>();
    send_json(
        socket,
        json!({
            "op": OP_BULK_GUILD_SUBSCRIBE,
            "d": {
                "subscriptions": {
                    guild_id: {
                        "typing": true,
                        "threads": true,
                        "activities": true,
                        "member_updates": true,
                        "channels": channels
                    }
                }
            }
        }),
    )
    .await
}

fn take_ranges(ranges: &mut VecDeque<(u64, u64)>) -> Vec<(u64, u64)> {
    ranges
        .drain(..RANGES_PER_REQUEST.min(ranges.len()))
        .collect()
}

fn ranges_are_covered(required: &[(u64, u64)], received: &[(u64, u64)]) -> bool {
    required.iter().all(|required_range| {
        received.iter().any(|received_range| {
            received_range.0 <= required_range.0 && received_range.1 >= required_range.1
        })
    })
}

fn merge_update(
    members: &mut HashMap<String, crate::domain::RawMember>,
    update: &MemberListUpdate,
) {
    for member in &update.members {
        members.insert(member.discord_id.clone(), member.clone());
    }
}

fn find_guild<'a>(payload: &'a Value, guild_id: &str) -> Result<&'a Value, CollectorError> {
    payload
        .get("d")
        .and_then(|data| data.get("guilds"))
        .and_then(Value::as_array)
        .and_then(|guilds| {
            guilds
                .iter()
                .find(|guild| guild.get("id").and_then(Value::as_str) == Some(guild_id))
        })
        .ok_or_else(|| {
            CollectorError::GatewayProtocol(format!("guild {guild_id} not found in READY"))
        })
}

fn channels(guild: &Value) -> Result<Vec<String>, CollectorError> {
    let ids = guild
        .get("channels")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|channel| matches!(channel.get("type").and_then(Value::as_u64), Some(0 | 5)))
        .filter_map(|channel| channel.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .take(5)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Err(CollectorError::Configuration(
            "no text channels available for member sidebar".into(),
        ));
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::ranges_are_covered;

    #[test]
    fn accepts_server_coalesced_ranges() {
        assert!(ranges_are_covered(
            &[(500, 592)],
            &[(300, 399), (400, 499), (500, 599)]
        ));
    }

    #[test]
    fn rejects_partial_range_coverage() {
        assert!(!ranges_are_covered(&[(500, 592)], &[(500, 591)]));
    }
}
