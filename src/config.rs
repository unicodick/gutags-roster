use crate::domain::BadgeRule;
use serde::Deserialize;
use std::env;
use std::path::Path;
use thiserror::Error;

pub const BIND_ADDR: &str = "0.0.0.0:8080";
pub const DATABASE_URL: &str = "sqlite://data/gytags.sqlite3";
pub const BADGE_RULES_PATH: &str = "config/badges.json";
pub const SOURCE_TTL_SECONDS: u64 = 172_800;

#[derive(Debug, Clone)]
pub struct Settings {
    pub bind_addr: String,
    pub database_url: String,
    pub badge_rules_path: String,
    pub websocket_token: Option<String>,
    pub ingest_token: Option<String>,
    pub source_ttl_seconds: u64,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read badge rules: {0}")]
    BadgeRulesIo(#[from] std::io::Error),
    #[error("could not parse badge rules: {0}")]
    BadgeRulesJson(#[from] serde_json::Error),
    #[error("invalid badge rule at index {index}: {message}")]
    InvalidBadgeRule { index: usize, message: &'static str },
}

impl Settings {
    pub fn from_env() -> Self {
        Self {
            bind_addr: BIND_ADDR.to_owned(),
            database_url: DATABASE_URL.to_owned(),
            badge_rules_path: BADGE_RULES_PATH.to_owned(),
            websocket_token: env::var("GYTAGS_WS_TOKEN").ok(),
            ingest_token: env::var("GYTAGS_INGEST_TOKEN").ok(),
            source_ttl_seconds: SOURCE_TTL_SECONDS,
        }
    }
}

pub fn load_badge_rules(path: impl AsRef<Path>) -> Result<Vec<BadgeRule>, ConfigError> {
    let contents = std::fs::read_to_string(path)?;
    let rules = serde_json::from_str::<BadgeRulesFile>(&contents)?.rules;
    for (index, rule) in rules.iter().enumerate() {
        if rule.role_id.trim().is_empty() {
            return Err(ConfigError::InvalidBadgeRule {
                index,
                message: "role_id cannot be empty",
            });
        }
        if rule.badge_id.trim().is_empty() {
            return Err(ConfigError::InvalidBadgeRule {
                index,
                message: "badge_id cannot be empty",
            });
        }
    }
    Ok(rules)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BadgeRulesFile {
    #[serde(default)]
    rules: Vec<BadgeRule>,
}
