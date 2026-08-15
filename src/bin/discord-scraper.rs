use clap::Parser;
use gytags_roster::collector::{CollectorClient, CollectorClientConfig, DiscordGateway};
use gytags_roster::scraper::run;
use gytags_roster::shutdown;
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

const BACKEND_URL: &str = "http://api:8080";

#[derive(Debug, Parser)]
#[command(name = "discord-scraper")]
struct Args {
    #[arg(long, env = "GYTAGS_INGEST_TOKEN")]
    ingest_token: Option<String>,
    #[arg(long, env = "GYTAGS_DISCORD_TOKEN")]
    discord_token: String,
    #[arg(long, env = "GYTAGS_DISCORD_GUILD_ID")]
    guild_id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();
    let args = Args::parse();
    let gateway = DiscordGateway::new(args.discord_token, args.guild_id);
    let client = CollectorClient::new(CollectorClientConfig {
        base_url: BACKEND_URL.to_owned(),
        ingest_token: args.ingest_token,
    })?;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let task = tokio::spawn(run(gateway, client, shutdown_rx));
    tokio::select! {
        result = task => {
            if let Err(error) = result {
                tracing::error!(%error, "discord scraper task stopped");
            }
        }
        _ = shutdown::wait() => {
            let _ = shutdown_tx.send(true);
        }
    }
    Ok(())
}
