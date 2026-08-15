use crate::collector::{ChangeEvent, SnapshotSync};
use crate::storage::Repository;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub repository: Repository,
    pub events: broadcast::Sender<ChangeEvent>,
    pub websocket_token: Option<String>,
    pub ingest_token: Option<String>,
    pub sync: SnapshotSync,
    pub source_ttl_seconds: u64,
}
