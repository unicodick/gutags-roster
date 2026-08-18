use anyhow::Context;
use std::path::Path;
use tokio::sync::{broadcast, watch};
use tracing_subscriber::EnvFilter;

use gutags_roster::api::{AppState, router};
use gutags_roster::collector::{DiscordGateway, SnapshotSync, ensure_parent_directory};
use gutags_roster::config::{Settings, load_badge_rules, load_member_overrides};
use gutags_roster::scraper::run;
use gutags_roster::storage::Repository;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    let settings = Settings::from_env()?;
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
        source_ttl_seconds: settings.source_ttl_seconds,
    };
    let gateway = DiscordGateway::new(settings.discord_token, settings.discord_guild_id);

    let listener = tokio::net::TcpListener::bind(&settings.bind_addr).await?;
    tracing::info!(address = %settings.bind_addr, "backend started");
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(async move {
        gutags_roster::shutdown::wait().await;
        let _ = shutdown_tx.send(true);
    });

    let api = axum::serve(listener, router(state))
        .with_graceful_shutdown(gutags_roster::shutdown::requested(shutdown_rx.clone()));
    tokio::try_join!(async { api.await.context("API server failed") }, async {
        run(gateway, sync, shutdown_rx).await;
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
