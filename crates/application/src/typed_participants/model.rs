use super::*;
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectSnapshot {
    pub case_id: CaseId,
    pub id: CaseSubjectId,
    pub revision: SubjectRevision,
    pub values: SubjectValues,
    pub values_digest: Sha256Digest,
    pub changed_at: OffsetDateTime,
    pub changed_by: ParticipantActorSnapshot,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantCredentialRef {
    pub participant_id: ParticipantId,
    pub participant_revision: ParticipantRevision,
    pub statement_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedParticipantSnapshot {
    pub case_id: CaseId,
    pub id: ParticipantId,
    pub revision: ParticipantRevision,
    pub values: TypedParticipantValues,
    pub values_digest: Sha256Digest,
    pub changed_at: OffsetDateTime,
    pub changed_by: ParticipantActorSnapshot,
    pub credential_origin: Option<ParticipantCredentialRef>,
    pub submission_digest: Sha256Digest,
    pub submission_revision: ParticipantRevision,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipantRevisionSnapshot {
    Manual(Box<ParticipantSnapshot>),
    Typed(Box<TypedParticipantSnapshot>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantDetail {
    pub revision: ParticipantRevisionSnapshot,
    pub bound_subject: Option<SubjectSnapshot>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantOverview {
    pub case_id: CaseId,
    pub id: ParticipantId,
    pub revision: ParticipantRevision,
    pub display_name: String,
    pub procedural_role: String,
    pub organization: Option<String>,
    pub directory_status: DirectoryStatus,
    pub kind: Option<ParticipantKind>,
    pub subject: Option<SubjectRevisionRef>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectOverview {
    pub case_id: CaseId,
    pub id: CaseSubjectId,
    pub revision: SubjectRevision,
    pub kind: SubjectKind,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectPage {
    pub subjects: Vec<SubjectOverview>,
    pub has_more: bool,
    pub next_after_id: Option<CaseSubjectId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectHistoryPage {
    pub revisions: Vec<SubjectSnapshot>,
    pub has_more: bool,
    pub next_before_revision: Option<SubjectRevision>,
}
