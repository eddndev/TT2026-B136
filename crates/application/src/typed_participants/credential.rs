use super::*;
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::CredentialCheck};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantCredentialEvidence {
    pub case_id: CaseId,
    pub reference: ParticipantCredentialRef,
    pub subject: SubjectRevisionRef,
    pub declaration: Vec<u8>,
    pub check: CredentialCheck,
    pub trust: CredentialTrustSnapshot,
    pub accepted_at: OffsetDateTime,
    pub accepted_by: ParticipantActorSnapshot,
}
