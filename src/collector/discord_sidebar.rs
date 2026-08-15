use super::CollectorError;
use crate::domain::RawMember;
use serde_json::Value;
use std::collections::HashMap;

pub const MEMBERS_PER_RANGE: usize = 100;
pub const RANGES_PER_REQUEST: usize = 3;

#[derive(Debug, Default)]
pub struct MemberListUpdate {
    pub member_count: Option<usize>,
    pub sync_ranges: Vec<(u64, u64)>,
    pub members: Vec<RawMember>,
}

pub fn plan_ranges(member_count: usize) -> Vec<(u64, u64)> {
    if member_count == 0 {
        return Vec::new();
    }

    (0..member_count)
        .step_by(MEMBERS_PER_RANGE)
        .map(|start| {
            (
                start as u64,
                (start + MEMBERS_PER_RANGE - 1).min(member_count - 1) as u64,
            )
        })
        .collect()
}

pub fn parse_update(event: &Value, guild_id: &str) -> Result<MemberListUpdate, CollectorError> {
    let data = event
        .get("d")
        .ok_or_else(|| CollectorError::GatewayProtocol("member list event has no data".into()))?;
    let event_guild_id = data
        .get("guild_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CollectorError::GatewayProtocol("member list event has no guild_id".into())
        })?;
    if event_guild_id != guild_id {
        return Ok(MemberListUpdate::default());
    }

    let member_count = data
        .get("member_count")
        .and_then(Value::as_u64)
        .map(|value| value as usize);
    let mut update = MemberListUpdate {
        member_count,
        ..Default::default()
    };
    let mut members = HashMap::new();

    for operation in data
        .get("ops")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if operation.get("op").and_then(Value::as_str) != Some("SYNC") {
            continue;
        }

        if let Some(range) = parse_range(operation.get("range")) {
            update.sync_ranges.push(range);
        }

        for item in operation
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if item.get("group").is_some() {
                continue;
            }
            let Some(member) = item.get("member") else {
                continue;
            };
            let Some(user) = member.get("user") else {
                continue;
            };
            let Some(discord_id) = user.get("id").and_then(Value::as_str) else {
                continue;
            };
            let Some(nickname) = member.get("nick").and_then(Value::as_str) else {
                continue;
            };
            if nickname.trim().is_empty() {
                continue;
            }

            let role_ids = member
                .get("roles")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|role_id| *role_id != guild_id)
                .map(str::to_owned)
                .collect();

            members.insert(
                discord_id.to_owned(),
                RawMember {
                    discord_id: discord_id.to_owned(),
                    nickname: nickname.to_owned(),
                    role_ids,
                },
            );
        }
    }

    update.members = members.into_values().collect();
    Ok(update)
}

fn parse_range(value: Option<&Value>) -> Option<(u64, u64)> {
    let range = value?.as_array()?;
    Some((range.first()?.as_u64()?, range.get(1)?.as_u64()?))
}

#[cfg(test)]
mod tests {
    use super::{MEMBERS_PER_RANGE, parse_update, plan_ranges};
    use serde_json::json;

    #[test]
    fn plans_full_ranges() {
        assert_eq!(plan_ranges(550).len(), 6);
        assert_eq!(plan_ranges(550)[0], (0, 99));
        assert_eq!(plan_ranges(550)[5], (500, 549));
        assert_eq!(plan_ranges(0), Vec::new());
        assert_eq!(MEMBERS_PER_RANGE, 100);
    }

    #[test]
    fn parses_nicknames_roles_and_skips_groups() {
        let event = json!({
            "d": {
                "guild_id": "guild",
                "member_count": 2,
                "ops": [{
                    "op": "SYNC",
                    "range": [0, 99],
                    "items": [
                        {"group": {"id": "role"}},
                        {"member": {"user": {"id": "1"}, "nick": " Player ", "roles": ["guild", "role"]}},
                        {"member": {"user": {"id": "2"}, "nick": null, "roles": []}}
                    ]
                }]
            }
        });

        let update = parse_update(&event, "guild").unwrap();
        assert_eq!(update.member_count, Some(2));
        assert_eq!(update.sync_ranges, vec![(0, 99)]);
        assert_eq!(update.members.len(), 1);
        assert_eq!(update.members[0].role_ids, vec!["role"]);
    }
}
