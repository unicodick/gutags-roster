pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn age_seconds(last_seen_at: Option<i64>, now: i64) -> Option<i64> {
    last_seen_at.map(|timestamp| now.saturating_sub(timestamp).max(0))
}
