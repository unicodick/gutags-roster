use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawMember {
    pub discord_id: String,
    pub nickname: String,
    #[serde(default)]
    pub role_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BadgeGroup {
    Career,
    Team,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BadgeRule {
    pub role_id: String,
    pub badge_id: String,
    pub group: BadgeGroup,
    pub priority: i32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MemberOverride {
    pub discord_id: String,
    pub nickname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemberRecord {
    pub discord_id: String,
    pub nickname_raw: String,
    pub nickname_key: String,
    pub role_ids: Vec<String>,
    pub badges: Vec<String>,
    pub observed_at: i64,
}
