use crate::domain::RawMember;
use crate::protocol::{IngestRequest, IngestResponse};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct CollectorClientConfig {
    pub base_url: String,
    pub ingest_token: Option<String>,
}

#[derive(Debug, Error)]
pub enum CollectorClientError {
    #[error("invalid backend URL: {0}")]
    InvalidUrl(String),
    #[error("backend request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("backend returned HTTP {status}: {body}")]
    ApiStatus { status: u16, body: String },
    #[error("backend request failed after {attempts} attempts: {last_error}")]
    RetriesExhausted { attempts: usize, last_error: String },
}

#[derive(Clone)]
pub struct CollectorClient {
    client: Client,
    base_url: Url,
    ingest_token: Option<String>,
}

impl CollectorClient {
    pub fn new(config: CollectorClientConfig) -> Result<Self, CollectorClientError> {
        let base_url = Url::parse(config.base_url.trim_end_matches('/'))
            .map_err(|error| CollectorClientError::InvalidUrl(error.to_string()))?;
        let client = Client::builder().timeout(Duration::from_secs(15)).build()?;
        Ok(Self {
            client,
            base_url,
            ingest_token: config.ingest_token,
        })
    }

    pub async fn snapshot(
        &self,
        members: Vec<RawMember>,
    ) -> Result<IngestResponse, CollectorClientError> {
        self.post_json("/internal/v1/ingest", IngestRequest::Snapshot { members })
            .await
    }

    async fn post_json<T, R>(&self, path: &str, payload: T) -> Result<R, CollectorClientError>
    where
        T: serde::Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let url = self
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|error| CollectorClientError::InvalidUrl(error.to_string()))?;
        const MAX_ATTEMPTS: usize = 3;
        const INITIAL_BACKOFF: Duration = Duration::from_millis(250);
        const MAX_BACKOFF: Duration = Duration::from_secs(5);
        let attempts = MAX_ATTEMPTS;
        let mut backoff = INITIAL_BACKOFF;
        let mut last_error = None;

        for attempt in 1..=attempts {
            let mut request = self.client.post(url.clone()).json(&payload);
            if let Some(token) = &self.ingest_token {
                request = request.header("x-gytags-ingest-token", token);
            }

            match request.send().await {
                Ok(response) if response.status().is_success() => {
                    return Ok(response.json::<R>().await?);
                }
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    if !is_retryable_status(status) {
                        return Err(CollectorClientError::ApiStatus {
                            status: status.as_u16(),
                            body,
                        });
                    }
                    last_error = Some(format!("HTTP {}: {}", status.as_u16(), body));
                }
                Err(error) => {
                    last_error = Some(error.to_string());
                }
            }

            if attempt < attempts {
                tokio::time::sleep(backoff).await;
                backoff = backoff.saturating_mul(2).min(MAX_BACKOFF);
            }
        }

        Err(CollectorClientError::RetriesExhausted {
            attempts,
            last_error: last_error.unwrap_or_else(|| "unknown error".into()),
        })
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}
