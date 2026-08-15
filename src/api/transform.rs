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

    PublicMember {
        nickname: first.nickname_raw.clone(),
        status: PublicMemberStatus::Ok,
        badges: first.badges.clone(),
    }
}
