use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("nickname cannot be empty")]
    EmptyNickname,
    #[error("too many nicknames; maximum is {max}")]
    TooManyNicknames { max: usize },
    #[error("discord id cannot be empty")]
    EmptyDiscordId,
    #[error("duplicate discord id in snapshot: {0}")]
    DuplicateDiscordId(String),
}
