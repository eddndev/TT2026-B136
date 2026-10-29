use std::sync::Arc;

use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{DocumentHasher, DocumentVersionRef},
    document_signing::Uuid,
};

use super::{
    DocumentSigningVerifier, ManifestTimestamping, SigningBindingStore, SigningCommit,
    SigningReceipt, SigningVerificationContext, VerifiedDocumentSigning,
};
use crate::{
    documents::{CaseDocumentStore, DocumentFormatBatchValidator, DocumentProcessor},
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};

/// Durable exact-operation recovery and atomic evidence, document and audit writes.
///
/// Reads authorize the complete current principal, role and case membership and
/// exact document association, including closed cases. A mutation additionally
/// locks and rechecks active case, account counters, current binding and complete
/// published trust. It compares the immutable base version before modifying it.
/// The clock is read after lock waits and must not precede not_before or exceed
/// the supplied check's validity interval. The same transaction captures that
/// time in both receipt and individual evidence and appends the success audit.
/// Exact operation replay returns the original evidence before mutable trust or
/// expiry checks; any different consent or either signature conflicts. No method
/// may overwrite historical evidence or retry the timestamp request.
pub trait IndividualSigningStore: Send + Sync {
    fn find(
        &self,
        principal: &Principal,
        case: CaseId,
        reference: DocumentVersionRef,
        operation: Uuid,
    ) -> Result<Option<SigningReceipt>, ApplicationError>;
    fn commit(&self, verified: VerifiedDocumentSigning) -> Result<SigningCommit, ApplicationError>;
    fn verification_context(
        &self,
        principal: &Principal,
        case: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<SigningVerificationContext, ApplicationError>;
}

pub struct DocumentSigningPorts {
    pub identity: Arc<dyn IdentityWorkflow>,
    pub bindings: Arc<dyn SigningBindingStore>,
    pub documents: Arc<dyn CaseDocumentStore>,
    pub store: Arc<dyn IndividualSigningStore>,
    pub verifier: Arc<dyn DocumentSigningVerifier>,
    pub hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub processor: Arc<DocumentProcessor>,
    pub formats: Arc<dyn DocumentFormatBatchValidator>,
    /// Time-stamps the evidence manifest; the time-stamping CRL it checks is
    /// the current credential trust snapshot's, so the anchor of
    /// `processor` must be that trust root. Rationale:
    /// `docs/adr/0085-in-browser-evidence-validation.md`.
    pub manifest_timestamps: Arc<dyn ManifestTimestamping>,
}
