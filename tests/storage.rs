use gytags_roster::domain::MemberRecord;
use gytags_roster::storage::{Repository, age_seconds};

#[tokio::test]
async fn stores_and_reads_snapshot() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    let member = MemberRecord {
        discord_id: "1".into(),
        nickname_raw: "Player".into(),
        nickname_key: "player".into(),
        role_ids: vec!["role".into()],
        badges: vec!["staff".into()],
        observed_at: 1,
    };

    assert_eq!(repository.replace_snapshot(&[member]).await.unwrap(), 1);
    let result = repository
        .members_by_keys(&["player".into()])
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].badges, vec!["staff"]);
}

#[tokio::test]
async fn reports_stale_after_source_ttl() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    repository.record_sync(100).await.unwrap();
    let status = repository.status().await.unwrap();

    assert_eq!(age_seconds(status.last_source_sync_at, 105), Some(5));
    assert_eq!(status.effective_source_status(10, 105), "fresh");
    assert_eq!(status.effective_source_status(10, 111), "stale");
}

#[tokio::test]
async fn reads_admin_member_overrides_from_migration() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    let overrides = repository.member_overrides().await.unwrap();

    assert_eq!(overrides.len(), 3);
    assert_eq!(overrides[0].nickname_raw, "Likholesye");
    assert_eq!(overrides[0].badges, vec!["staff"]);
    assert_eq!(overrides[2].nickname_raw, "TBEPDblHYA");
    assert_eq!(overrides[2].badges, vec!["yrod"]);
}
