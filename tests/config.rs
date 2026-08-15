use gytags_roster::config::{ConfigError, load_badge_rules};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_rules_path() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "gytags-badges-{}-{suffix}.json",
        std::process::id()
    ))
}

#[test]
fn rejects_badge_rules_without_role_or_badge_ids() {
    let path = temp_rules_path();
    fs::write(&path, r#"{"rules":[{"role_id":"","badge_id":"staff"}]}"#).unwrap();

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
        r#"{"rules":[{"role_id":"role","badge_id":"staff","unexpected":true}]}"#,
    )
    .unwrap();

    let result = load_badge_rules(&path);
    fs::remove_file(path).unwrap();

    assert!(matches!(result, Err(ConfigError::BadgeRulesJson(_))));
}
