use serde::{Deserialize, Serialize};

use crate::domain::RawMember;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello {
        protocol_version: u16,
        #[serde(default)]
        token: Option<String>,
        #[serde(default)]
        nicknames: Vec<String>,
    },
    Subscribe {
        nicknames: Vec<String>,
    },
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IngestRequest {
    Snapshot { members: Vec<RawMember> },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestResponse {
    pub status: String,
    pub revision: i64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    HelloAck {
        protocol_version: u16,
        revision: i64,
        source_status: String,
        source_age_seconds: Option<i64>,
    },
    Snapshot {
        revision: i64,
        source_status: String,
        source_age_seconds: Option<i64>,
        members: Vec<PublicMember>,
    },
    Update {
        revision: i64,
        members: Vec<PublicMember>,
    },
    Pong,
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicMember {
    pub nickname: String,
    pub status: PublicMemberStatus,
    pub badges: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicMemberStatus {
    Ok,
    Ambiguous,
    NotFound,
}
