use crate::domain::MemberRecord;
use crate::protocol::{PublicMember, PublicMemberStatus};
use std::collections::HashMap;

pub fn public_members(keys: &[String], records: Vec<MemberRecord>) -> Vec<PublicMember> {
    let mut grouped: HashMap<String, Vec<MemberRecord>> = HashMap::new();
    for record in records {
        grouped
            .entry(record.nickname_key.clone())
            .or_default()
            .push(record);
    }

    if keys.is_empty() {
        let mut members = grouped
            .into_values()
            .map(public_member_group)
            .filter(|member| matches!(&member.status, PublicMemberStatus::Ok))
            .collect::<Vec<_>>();
        members.sort_by(|left, right| {
            left.nickname
                .to_lowercase()
                .cmp(&right.nickname.to_lowercase())
        });
        return members;
    }

    keys.iter()
        .map(|key| match grouped.remove(key) {
            Some(records) => public_member_group(records),
            None => PublicMember {
                nickname: key.clone(),
                status: PublicMemberStatus::NotFound,
                badges: Vec::new(),
            },
        })
        .collect()
}

fn public_member_group(records: Vec<MemberRecord>) -> PublicMember {
    let first = &records[0];
    if records.len() > 1 {
        return PublicMember {
            nickname: first.nickname_raw.clone(),
            status: PublicMemberStatus::Ambiguous,
            badges: Vec::new(),
        };
    }

    if first.badges.is_empty() {
        return PublicMember {
            nickname: first.nickname_raw.clone(),
            status: PublicMemberStatus::NotFound,
            badges: Vec::new(),
        };
    }

    PublicMember {
        nickname: first.nickname_raw.clone(),
        status: PublicMemberStatus::Ok,
        badges: first.badges.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(nickname: &str, badges: &[&str]) -> MemberRecord {
        MemberRecord {
            discord_id: nickname.to_owned(),
            nickname_raw: nickname.to_owned(),
            nickname_key: nickname.to_lowercase(),
            role_ids: Vec::new(),
            badges: badges.iter().map(|badge| (*badge).to_owned()).collect(),
            observed_at: 0,
        }
    }

    #[test]
    fn full_snapshot_excludes_members_without_badges() {
        let members = public_members(
            &[],
            vec![record("Player", &[]), record("Roster", &["head"])],
        );

        assert_eq!(members.len(), 1);
        assert_eq!(members[0].nickname, "Roster");
    }

    #[test]
    fn subscribed_member_without_badges_is_not_found() {
        let members = public_members(&["player".into()], vec![record("Player", &[])]);

        assert_eq!(members.len(), 1);
        assert!(matches!(members[0].status, PublicMemberStatus::NotFound));
        assert!(members[0].badges.is_empty());
    }
}
