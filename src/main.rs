use anyhow::Context;
use std::path::Path;
use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;

use gytags_roster::api::{AppState, router};
use gytags_roster::collector::{SnapshotSync, ensure_parent_directory};
use gytags_roster::config::{Settings, load_badge_rules, load_member_overrides};
use gytags_roster::storage::Repository;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    let settings = Settings::from_env();
    if settings.database_url.starts_with("sqlite://") {
        let path = settings.database_url.trim_start_matches("sqlite://");
        if path != ":memory:" {
            ensure_parent_directory(Path::new(path))?;
        }
    }

    let repository = Repository::connect(&settings.database_url).await?;
    let badge_rules = load_badge_rules(&settings.badge_rules_path)
        .with_context(|| format!("loading {}", settings.badge_rules_path))?;
    let member_overrides = load_member_overrides(&settings.member_overrides_path)
        .with_context(|| format!("loading {}", settings.member_overrides_path))?;
    let (events, _) = broadcast::channel(128);
    let sync = SnapshotSync::new(
        repository.clone(),
        badge_rules,
        member_overrides,
        events.clone(),
    );
    let state = AppState {
        repository: repository.clone(),
        events: events.clone(),
        websocket_token: settings.websocket_token.clone(),
        ingest_token: settings.ingest_token.clone(),
        sync,
        source_ttl_seconds: settings.source_ttl_seconds,
    };

    let listener = tokio::net::TcpListener::bind(&settings.bind_addr).await?;
    tracing::info!(address = %settings.bind_addr, "backend started");
    axum::serve(listener, router(state))
        .with_graceful_shutdown(gytags_roster::shutdown::wait())
        .await?;

    Ok(())
}
