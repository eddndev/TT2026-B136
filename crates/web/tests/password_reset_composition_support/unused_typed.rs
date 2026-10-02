use super::unused::Unused;
use application::typed_participants::*;
use domain::cases::CaseId;

impl TypedParticipantWorkflow for Unused {
    fn review_participant(
        &self,
        _: &str,
        _: CaseId,
        _: ParticipantProposalRequest,
    ) -> Result<ParticipantProposalReview, ApplicationError> {
        unreachable!("composition test does not review participants")
    }
    fn prepare_participant(
        &self,
        _: &str,
        _: CaseId,
        _: ParticipantPreparationRequest,
    ) -> Result<ParticipantSigningDraft, ApplicationError> {
        unreachable!("composition test does not prepare participants")
    }
    fn submit_participant(
        &self,
        _: &str,
        _: CaseId,
        _: ParticipantSubmission,
    ) -> Result<ParticipantDetail, ApplicationError> {
        unreachable!("composition test does not submit participants")
    }
    fn review_subject(
        &self,
        _: &str,
        _: CaseId,
        _: CaseSubjectId,
        _: SubjectRevision,
        _: SubjectValues,
    ) -> Result<SubjectReplacementReview, ApplicationError> {
        unreachable!("composition test does not review subjects")
    }
    fn replace_subject(
        &self,
        _: &str,
        _: CaseId,
        _: SubjectReplacementRequest,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        unreachable!("composition test does not replace subjects")
    }
    fn list_subjects(
        &self,
        _: &str,
        _: CaseId,
        _: SubjectQuery,
    ) -> Result<SubjectPage, ApplicationError> {
        unreachable!("composition test does not list subjects")
    }
    fn get_subject(
        &self,
        _: &str,
        _: CaseId,
        _: CaseSubjectId,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        unreachable!("composition test does not read subjects")
    }
    fn get_subject_revision(
        &self,
        _: &str,
        _: CaseId,
        _: CaseSubjectId,
        _: SubjectRevision,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        unreachable!("composition test does not read subject revisions")
    }
    fn subject_history(
        &self,
        _: &str,
        _: CaseId,
        _: CaseSubjectId,
        _: SubjectHistoryQuery,
    ) -> Result<SubjectHistoryPage, ApplicationError> {
        unreachable!("composition test does not read subject history")
    }
    fn credential(
        &self,
        _: &str,
        _: CaseId,
        _: ParticipantId,
        _: ParticipantRevision,
    ) -> Result<ParticipantCredentialEvidence, ApplicationError> {
        unreachable!("composition test does not read participant credentials")
    }
}
