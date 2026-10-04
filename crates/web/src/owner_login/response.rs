use application::identity::certificate_login::CertificateLoginChallenge;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct StartResponse {
    challenge_token: String,
    statement_base64: String,
    expires_in_seconds: u64,
}

impl From<CertificateLoginChallenge> for StartResponse {
    fn from(value: CertificateLoginChallenge) -> Self {
        Self {
            challenge_token: value.challenge_token,
            statement_base64: STANDARD.encode(value.statement.canonical_bytes()),
            expires_in_seconds: value.expires_in_seconds,
        }
    }
}
