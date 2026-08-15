use super::DomainError;
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

pub const MAX_NICKNAMES: usize = 256;

pub fn normalize_nickname(value: &str) -> Result<String, DomainError> {
    let normalized: String = value.trim().nfc().collect();
    if normalized.is_empty() {
        return Err(DomainError::EmptyNickname);
    }

    Ok(normalized.to_lowercase())
}

pub fn normalize_nicknames(nicknames: Vec<String>) -> Result<Vec<String>, DomainError> {
    if nicknames.len() > MAX_NICKNAMES {
        return Err(DomainError::TooManyNicknames { max: MAX_NICKNAMES });
    }

    let mut unique = HashSet::with_capacity(nicknames.len());
    let mut result = Vec::with_capacity(nicknames.len());
    for nickname in nicknames {
        let key = normalize_nickname(&nickname)?;
        if unique.insert(key.clone()) {
            result.push(key);
        }
    }
    Ok(result)
}
