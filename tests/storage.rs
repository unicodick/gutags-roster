use gytags_roster::domain::MemberRecord;
use gytags_roster::storage::{Repository, age_seconds};

fn member(discord_id: &str, nickname: &str) -> MemberRecord {
    MemberRecord {
        discord_id: discord_id.into(),
        nickname_raw: nickname.into(),
        nickname_key: nickname.to_lowercase(),
        role_ids: vec!["role".into()],
        badges: vec!["staff".into()],
        observed_at: 1,
    }
}

#[tokio::test]
async fn stores_and_reads_snapshot() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    let member = member("1", "Player");

    assert_eq!(
        repository.replace_snapshot(&[member], 100).await.unwrap(),
        1
    );
    let result = repository
        .members_by_keys(&["player".into()])
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].badges, vec!["staff"]);
    assert_eq!(
        repository.status().await.unwrap().last_source_sync_at,
        Some(100)
    );
}

#[tokio::test]
async fn rolls_back_snapshot_revision_and_freshness_together() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    repository
        .replace_snapshot(&[member("1", "Original")], 100)
        .await
        .unwrap();

    let result = repository
        .replace_snapshot(&[member("2", "First"), member("2", "Duplicate")], 200)
        .await;

    assert!(result.is_err());
    let status = repository.status().await.unwrap();
    assert_eq!(status.revision, 1);
    assert_eq!(status.last_source_sync_at, Some(100));
    let records = repository.members_by_keys(&[]).await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].nickname_raw, "Original");
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
