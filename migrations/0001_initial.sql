CREATE TABLE IF NOT EXISTS members (
    discord_id TEXT PRIMARY KEY NOT NULL,
    nickname_raw TEXT NOT NULL,
    nickname_key TEXT NOT NULL,
    role_ids_json TEXT NOT NULL,
    badges_json TEXT NOT NULL,
    observed_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_members_nickname_key
    ON members (nickname_key);

CREATE TABLE IF NOT EXISTS system_state (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO system_state (key, value) VALUES ('revision', '0');
INSERT OR IGNORE INTO system_state (key, value) VALUES ('last_source_sync_at', '');
