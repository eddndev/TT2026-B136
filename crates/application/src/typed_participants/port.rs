use super::*;
use crate::documents::DocumentRecord;
use domain::{
    cases::CaseId, clock::OffsetDateTime, crypto::CredentialCertificate, identity::UserId,
};
pub struct TypedParticipantPreparation {
    pub directory_stamp: CaseDirectoryStamp,
    pub subject_values: SubjectValues,
    pub records: Vec<DocumentRecord>,
    pub trust: Option<CredentialTrustSnapshot>,
}
pub struct SubjectPreparation {
    pub directory_stamp: CaseDirectoryStamp,
    pub records: Vec<DocumentRecord>,
}
pub trait TypedParticipantStore: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn review_participant(
        &self,
        actor: UserId,
        case_id: CaseId,
        request: ParticipantProposalRequest,
        proposed_subject: CaseSubjectId,
        proposed_participant: ParticipantId,
        certificate: Option<CredentialCertificate>,
        at: OffsetDateTime,
    ) -> Result<ParticipantProposalReview, ApplicationError>;
    fn prepare_participant(
        &self,
        actor: UserId,
        case_id: CaseId,
        request: &ParticipantPreparationRequest,
        limits: &StageSupportReadLimits,
    ) -> Result<TypedParticipantPreparation, ApplicationError>;
    fn commit_participant(
        &self,
        actor: UserId,
        case_id: CaseId,
        prepared: PreparedTypedParticipantChange,
    ) -> Result<ParticipantDetail, ApplicationError>;
    fn review_subject(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: CaseSubjectId,
        expected: SubjectRevision,
        values: SubjectValues,
        at: OffsetDateTime,
    ) -> Result<SubjectReplacementReview, ApplicationError>;
    fn prepare_subject(
        &self,
        actor: UserId,
        case_id: CaseId,
        request: &SubjectReplacementRequest,
        limits: &StageSupportReadLimits,
    ) -> Result<SubjectPreparation, ApplicationError>;
    fn commit_subject(
        &self,
        actor: UserId,
        case_id: CaseId,
        prepared: PreparedSubjectChange,
    ) -> Result<SubjectSnapshot, ApplicationError>;
    // Read signatures mirror workflow, token replaced by actor:UserId,
    // with final at:OffsetDateTime argument; all commit read audit before return.
    fn list_subjects(
        &self,
        actor: UserId,
        case_id: CaseId,
        query: SubjectQuery,
        at: OffsetDateTime,
    ) -> Result<SubjectPage, ApplicationError>;
    fn get_subject(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: CaseSubjectId,
        at: OffsetDateTime,
    ) -> Result<SubjectSnapshot, ApplicationError>;
    fn get_subject_revision(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: CaseSubjectId,
        revision: SubjectRevision,
        at: OffsetDateTime,
    ) -> Result<SubjectSnapshot, ApplicationError>;
    fn subject_history(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: CaseSubjectId,
        query: SubjectHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<SubjectHistoryPage, ApplicationError>;
    fn credential(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: ParticipantId,
        revision: ParticipantRevision,
        at: OffsetDateTime,
    ) -> Result<ParticipantCredentialEvidence, ApplicationError>;
}
