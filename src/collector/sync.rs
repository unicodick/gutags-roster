use super::CollectorError;
use crate::domain::{
    BadgeRule, MemberOverride, MemberRecord, RawMember, build_member_record, normalize_nickname,
};
use crate::storage::{Repository, now_unix};
use std::collections::HashSet;
use std::path::Path;
use tokio::sync::broadcast;

const MIN_RETAINED_SNAPSHOT_PERCENT: usize = 50;

#[derive(Debug, Clone)]
pub struct ChangeEvent {
    pub revision: i64,
}

#[derive(Clone)]
pub struct SnapshotSync {
    repository: Repository,
    badge_rules: Vec<BadgeRule>,
    member_overrides: Vec<MemberOverride>,
    events: broadcast::Sender<ChangeEvent>,
}

impl SnapshotSync {
    pub fn new(
        repository: Repository,
        badge_rules: Vec<BadgeRule>,
        member_overrides: Vec<MemberOverride>,
        events: broadcast::Sender<ChangeEvent>,
    ) -> Self {
        Self {
            repository,
            badge_rules,
            member_overrides,
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
        let records = apply_overrides(records, &self.member_overrides, observed_at)?;
        self.apply_records(records, observed_at).await
    }

    async fn apply_records(
        &self,
        records: Vec<crate::domain::MemberRecord>,
        synced_at: i64,
    ) -> Result<i64, CollectorError> {
        let previous_records = self.repository.members_by_keys(&[]).await?;
        let previous_size = previous_records.len();
        let actual_size = records.len();
        let minimum_size = previous_size
            .saturating_mul(MIN_RETAINED_SNAPSHOT_PERCENT)
            .div_ceil(100);
        if actual_size < minimum_size {
            return Err(CollectorError::SnapshotTooSmall {
                actual: actual_size,
                minimum: minimum_size,
            });
        }

        if snapshots_equal(&previous_records, &records) {
            return Ok(self.repository.record_sync(synced_at).await?);
        }

        let revision = self
            .repository
            .replace_snapshot(&records, synced_at)
            .await?;
        let _ = self.events.send(ChangeEvent { revision });
        Ok(revision)
    }
}

fn apply_overrides(
    mut records: Vec<MemberRecord>,
    overrides: &[MemberOverride],
    observed_at: i64,
) -> Result<Vec<MemberRecord>, CollectorError> {
    for override_record in overrides {
        let nickname_key = normalize_nickname(&override_record.nickname)?;
        if let Some(record) = records
            .iter_mut()
            .find(|record| record.discord_id == override_record.discord_id)
        {
            record.nickname_raw = override_record.nickname.trim().to_owned();
            record.nickname_key = nickname_key;
            record.observed_at = observed_at;
        }
    }

    Ok(records)
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
