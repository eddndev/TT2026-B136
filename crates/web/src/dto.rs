//! JSON response types for the document API.

use application::documents::DocumentSummary;
use application::verification::{ComponentReport, ComponentStatus, Verdict, VerificationReport};
use domain::audit::ChainVerification;
use serde::Serialize;

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
