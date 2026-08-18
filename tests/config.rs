use gutags_roster::config::{ConfigError, load_badge_rules, load_member_overrides};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_rules_path() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "gutags-badges-{}-{suffix}.json",
        std::process::id()
    ))
}

fn temp_overrides_path() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "gutags-overrides-{}-{suffix}.json",
        std::process::id()
    ))
}

#[test]
fn rejects_badge_rules_without_role_or_badge_ids() {
    let path = temp_rules_path();
    fs::write(
        &path,
        r#"{"rules":[{"role_id":"","badge_id":"staff","group":"career","priority":1}]}"#,
    )
    .unwrap();

    let result = load_badge_rules(&path);
    fs::remove_file(path).unwrap();

    assert!(matches!(
        result,
        Err(ConfigError::InvalidBadgeRule { index: 0, .. })
    ));
}

#[test]
fn rejects_unknown_badge_rule_fields() {
    let path = temp_rules_path();
    fs::write(
        &path,
        r#"{"rules":[{"role_id":"role","badge_id":"staff","group":"career","priority":1,"unexpected":true}]}"#,
    )
    .unwrap();

    let result = load_badge_rules(&path);
    fs::remove_file(path).unwrap();

    assert!(matches!(result, Err(ConfigError::Json(_))));
}

#[test]
fn loads_member_overrides_from_config() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/overrides.json");
    let members = load_member_overrides(path).unwrap();

    assert_eq!(members.len(), 2);
    assert_eq!(members[0].nickname, "Likholesye");
    assert_eq!(members[1].discord_id, "959458266713321482");
}

#[test]
fn rejects_duplicate_member_override_ids() {
    let path = temp_overrides_path();
    fs::write(
        &path,
        r#"{"members":[
            {"discord_id":"1","nickname":"First"},
            {"discord_id":"1","nickname":"Second"}
        ]}"#,
    )
    .unwrap();

    let result = load_member_overrides(&path);
    fs::remove_file(path).unwrap();

    assert!(matches!(
        result,
        Err(ConfigError::InvalidMemberOverride { index: 1, .. })
    ));
}

#[test]
fn rejects_role_and_badge_override_fields() {
    let path = temp_overrides_path();
    fs::write(
        &path,
        r#"{"members":[{"discord_id":"1","nickname":"Player","role_ids":["role"],"badges":["head"]}]}"#,
    )
    .unwrap();

    let result = load_member_overrides(&path);
    fs::remove_file(path).unwrap();

    assert!(matches!(result, Err(ConfigError::Json(_))));
}
