//! Authenticated document access and transactional persistence by case.

use domain::audit::{ChainVerification, ChainedEvent};
use domain::cases::CaseId;
use domain::clock::OffsetDateTime;
use domain::crypto::DocumentId;
use domain::identity::{Permission, UserId};

use super::{DocumentRecord, DocumentSummary, EvidenceExport};
use crate::{verification::VerificationReport, ApplicationError};

/// Document operation checked against the current role and case membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentAction {
    Upload,
    Seal,
    Verify,
    Export,
}

impl DocumentAction {
    pub const fn permission(self) -> Permission {
        match self {
            Self::Upload => Permission::CreateDocument,
            Self::Seal => Permission::SealDocument,
            Self::Verify => Permission::VerifyDocument,
            Self::Export => Permission::ExportEvidence,
        }
    }

    pub const fn audit_action(self) -> &'static str {
        match self {
            Self::Upload => "document.uploaded",
            Self::Seal => "document.sealed",
            Self::Verify => "document.verified",
            Self::Export => "document.evidence_exported",
        }
    }
}

/// Document metadata together with its immutable case association.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseDocumentSummary {
    pub case_id: CaseId,
    pub document: DocumentSummary,
}

/// The online document boundary accepts credentials, never an actor label.
pub trait CaseDocumentWorkflow: Send + Sync {
    fn upload(
        &self,
        token: &str,
        case_id: CaseId,
        name: &str,
        bytes: &[u8],
    ) -> Result<CaseDocumentSummary, ApplicationError>;
    fn seal(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<CaseDocumentSummary, ApplicationError>;
    fn verify(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<VerificationReport, ApplicationError>;
    fn export_evidence(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<EvidenceExport, ApplicationError>;
    fn verify_audit(&self, token: &str) -> Result<ChainVerification, ApplicationError>;
}

/// Checks current database authority and preserves document/case identity.
///
/// Mutations and their audit events commit together. Every method rechecks the
/// actor's active status, role, and case membership. Document reads and commits
/// also require the supplied case to match the stored immutable association.
pub trait CaseDocumentStore: Send + Sync {
    fn check_access(
        &self,
        actor: UserId,
        case_id: CaseId,
        action: DocumentAction,
    ) -> Result<(), ApplicationError>;
    fn load(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: DocumentId,
        action: DocumentAction,
    ) -> Result<DocumentRecord, ApplicationError>;
    /// Inserts prepared ciphertext and its success event in one transaction.
    fn insert(
        &self,
        actor: UserId,
        case_id: CaseId,
        record: DocumentRecord,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError>;
    /// Requires unchanged base content and unsealed current state before commit.
    fn seal(
        &self,
        actor: UserId,
        case_id: CaseId,
        record: DocumentRecord,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError>;
    /// Rechecks an unchanged snapshot and records access before exposing results.
    fn record_access(
        &self,
        actor: UserId,
        case_id: CaseId,
        record: &DocumentRecord,
        action: DocumentAction,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError>;
    /// Requires an active owner and returns the complete ordered audit chain.
    fn audit_entries(&self, actor: UserId) -> Result<Vec<ChainedEvent>, ApplicationError>;
}
