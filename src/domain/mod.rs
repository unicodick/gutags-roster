mod badges;
mod errors;
mod models;
mod nicknames;

pub use badges::derive_badges;
pub use errors::DomainError;
pub use models::{BadgeRule, MemberOverride, MemberRecord, RawMember};
pub use nicknames::{MAX_NICKNAMES, normalize_nickname, normalize_nicknames};

pub fn build_member_record(
    raw: RawMember,
    rules: &[BadgeRule],
    observed_at: i64,
) -> Result<MemberRecord, DomainError> {
    let discord_id = raw.discord_id.trim().to_owned();
    if discord_id.is_empty() {
        return Err(DomainError::EmptyDiscordId);
    }

    let nickname_key = normalize_nickname(&raw.nickname)?;
    let badges = derive_badges(&raw.role_ids, rules);

    Ok(MemberRecord {
        discord_id,
        nickname_raw: raw.nickname.trim().to_owned(),
        nickname_key,
        role_ids: raw.role_ids,
        badges,
        observed_at,
    })
}
