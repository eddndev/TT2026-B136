//! Authenticated document access and transactional persistence by case.

use domain::audit::{ChainVerification, ChainedEvent};
use domain::cases::CaseId;
use domain::clock::OffsetDateTime;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef};
use domain::identity::{Permission, UserId};

use super::{
    CurrentDocumentMetadata, DocumentMetadata, DocumentOverview, DocumentPage, DocumentQuery,
    DocumentRecord, DocumentSummary, EvidenceExport, MetadataPage, MetadataQuery, MetadataRevision,
    VersionPage, VersionQuery, VersionSelection,
};
use crate::{verification::VerificationReport, ApplicationError};

/// Document operation checked against the current role and case membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentAction {
    Classify,
    ReadMetadata,
    MetadataHistory,
    List,
    Read,
    Upload,
    Append,
    History,
    Seal,
    Verify,
    Export,
}

impl DocumentAction {
    pub const fn permission(self) -> Permission {
        match self {
            Self::Classify => Permission::ClassifyDocument,
            Self::ReadMetadata
            | Self::MetadataHistory
            | Self::List
            | Self::Read
            | Self::History => Permission::ReadDocument,
            Self::Upload => Permission::CreateDocument,
            Self::Append => Permission::AppendDocument,
            Self::Seal => Permission::SealDocument,
            Self::Verify => Permission::VerifyDocument,
            Self::Export => Permission::ExportEvidence,
        }
    }

    pub const fn audit_action(self) -> &'static str {
        match self {
            Self::Classify => "document.metadata_changed",
            Self::ReadMetadata => "document.metadata_read",
            Self::MetadataHistory => "document.metadata_history_listed",
            Self::List => "document.listed",
            Self::Read => "document.read",
            Self::Upload => "document.uploaded",
            Self::Append => "document.version_added",
            Self::History => "document.versions_listed",
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
    fn get_metadata(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<CurrentDocumentMetadata, ApplicationError>;
    fn replace_metadata(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
        expected: MetadataRevision,
        metadata: DocumentMetadata,
    ) -> Result<CurrentDocumentMetadata, ApplicationError>;
    fn metadata_history(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
        query: MetadataQuery,
    ) -> Result<MetadataPage, ApplicationError>;
    fn upload_with_metadata(
        &self,
        token: &str,
        case_id: CaseId,
        name: &str,
        bytes: &[u8],
        metadata: DocumentMetadata,
    ) -> Result<DocumentOverview, ApplicationError>;
    fn append(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
        expected_version: DocumentVersion,
        name: &str,
        bytes: &[u8],
    ) -> Result<DocumentOverview, ApplicationError>;
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
        query: VersionQuery,
    ) -> Result<VersionPage, ApplicationError>;
    fn get_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<CaseDocumentSummary, ApplicationError>;
    fn seal_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<CaseDocumentSummary, ApplicationError>;
    fn verify_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<VerificationReport, ApplicationError>;
    fn export_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<EvidenceExport, ApplicationError>;
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: DocumentQuery,
    ) -> Result<DocumentPage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<DocumentOverview, ApplicationError>;
    fn upload(
        &self,
        token: &str,
        case_id: CaseId,
        name: &str,
        bytes: &[u8],
    ) -> Result<DocumentOverview, ApplicationError>;
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
    fn get_overview(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: DocumentId,
        at: OffsetDateTime,
    ) -> Result<DocumentOverview, ApplicationError>;
    fn get_metadata(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: DocumentId,
        at: OffsetDateTime,
    ) -> Result<CurrentDocumentMetadata, ApplicationError>;
    fn replace_metadata(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: DocumentId,
        expected: MetadataRevision,
        metadata: DocumentMetadata,
        at: OffsetDateTime,
    ) -> Result<CurrentDocumentMetadata, ApplicationError>;
    fn metadata_history(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: DocumentId,
        query: MetadataQuery,
        at: OffsetDateTime,
    ) -> Result<MetadataPage, ApplicationError>;
    /// Commits ciphertext, initial classification and both events atomically.
    fn insert_with_metadata(
        &self,
        actor: UserId,
        case_id: CaseId,
        record: DocumentRecord,
        metadata: DocumentMetadata,
        at: OffsetDateTime,
    ) -> Result<DocumentOverview, ApplicationError>;
    /// Commits the next snapshot and audit only if the current version still matches.
    fn append(
        &self,
        actor: UserId,
        case_id: CaseId,
        expected_version: DocumentVersion,
        record: DocumentRecord,
        at: OffsetDateTime,
    ) -> Result<DocumentOverview, ApplicationError>;
    /// Authorizes the exact document and audits its descending history page.
    fn history(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: DocumentId,
        query: VersionQuery,
        at: OffsetDateTime,
    ) -> Result<VersionPage, ApplicationError>;
    /// Filters metadata before pagination and audits access before returning it.
    fn list(
        &self,
        actor: UserId,
        case_id: CaseId,
        query: DocumentQuery,
        at: OffsetDateTime,
    ) -> Result<DocumentPage, ApplicationError>;
    /// Reads metadata only and audits access in the authorization transaction.
    fn get(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: DocumentId,
        selection: VersionSelection,
        at: OffsetDateTime,
    ) -> Result<CaseDocumentSummary, ApplicationError>;
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
        selection: VersionSelection,
        action: DocumentAction,
    ) -> Result<DocumentRecord, ApplicationError>;
    /// Inserts prepared ciphertext and its success event in one transaction.
    fn insert(
        &self,
        actor: UserId,
        case_id: CaseId,
        record: DocumentRecord,
        at: OffsetDateTime,
    ) -> Result<DocumentOverview, ApplicationError>;
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
