use gytags_roster::collector::{CollectorError, SnapshotSync};
use gytags_roster::domain::{MemberOverride, RawMember};
use gytags_roster::storage::Repository;
use tokio::sync::broadcast;

fn member(discord_id: &str, nickname: &str, role_id: &str) -> RawMember {
    RawMember {
        discord_id: discord_id.into(),
        nickname: nickname.into(),
        role_ids: vec![role_id.into()],
    }
}

#[tokio::test]
async fn skips_unchanged_snapshots_and_reports_revision() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    let (events, mut receiver) = broadcast::channel(8);
    let sync = SnapshotSync::new(repository, Vec::new(), Vec::new(), events);

    assert_eq!(
        sync.apply(vec![member("1", "Player", "staff")])
            .await
            .unwrap(),
        1
    );
    let first = receiver.recv().await.unwrap();
    assert_eq!(first.revision, 1);

    assert_eq!(
        sync.apply(vec![member("1", "Player", "staff")])
            .await
            .unwrap(),
        1
    );
    assert!(receiver.try_recv().is_err());

    assert_eq!(
        sync.apply(vec![member("1", "Renamed", "staff")])
            .await
            .unwrap(),
        2
    );
    assert_eq!(receiver.recv().await.unwrap().revision, 2);
}

#[tokio::test]
async fn rejects_duplicate_discord_ids_and_detects_raw_nickname_changes() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    let (events, mut receiver) = broadcast::channel(8);
    let sync = SnapshotSync::new(repository, Vec::new(), Vec::new(), events);

    let error = sync
        .apply(vec![
            member("1", "Player", "staff"),
            member("1", "Other", "staff"),
        ])
        .await
        .unwrap_err();
    assert!(error.to_string().contains("duplicate discord id"));

    sync.apply(vec![member("1", "Player", "staff")])
        .await
        .unwrap();
    receiver.recv().await.unwrap();

    sync.apply(vec![member("1", "PLAYER", "staff")])
        .await
        .unwrap();
    assert_eq!(receiver.recv().await.unwrap().revision, 2);
}

#[tokio::test]
async fn applies_admin_overrides_to_discord_snapshots() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    let (events, _) = broadcast::channel(8);
    let sync = SnapshotSync::new(
        repository.clone(),
        Vec::new(),
        vec![MemberOverride {
            discord_id: "376674641676206080".into(),
            nickname: "Likholesye".into(),
            role_ids: vec!["staff".into()],
            badges: vec!["staff".into()],
        }],
        events,
    );

    sync.apply(vec![member(
        "376674641676206080",
        "OldDiscordName",
        "staff",
    )])
    .await
    .unwrap();

    let records = repository
        .members_by_keys(&["likholesye".into()])
        .await
        .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].discord_id, "376674641676206080");
    assert_eq!(records[0].nickname_raw, "Likholesye");
    assert_eq!(records[0].badges, vec!["staff"]);
}

#[tokio::test]
async fn rejects_snapshot_that_loses_more_than_half_of_roster() {
    let repository = Repository::connect("sqlite::memory:").await.unwrap();
    let (events, mut receiver) = broadcast::channel(8);
    let sync = SnapshotSync::new(repository.clone(), Vec::new(), Vec::new(), events);
    let initial = (0..6)
        .map(|index| member(&index.to_string(), &format!("Player{index}"), "staff"))
        .collect();

    assert_eq!(sync.apply(initial).await.unwrap(), 1);
    receiver.recv().await.unwrap();

    let partial = (0..2)
        .map(|index| member(&index.to_string(), &format!("Player{index}"), "staff"))
        .collect();
    let error = sync.apply(partial).await.unwrap_err();

    assert!(matches!(
        error,
        CollectorError::SnapshotTooSmall {
            actual: 2,
            minimum: 3
        }
    ));
    assert_eq!(repository.status().await.unwrap().revision, 1);
    assert_eq!(repository.members_by_keys(&[]).await.unwrap().len(), 6);
    assert!(receiver.try_recv().is_err());

    let half_snapshot = (0..3)
        .map(|index| member(&index.to_string(), &format!("Player{index}"), "staff"))
        .collect();
    assert_eq!(sync.apply(half_snapshot).await.unwrap(), 2);
    assert_eq!(receiver.recv().await.unwrap().revision, 2);
}
