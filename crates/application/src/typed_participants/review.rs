use super::*;
use domain::crypto::Sha256Digest;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseDirectoryStamp(pub Sha256Digest);
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityCandidateRef {
    Subject {
        id: CaseSubjectId,
        revision: SubjectRevision,
    },
    ManualParticipant {
        id: ParticipantId,
        revision: ParticipantRevision,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityCandidateSignal {
    Name,
    DeclaredIdentifier,
    Certificate,
    DocumentaryEvidence,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityCandidate {
    pub reference: IdentityCandidateRef,
    pub display_name: String,
    pub kind: Option<SubjectKind>,
    pub signals: Vec<IdentityCandidateSignal>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityDifferentDecision {
    pub candidate: IdentityCandidateRef,
    pub reason: ParticipantText<200>,
    pub support: ParticipantEvidenceLocator,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityReviewSubmission {
    pub directory_stamp: CaseDirectoryStamp,
    pub different: Vec<IdentityDifferentDecision>,
    pub selection_reason: ParticipantReason,
}
