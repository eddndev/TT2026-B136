use super::*;
use crate::documents::{DocumentRecord, StageDocumentFormat};
use domain::crypto::{CredentialCheck, Sha256Digest};

/// Constructed only after exact bounded support and declaration verification.
pub struct PreparedTypedParticipantChange {
    request: ParticipantPreparationRequest,
    records: Vec<DocumentRecord>,
    formats: Vec<StageDocumentFormat>,
    trust: Option<CredentialTrustSnapshot>,
    check: Option<CredentialCheck>,
    declaration: Option<Vec<u8>>,
    submission_digest: Sha256Digest,
}
impl PreparedTypedParticipantChange {
    pub(super) fn new(
        request: ParticipantPreparationRequest,
        preparation: TypedParticipantPreparation,
        formats: Vec<StageDocumentFormat>,
        check: Option<CredentialCheck>,
        declaration: Option<Vec<u8>>,
        submission_digest: Sha256Digest,
    ) -> Self {
        Self {
            request,
            records: preparation.records,
            formats,
            trust: preparation.trust,
            check,
            declaration,
            submission_digest,
        }
    }
    pub fn request(&self) -> &ParticipantPreparationRequest {
        &self.request
    }
    pub fn records(&self) -> &[DocumentRecord] {
        &self.records
    }
    pub fn formats(&self) -> &[StageDocumentFormat] {
        &self.formats
    }
    pub fn trust(&self) -> Option<&CredentialTrustSnapshot> {
        self.trust.as_ref()
    }
    pub fn check(&self) -> Option<&CredentialCheck> {
        self.check.as_ref()
    }
    pub fn declaration(&self) -> Option<&[u8]> {
        self.declaration.as_deref()
    }
    pub const fn submission_digest(&self) -> Sha256Digest {
        self.submission_digest
    }
}
/// Staff identity replacement does not rewrite any bound participant revision.
pub struct PreparedSubjectChange {
    request: SubjectReplacementRequest,
    records: Vec<DocumentRecord>,
    formats: Vec<StageDocumentFormat>,
}
impl PreparedSubjectChange {
    pub(super) fn new(
        request: SubjectReplacementRequest,
        records: Vec<DocumentRecord>,
        formats: Vec<StageDocumentFormat>,
    ) -> Self {
        Self {
            request,
            records,
            formats,
        }
    }

    pub fn request(&self) -> &SubjectReplacementRequest {
        &self.request
    }
    pub fn records(&self) -> &[DocumentRecord] {
        &self.records
    }
    pub fn formats(&self) -> &[StageDocumentFormat] {
        &self.formats
    }
}
