//! JSON response types for the document API.

use application::documents::DocumentSummary;
use application::identity::{EnrollmentResult, LoginChallenge, Principal, SessionResult};
use application::verification::{ComponentReport, ComponentStatus, Verdict, VerificationReport};
use domain::audit::ChainVerification;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CredentialsRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct ChallengeCodeRequest {
    pub challenge_token: String,
    pub code: String,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct PrincipalResponse {
    id: String,
    email: String,
    role: &'static str,
}

impl From<Principal> for PrincipalResponse {
    fn from(principal: Principal) -> Self {
        Self {
            id: principal.id.to_string(),
            email: principal.email,
            role: principal.role.as_str(),
        }
    }
}

#[derive(Serialize)]
pub struct EnrollmentResponse {
    user: PrincipalResponse,
    totp_secret_base32: String,
    otpauth_uri: String,
    recovery_codes: Vec<String>,
}

impl From<EnrollmentResult> for EnrollmentResponse {
    fn from(result: EnrollmentResult) -> Self {
        Self {
            user: result.principal.into(),
            totp_secret_base32: result.totp_secret_base32.to_string(),
            otpauth_uri: result.otpauth_uri.to_string(),
            recovery_codes: result
                .recovery_codes
                .iter()
                .map(|code| code.as_str().to_owned())
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub struct LoginChallengeResponse {
    challenge_token: String,
    expires_in_seconds: u64,
}

impl From<LoginChallenge> for LoginChallengeResponse {
    fn from(challenge: LoginChallenge) -> Self {
        Self {
            challenge_token: challenge.challenge_token,
            expires_in_seconds: challenge.expires_in_seconds,
        }
    }
}

#[derive(Serialize)]
pub struct SessionResponse {
    access_token: String,
    token_type: &'static str,
    expires_in_seconds: u64,
    user: PrincipalResponse,
}

impl From<SessionResult> for SessionResponse {
    fn from(session: SessionResult) -> Self {
        Self {
            access_token: session.access_token,
            token_type: "Bearer",
            expires_in_seconds: session.expires_in_seconds,
            user: session.principal.into(),
        }
    }
}

#[derive(Serialize)]
pub struct DocumentResponse {
    id: String,
    version: u32,
    name: String,
    digest: String,
    sealed: bool,
}

impl From<DocumentSummary> for DocumentResponse {
    fn from(summary: DocumentSummary) -> Self {
        Self {
            id: summary.id.to_string(),
            version: summary.version.get(),
            name: summary.name,
            digest: summary.digest_hex,
            sealed: summary.sealed,
        }
    }
}

#[derive(Serialize)]
pub struct VerificationResponse {
    document_digest: String,
    integrity: ComponentResponse,
    signature: ComponentResponse,
    certificate: ComponentResponse,
    timestamp: ComponentResponse,
    verdict: &'static str,
}

#[derive(Serialize)]
struct ComponentResponse {
    status: &'static str,
    detail: String,
}

impl From<ComponentReport> for ComponentResponse {
    fn from(report: ComponentReport) -> Self {
        let status = match report.status {
            ComponentStatus::Passed => "passed",
            ComponentStatus::Failed => "failed",
            ComponentStatus::Skipped => "skipped",
        };
        Self {
            status,
            detail: report.detail,
        }
    }
}

impl From<VerificationReport> for VerificationResponse {
    fn from(report: VerificationReport) -> Self {
        let verdict = match report.verdict {
            Verdict::Valid => "valid",
            Verdict::NotValid => "not_valid",
        };
        Self {
            document_digest: report.document_digest_hex,
            integrity: report.integrity.into(),
            signature: report.signature.into(),
            certificate: report.certificate.into(),
            timestamp: report.timestamp.into(),
            verdict,
        }
    }
}

#[derive(Serialize)]
pub struct AuditResponse {
    valid: bool,
    entries: Option<usize>,
    first_broken_index: Option<usize>,
}

impl From<ChainVerification> for AuditResponse {
    fn from(outcome: ChainVerification) -> Self {
        match outcome {
            ChainVerification::Valid { entries } => Self {
                valid: true,
                entries: Some(entries),
                first_broken_index: None,
            },
            ChainVerification::Broken { first_broken_index } => Self {
                valid: false,
                entries: None,
                first_broken_index: Some(first_broken_index),
            },
        }
    }
}
