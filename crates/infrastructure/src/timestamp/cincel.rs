//! HTTP adapter for the Cincel timestamping provider's sandbox.
//!
//! # Assumed HTTP contract
//!
//! This is the adapter's own contract, subject to confirmation against
//! the provider's sandbox. It is exercised as written by the stub
//! server in `tests/cincel_stub.rs`.
//!
//! - Submit: `POST {base_url}/api/v1/timestamps` with the api key in an
//!   `X-Api-Key` header and the JSON body
//!   `{"digest_algorithm":"sha-256","digest_hex":"<64 hex chars>"}`.
//! - A `2xx` response body is JSON with a `status` field:
//!   `{"status":"completed","token_base64":"..."}` carries the DER
//!   token; `{"status":"processing","id":"..."}` means the provider is
//!   still producing it; `{"status":"rejected","detail":"..."}` is a
//!   refusal.
//! - While processing, the adapter polls
//!   `GET {base_url}/api/v1/timestamps/{id}` with the same header,
//!   sleeping the retry delay before each poll, up to the configured
//!   number of polls.
//! - Any non-`2xx` status is a provider rejection.
//!
//! The api key is held in a zeroizing buffer, travels only in the
//! request header, and is excluded from the adapter's Debug form and
//! from every error message.

use std::fmt;
use std::thread;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use domain::crypto::timestamp::TimestampService;
use domain::crypto::Sha256Digest;
use domain::DomainError;
use zeroize::Zeroizing;

use crate::error::TsaError;

/// Header carrying the api key on every request.
const API_KEY_HEADER: &str = "X-Api-Key";

/// Longest response-body fragment copied into an error message.
const BODY_LIMIT: usize = 300;

/// How submissions that come back as processing are polled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of polls after the initial submission.
    pub max_attempts: u32,
    /// Fixed pause before each poll.
    pub delay: Duration,
}

/// [`TimestampService`] over the Cincel provider's HTTP interface.
pub struct CincelTsaAdapter {
    base_url: String,
    api_key: Zeroizing<String>,
    client: reqwest::blocking::Client,
    retry: RetryPolicy,
}

/// The api key must never leak through logs, so the Debug form redacts
/// it.
impl fmt::Debug for CincelTsaAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CincelTsaAdapter")
            .field("base_url", &self.base_url)
            .field("api_key", &"<redacted>")
            .field("retry", &self.retry)
            .finish_non_exhaustive()
    }
}

impl CincelTsaAdapter {
    /// Creates the adapter over the provider's base url, the api key,
    /// a per-request timeout, and the polling policy for deferred
    /// tokens.
    pub fn new(
        base_url: impl Into<String>,
        api_key: Zeroizing<String>,
        timeout: Duration,
        retry: RetryPolicy,
    ) -> Result<Self, TsaError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|err| TsaError::Unreachable(format!("cannot build the http client: {err}")))?;
        let mut base_url = base_url.into();
        while base_url.ends_with('/') {
            base_url.pop();
        }
        Ok(Self {
            base_url,
            api_key,
            client,
            retry,
        })
    }

    fn obtain(&self, digest: &Sha256Digest) -> Result<Vec<u8>, TsaError> {
        let submit_url = format!("{}/api/v1/timestamps", self.base_url);
        let body = serde_json::json!({
            "digest_algorithm": "sha-256",
            "digest_hex": digest.to_hex(),
        });
        let response = self
            .client
            .post(&submit_url)
            .header(API_KEY_HEADER, self.api_key.as_str())
            .json(&body)
            .send()
            .map_err(as_unreachable)?;
        let mut state = interpret(response)?;

        let mut polls = 0u32;
        loop {
            match state {
                ProviderState::Token(bytes) => return Ok(bytes),
                ProviderState::Processing(id) => {
                    if polls >= self.retry.max_attempts {
                        return Err(TsaError::Rejected(format!(
                            "the token was still processing after {polls} polls"
                        )));
                    }
                    polls += 1;
                    thread::sleep(self.retry.delay);
                    let poll_url = format!("{}/api/v1/timestamps/{id}", self.base_url);
                    let response = self
                        .client
                        .get(&poll_url)
                        .header(API_KEY_HEADER, self.api_key.as_str())
                        .send()
                        .map_err(as_unreachable)?;
                    state = interpret(response)?;
                }
            }
        }
    }
}

impl TimestampService for CincelTsaAdapter {
    fn request(&self, digest: &Sha256Digest) -> Result<Vec<u8>, DomainError> {
        Ok(self.obtain(digest)?)
    }
}

/// What one provider response means for the submission.
enum ProviderState {
    /// The token is ready.
    Token(Vec<u8>),
    /// The provider is still producing the token addressed by this id.
    Processing(String),
}

fn interpret(response: reqwest::blocking::Response) -> Result<ProviderState, TsaError> {
    let status = response.status();
    let body = response.text().map_err(as_unreachable)?;
    if !status.is_success() {
        return Err(TsaError::Rejected(format!(
            "the provider returned status {status}: {}",
            body_fragment(&body)
        )));
    }
    let value: serde_json::Value = serde_json::from_str(&body).map_err(|_| {
        TsaError::InvalidToken(format!(
            "the provider body is not json: {}",
            body_fragment(&body)
        ))
    })?;
    match value.get("status").and_then(serde_json::Value::as_str) {
        Some("completed") => {
            let encoded = value
                .get("token_base64")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    TsaError::InvalidToken("a completed response carries no token".to_string())
                })?;
            let token = BASE64.decode(encoded).map_err(|err| {
                TsaError::InvalidToken(format!("the token is not valid base64: {err}"))
            })?;
            if token.is_empty() {
                return Err(TsaError::InvalidToken(
                    "the decoded token is empty".to_string(),
                ));
            }
            Ok(ProviderState::Token(token))
        }
        Some("processing") => {
            let id = value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    TsaError::InvalidToken("a processing response carries no id".to_string())
                })?;
            Ok(ProviderState::Processing(id.to_string()))
        }
        Some("rejected") => {
            let detail = value
                .get("detail")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("no detail given");
            Err(TsaError::Rejected(detail.to_string()))
        }
        other => Err(TsaError::InvalidToken(format!(
            "unknown provider status: {other:?}"
        ))),
    }
}

/// Transport failures carry no response; reqwest's message names the
/// url and cause but never the request headers, so the api key cannot
/// appear here.
fn as_unreachable(err: reqwest::Error) -> TsaError {
    TsaError::Unreachable(err.to_string())
}

fn body_fragment(body: &str) -> String {
    body.trim().chars().take(BODY_LIMIT).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 1,
            delay: Duration::from_millis(1),
        }
    }

    #[test]
    fn the_debug_form_redacts_the_api_key() {
        let adapter = CincelTsaAdapter::new(
            "http://127.0.0.1:1",
            Zeroizing::new("clave-super-secreta".to_string()),
            Duration::from_secs(1),
            policy(),
        )
        .unwrap();
        let debug = format!("{adapter:?}");
        assert!(!debug.contains("clave-super-secreta"), "got: {debug}");
        assert!(debug.contains("<redacted>"), "got: {debug}");
    }

    #[test]
    fn trailing_slashes_in_the_base_url_are_trimmed() {
        let adapter = CincelTsaAdapter::new(
            "http://127.0.0.1:1//",
            Zeroizing::new("k".to_string()),
            Duration::from_secs(1),
            policy(),
        )
        .unwrap();
        assert_eq!(adapter.base_url, "http://127.0.0.1:1");
    }
}
