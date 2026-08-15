use crate::collector::{CollectorClient, DiscordGateway};
use tokio::sync::watch;

use std::time::Duration;

const FULL_SYNC_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const RETRY_INTERVAL: Duration = Duration::from_secs(5 * 60);

pub async fn run(
    gateway: DiscordGateway,
    client: CollectorClient,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut next_sync = Box::pin(tokio::time::sleep(Duration::ZERO));

    loop {
        tokio::select! {
            _ = &mut next_sync => {
                match gateway.fetch_snapshot().await {
                    Ok(snapshot) => {
                        let member_count = snapshot.len();
                        match client.snapshot(snapshot).await {
                        Ok(response) => {
                            tracing::info!(revision = response.revision, members = member_count, "discord snapshot ingested");
                            next_sync = Box::pin(tokio::time::sleep(FULL_SYNC_INTERVAL));
                        }
                        Err(error) => {
                            tracing::error!(%error, "discord snapshot ingest failed");
                            next_sync = Box::pin(tokio::time::sleep(RETRY_INTERVAL));
                        }
                        }
                    }
                    Err(error) => {
                        tracing::error!(%error, "discord snapshot collection failed");
                        next_sync = Box::pin(tokio::time::sleep(RETRY_INTERVAL));
                    }
                }
            }
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return;
                }
            }
        }
    }
}
