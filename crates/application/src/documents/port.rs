//! Outbound port for encrypted document records.

use domain::audit::ChainVerification;
use domain::crypto::DocumentId;

use crate::documents::{DocumentRecord, DocumentSummary, EvidenceExport};
use crate::verification::VerificationReport;
use crate::ApplicationError;

/// Persistent storage used by the document workflow.
pub trait DocumentRepository: Send + Sync {
    /// Inserts a new identity and rejects duplicates.
    fn insert(&self, record: DocumentRecord) -> Result<(), ApplicationError>;

    /// Replaces an existing identity and rejects missing records.
    fn replace(&self, record: DocumentRecord) -> Result<(), ApplicationError>;

    /// Loads one identity, returning `None` when it is absent.
    fn find(&self, id: DocumentId) -> Result<Option<DocumentRecord>, ApplicationError>;
}

/// Inbound application boundary consumed by the HTTP adapter.
pub trait DocumentWorkflow: Send + Sync {
    fn upload(
        &self,
        actor: &str,
        name: &str,
        document: &[u8],
    ) -> Result<DocumentSummary, ApplicationError>;

    fn seal(&self, actor: &str, id: DocumentId) -> Result<DocumentSummary, ApplicationError>;

    fn verify(&self, actor: &str, id: DocumentId) -> Result<VerificationReport, ApplicationError>;

    fn export_evidence(
        &self,
        actor: &str,
        id: DocumentId,
    ) -> Result<EvidenceExport, ApplicationError>;

    fn verify_audit(&self) -> Result<ChainVerification, ApplicationError>;
}
