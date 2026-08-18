use gutags_roster::domain::{
    BadgeGroup, BadgeRule, DomainError, derive_badges, normalize_nickname, normalize_nicknames,
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
fn derives_highest_career_and_team_badges() {
    let rules = vec![
        BadgeRule {
            role_id: "career_low".into(),
            badge_id: "academ".into(),
            group: BadgeGroup::Career,
            priority: 8,
        },
        BadgeRule {
            role_id: "career_high".into(),
            badge_id: "head".into(),
            group: BadgeGroup::Career,
            priority: 1,
        },
        BadgeRule {
            role_id: "team_low".into(),
            badge_id: "team_5".into(),
            group: BadgeGroup::Team,
            priority: 5,
        },
        BadgeRule {
            role_id: "team_high".into(),
            badge_id: "team_1".into(),
            group: BadgeGroup::Team,
            priority: 1,
        },
    ];
    let roles = vec![
        "career_low".into(),
        "career_high".into(),
        "team_low".into(),
        "team_high".into(),
    ];

    assert_eq!(derive_badges(&roles, &rules), vec!["head", "team_1"]);
}

#[test]
fn derives_only_the_available_badge_groups() {
    let rules = vec![BadgeRule {
        role_id: "team".into(),
        badge_id: "team_3".into(),
        group: BadgeGroup::Team,
        priority: 3,
    }];

    assert_eq!(derive_badges(&["team".into()], &rules), vec!["team_3"]);
    assert!(derive_badges(&[], &rules).is_empty());
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
    let error = gutags_roster::domain::build_member_record(
        gutags_roster::domain::RawMember {
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
