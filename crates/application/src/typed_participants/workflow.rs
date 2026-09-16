use super::*;
use domain::cases::CaseId;
pub trait TypedParticipantWorkflow: Send + Sync {
    fn review_participant(
        &self,
        token: &str,
        case_id: CaseId,
        request: ParticipantProposalRequest,
    ) -> Result<ParticipantProposalReview, ApplicationError>;
    fn prepare_participant(
        &self,
        token: &str,
        case_id: CaseId,
        request: ParticipantPreparationRequest,
    ) -> Result<ParticipantSigningDraft, ApplicationError>;
    fn submit_participant(
        &self,
        token: &str,
        case_id: CaseId,
        submission: ParticipantSubmission,
    ) -> Result<ParticipantDetail, ApplicationError>;
    fn review_subject(
        &self,
        token: &str,
        case_id: CaseId,
        id: CaseSubjectId,
        expected: SubjectRevision,
        values: SubjectValues,
    ) -> Result<SubjectReplacementReview, ApplicationError>;
    fn replace_subject(
        &self,
        token: &str,
        case_id: CaseId,
        request: SubjectReplacementRequest,
    ) -> Result<SubjectSnapshot, ApplicationError>;
    fn list_subjects(
        &self,
        token: &str,
        case_id: CaseId,
        query: SubjectQuery,
    ) -> Result<SubjectPage, ApplicationError>;
    fn get_subject(
        &self,
        token: &str,
        case_id: CaseId,
        id: CaseSubjectId,
    ) -> Result<SubjectSnapshot, ApplicationError>;
    fn get_subject_revision(
        &self,
        token: &str,
        case_id: CaseId,
        id: CaseSubjectId,
        revision: SubjectRevision,
    ) -> Result<SubjectSnapshot, ApplicationError>;
    fn subject_history(
        &self,
        token: &str,
        case_id: CaseId,
        id: CaseSubjectId,
        query: SubjectHistoryQuery,
    ) -> Result<SubjectHistoryPage, ApplicationError>;
    fn credential(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
        revision: ParticipantRevision,
    ) -> Result<ParticipantCredentialEvidence, ApplicationError>;
}
