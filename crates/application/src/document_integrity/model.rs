use super::{DocumentIntegrityIncidentId, DocumentIntegrityObservationId};
use crate::documents::DocumentRecord;
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{DocumentVersionRef, Sha256Digest},
    identity::UserId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DocumentIntegrityFailure {
    #[error("stored vault structure was rejected")]
    MalformedVault,
    #[error("stored content authentication was rejected")]
    AuthenticationFailed,
    #[error("stored content digest differs from its reference")]
    DigestMismatch,
    #[error("stored content snapshot changed before authorization")]
    SnapshotChanged,
}

impl DocumentIntegrityFailure {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MalformedVault => "malformed_vault",
            Self::AuthenticationFailed => "authentication_failed",
            Self::DigestMismatch => "digest_mismatch",
            Self::SnapshotChanged => "snapshot_changed",
        }
    }
}

/// Internal observation from an authorized exact read, never a caller-submitted report.
/// The encrypted snapshot is transient and must not be logged or stored in the incident.
#[derive(Clone)]
pub struct DocumentIntegrityObservation {
    pub observation_id: DocumentIntegrityObservationId,
    pub requester: UserId,
    pub case_id: CaseId,
    pub record: DocumentRecord,
    pub failure: DocumentIntegrityFailure,
    pub detected_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentIntegrityReceipt {
    pub incident_id: DocumentIntegrityIncidentId,
    pub observation_id: DocumentIntegrityObservationId,
    pub recorded_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentIntegrityIncident {
    pub id: DocumentIntegrityIncidentId,
    pub observation_id: DocumentIntegrityObservationId,
    pub case_id: CaseId,
    pub reference: DocumentVersionRef,
    pub requester: UserId,
    pub failure: DocumentIntegrityFailure,
    pub detected_at: OffsetDateTime,
    pub recorded_at: OffsetDateTime,
    pub expected_digest: Sha256Digest,
    pub observed_snapshot_digest: Sha256Digest,
}
