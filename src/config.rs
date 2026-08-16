use crate::domain::{BadgeRule, MemberOverride};
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::path::Path;
use thiserror::Error;

pub const BIND_ADDR: &str = "0.0.0.0:8080";
pub const DATABASE_URL: &str = "sqlite://data/gytags.sqlite3";
pub const BADGE_RULES_PATH: &str = "config/badges.json";
pub const MEMBER_OVERRIDES_PATH: &str = "config/overrides.json";
pub const SOURCE_TTL_SECONDS: u64 = 172_800;

#[derive(Debug, Clone)]
pub struct Settings {
    pub bind_addr: String,
    pub database_url: String,
    pub badge_rules_path: String,
    pub member_overrides_path: String,
    pub discord_token: String,
    pub discord_guild_id: String,
    pub source_ttl_seconds: u64,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read config: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not parse config: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid badge rule at index {index}: {message}")]
    InvalidBadgeRule { index: usize, message: &'static str },
    #[error("invalid member override at index {index}: {message}")]
    InvalidMemberOverride { index: usize, message: &'static str },
    #[error("{0} is required")]
    MissingEnvironment(&'static str),
}

impl Settings {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            bind_addr: BIND_ADDR.to_owned(),
            database_url: DATABASE_URL.to_owned(),
            badge_rules_path: BADGE_RULES_PATH.to_owned(),
            member_overrides_path: MEMBER_OVERRIDES_PATH.to_owned(),
            discord_token: required_env("GYTAGS_DISCORD_TOKEN")?,
            discord_guild_id: required_env("GYTAGS_DISCORD_GUILD_ID")?,
            source_ttl_seconds: SOURCE_TTL_SECONDS,
        })
    }
}

fn required_env(name: &'static str) -> Result<String, ConfigError> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or(ConfigError::MissingEnvironment(name))
}

pub fn load_member_overrides(path: impl AsRef<Path>) -> Result<Vec<MemberOverride>, ConfigError> {
    let contents = std::fs::read_to_string(path)?;
    let mut members = serde_json::from_str::<MemberOverridesFile>(&contents)?.members;
    let mut discord_ids = HashSet::with_capacity(members.len());
    for (index, member) in members.iter_mut().enumerate() {
        member.discord_id = member.discord_id.trim().to_owned();
        member.nickname = member.nickname.trim().to_owned();
        if member.discord_id.is_empty() {
            return Err(ConfigError::InvalidMemberOverride {
                index,
                message: "discord_id cannot be empty",
            });
        }
        if member.nickname.is_empty() {
            return Err(ConfigError::InvalidMemberOverride {
                index,
                message: "nickname cannot be empty",
            });
        }
        if !discord_ids.insert(member.discord_id.clone()) {
            return Err(ConfigError::InvalidMemberOverride {
                index,
                message: "discord_id must be unique",
            });
        }
    }
    Ok(members)
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberOverridesFile {
    #[serde(default)]
    members: Vec<MemberOverride>,
}
