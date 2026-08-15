use super::CollectorError;
use crate::domain::{BadgeRule, RawMember, build_member_record};
use crate::storage::{Repository, now_unix};
use std::collections::HashSet;
use std::path::Path;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct ChangeEvent {
    pub revision: i64,
}

#[derive(Clone)]
pub struct SnapshotSync {
    repository: Repository,
    badge_rules: Vec<BadgeRule>,
    events: broadcast::Sender<ChangeEvent>,
}

impl SnapshotSync {
    pub fn new(
        repository: Repository,
        badge_rules: Vec<BadgeRule>,
        events: broadcast::Sender<ChangeEvent>,
    ) -> Self {
        Self {
            repository,
            badge_rules,
            events,
        }
    }

    pub async fn apply(&self, raw_members: Vec<RawMember>) -> Result<i64, CollectorError> {
        validate_unique_discord_ids(&raw_members)?;
        let observed_at = now_unix();
        let records = raw_members
            .into_iter()
            .map(|member| build_member_record(member, &self.badge_rules, observed_at))
            .collect::<Result<Vec<_>, _>>()?;
        self.apply_records(records).await
    }

    async fn apply_records(
        &self,
        records: Vec<crate::domain::MemberRecord>,
    ) -> Result<i64, CollectorError> {
        let previous_records = self.repository.members_by_keys(&[]).await?;
        if snapshots_equal(&previous_records, &records) {
            self.repository.record_sync(now_unix()).await?;
            return Ok(self.repository.status().await?.revision);
        }

        let revision = self.repository.replace_snapshot(&records).await?;
        self.repository.record_sync(now_unix()).await?;
        let _ = self.events.send(ChangeEvent { revision });
        Ok(revision)
    }
}

fn snapshots_equal(
    left: &[crate::domain::MemberRecord],
    right: &[crate::domain::MemberRecord],
) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut left = left.iter().collect::<Vec<_>>();
    let mut right = right.iter().collect::<Vec<_>>();
    left.sort_by(|a, b| a.discord_id.cmp(&b.discord_id));
    right.sort_by(|a, b| a.discord_id.cmp(&b.discord_id));

    left.iter().zip(right).all(|(left, right)| {
        left.discord_id == right.discord_id
            && left.nickname_raw == right.nickname_raw
            && left.nickname_key == right.nickname_key
            && left.role_ids == right.role_ids
            && left.badges == right.badges
    })
}

fn validate_unique_discord_ids(raw_members: &[RawMember]) -> Result<(), CollectorError> {
    let mut ids = HashSet::with_capacity(raw_members.len());
    for member in raw_members {
        let id = member.discord_id.trim();
        if !ids.insert(id.to_owned()) {
            return Err(crate::domain::DomainError::DuplicateDiscordId(id.to_owned()).into());
        }
    }
    Ok(())
}

pub fn ensure_parent_directory(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
