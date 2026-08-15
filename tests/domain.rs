use gytags_roster::domain::{
    BadgeRule, DomainError, derive_badges, normalize_nickname, normalize_nicknames,
};

#[test]
fn normalizes_nickname_without_merging_symbols() {
    assert_eq!(
        normalize_nickname("  Player_Name  ").unwrap(),
        "player_name"
    );
    assert_eq!(normalize_nickname("Player-Name").unwrap(), "player-name");
}

#[test]
fn derives_priority_ordered_unique_badges() {
    let rules = vec![
        BadgeRule {
            role_id: "r1".into(),
            badge_id: "builder".into(),
            priority: 10,
        },
        BadgeRule {
            role_id: "r2".into(),
            badge_id: "staff".into(),
            priority: 100,
        },
        BadgeRule {
            role_id: "r3".into(),
            badge_id: "staff".into(),
            priority: 1,
        },
    ];
    let roles = vec!["r1".into(), "r2".into(), "r3".into()];

    assert_eq!(derive_badges(&roles, &rules), vec!["staff", "builder"]);
}

#[test]
fn normalizes_and_deduplicates_requested_nicknames() {
    assert_eq!(
        normalize_nicknames(vec![" Player ".into(), "PLAYER".into(), "Other".into()]).unwrap(),
        vec!["player", "other"]
    );
}

#[test]
fn rejects_empty_discord_ids() {
    let error = gytags_roster::domain::build_member_record(
        gytags_roster::domain::RawMember {
            discord_id: "  ".into(),
            nickname: "Player".into(),
            role_ids: Vec::new(),
        },
        &[],
        0,
    )
    .unwrap_err();

    assert!(matches!(error, DomainError::EmptyDiscordId));
}
