use crate::collector::{DiscordGateway, SnapshotSync};
use tokio::sync::watch;

use std::time::Duration;

const FULL_SYNC_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const RETRY_INTERVAL: Duration = Duration::from_secs(5 * 60);

pub async fn run(gateway: DiscordGateway, sync: SnapshotSync, mut shutdown: watch::Receiver<bool>) {
    let mut delay = Duration::ZERO;

    loop {
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            _ = wait_for_shutdown(&mut shutdown) => return,
        }

        let snapshot = tokio::select! {
            result = gateway.fetch_snapshot() => result,
            _ = wait_for_shutdown(&mut shutdown) => return,
        };
        delay = match snapshot {
            Ok(snapshot) => {
                let member_count = snapshot.len();
                match sync.apply(snapshot).await {
                    Ok(revision) => {
                        tracing::info!(revision, members = member_count, "discord snapshot synced");
                        FULL_SYNC_INTERVAL
                    }
                    Err(error) => {
                        tracing::error!(%error, "discord snapshot sync failed");
                        RETRY_INTERVAL
                    }
                }
            }
            Err(error) => {
                tracing::error!(%error, "discord snapshot collection failed");
                RETRY_INTERVAL
            }
        };
    }
}

async fn wait_for_shutdown(shutdown: &mut watch::Receiver<bool>) {
    let _ = shutdown.wait_for(|requested| *requested).await;
}
