use super::StorageError;
use crate::domain::MemberRecord;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;

const REVISION_KEY: &str = "revision";
const LAST_SOURCE_SYNC_AT_KEY: &str = "last_source_sync_at";

#[derive(Debug, Clone)]
pub struct Repository {
    pool: SqlitePool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemStatus {
    pub revision: i64,
    pub last_source_sync_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberOverride {
    pub discord_id: String,
    pub nickname_raw: String,
    pub role_ids: Vec<String>,
    pub badges: Vec<String>,
}

impl SystemStatus {
    pub fn effective_source_status(&self, ttl_seconds: u64, now: i64) -> String {
        match self.last_source_sync_at {
            None => "empty".into(),
            Some(last) if now.saturating_sub(last) > ttl_seconds as i64 => "stale".into(),
            Some(_) => "fresh".into(),
        }
    }
}

#[derive(Debug, FromRow)]
struct MemberRow {
    discord_id: String,
    nickname_raw: String,
    nickname_key: String,
    role_ids_json: String,
    badges_json: String,
    observed_at: i64,
}

#[derive(Debug, FromRow)]
struct MemberOverrideRow {
    discord_id: String,
    nickname_raw: String,
    role_ids_json: String,
    badges_json: String,
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

impl Repository {
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|error| StorageError::InvalidDatabaseUrl(error.to_string()))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn replace_snapshot(&self, members: &[MemberRecord]) -> Result<i64, StorageError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query("DELETE FROM members")
            .execute(&mut *transaction)
            .await?;

        for member in members {
            sqlx::query(
                "INSERT INTO members
                 (discord_id, nickname_raw, nickname_key, role_ids_json, badges_json, observed_at)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(&member.discord_id)
            .bind(&member.nickname_raw)
            .bind(&member.nickname_key)
            .bind(serde_json::to_string(&member.role_ids)?)
            .bind(serde_json::to_string(&member.badges)?)
            .bind(member.observed_at)
            .execute(&mut *transaction)
            .await?;
        }

        let revision = next_revision(&mut transaction).await?;
        transaction.commit().await?;
        Ok(revision)
    }

    pub async fn record_sync(&self, timestamp: i64) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        set_state(
            &mut transaction,
            LAST_SOURCE_SYNC_AT_KEY,
            &timestamp.to_string(),
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn status(&self) -> Result<SystemStatus, StorageError> {
        let revision =
            sqlx::query_scalar::<_, String>("SELECT value FROM system_state WHERE key = ?")
                .bind(REVISION_KEY)
                .fetch_one(&self.pool)
                .await?
                .parse::<i64>()
                .map_err(|error| StorageError::InvalidRevision(error.to_string()))?;

        let last_source_sync_at =
            sqlx::query_scalar::<_, String>("SELECT value FROM system_state WHERE key = ?")
                .bind(LAST_SOURCE_SYNC_AT_KEY)
                .fetch_optional(&self.pool)
                .await?
                .filter(|value| !value.is_empty())
                .map(|value| {
                    value
                        .parse::<i64>()
                        .map_err(|_| StorageError::InvalidTimestamp(value))
                })
                .transpose()?;

        Ok(SystemStatus {
            revision,
            last_source_sync_at,
        })
    }

    pub async fn members_by_keys(
        &self,
        nickname_keys: &[String],
    ) -> Result<Vec<MemberRecord>, StorageError> {
        let rows = if nickname_keys.is_empty() {
            sqlx::query_as::<_, MemberRow>(
                "SELECT discord_id, nickname_raw, nickname_key, role_ids_json, badges_json, observed_at
                 FROM members ORDER BY nickname_key, discord_id",
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            let placeholders = std::iter::repeat_n("?", nickname_keys.len())
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "SELECT discord_id, nickname_raw, nickname_key, role_ids_json, badges_json, observed_at
                 FROM members WHERE nickname_key IN ({placeholders}) ORDER BY nickname_key, discord_id"
            );
            let mut query = sqlx::query_as::<_, MemberRow>(&query);
            for key in nickname_keys {
                query = query.bind(key);
            }
            query.fetch_all(&self.pool).await?
        };

        rows.into_iter()
            .map(|row| {
                Ok(MemberRecord {
                    discord_id: row.discord_id,
                    nickname_raw: row.nickname_raw,
                    nickname_key: row.nickname_key,
                    role_ids: serde_json::from_str(&row.role_ids_json)?,
                    badges: serde_json::from_str(&row.badges_json)?,
                    observed_at: row.observed_at,
                })
            })
            .collect()
    }

    pub async fn member_overrides(&self) -> Result<Vec<MemberOverride>, StorageError> {
        let rows = sqlx::query_as::<_, MemberOverrideRow>(
            "SELECT discord_id, nickname_raw, role_ids_json, badges_json
             FROM member_overrides ORDER BY discord_id",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(MemberOverride {
                    discord_id: row.discord_id,
                    nickname_raw: row.nickname_raw,
                    role_ids: serde_json::from_str(&row.role_ids_json)?,
                    badges: serde_json::from_str(&row.badges_json)?,
                })
            })
            .collect()
    }
}

async fn set_state(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    key: &str,
    value: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO system_state (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn next_revision(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<i64, StorageError> {
    let current = sqlx::query_scalar::<_, String>("SELECT value FROM system_state WHERE key = ?")
        .bind(REVISION_KEY)
        .fetch_one(&mut **transaction)
        .await?;
    let revision = current
        .parse::<i64>()
        .map_err(|_| StorageError::InvalidRevision(current.clone()))?
        + 1;

    sqlx::query(
        "INSERT INTO system_state (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(REVISION_KEY)
    .bind(revision.to_string())
    .execute(&mut **transaction)
    .await?;

    Ok(revision)
}
