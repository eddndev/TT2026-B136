use super::*;
use domain::crypto::{CredentialCertificate, InternalDeclarationVerifier};
impl TypedParticipantStore for PostgresTypedParticipantStore {
    fn review_participant(
        &self,
        actor: UserId,
        case: CaseId,
        request: ParticipantProposalRequest,
        subject: CaseSubjectId,
        participant: ParticipantId,
        certificate: Option<CredentialCertificate>,
        at: OffsetDateTime,
    ) -> Result<ParticipantProposalReview, ApplicationError> {
        self.review_role(actor, case, request, subject, participant, certificate, at)
    }
    fn prepare_participant(
        &self,
        actor: UserId,
        case: CaseId,
        request: &ParticipantPreparationRequest,
        limits: &StageSupportReadLimits,
    ) -> Result<TypedParticipantPreparation, ApplicationError> {
        let fingerprint = request
            .certificate
            .as_ref()
            .map(|bytes| {
                crate::certificates::InternalRsaDeclarationVerifier
                    .inspect_certificate(bytes)
                    .map(|c| c.fingerprint)
            })
            .transpose()?;
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, true)?;
        let result = preparation::participant(
            &mut tx,
            case,
            request,
            limits,
            self.hasher.as_ref(),
            fingerprint,
        )?;
        audit(
            &mut tx,
            &principal,
            "participant.prepared",
            &format!(
                "case:{case}:participant:{}",
                request.proposal.participant_id()
            ),
            self.clock.now(),
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
    fn commit_participant(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedTypedParticipantChange,
    ) -> Result<ParticipantDetail, ApplicationError> {
        self.commit_role(actor, case, prepared)
    }
    fn review_subject(
        &self,
        actor: UserId,
        case: CaseId,
        id: CaseSubjectId,
        expected: SubjectRevision,
        values: SubjectValues,
        at: OffsetDateTime,
    ) -> Result<SubjectReplacementReview, ApplicationError> {
        self.review_identity(actor, case, id, expected, values, at)
    }
    fn prepare_subject(
        &self,
        actor: UserId,
        case: CaseId,
        request: &SubjectReplacementRequest,
        limits: &StageSupportReadLimits,
    ) -> Result<SubjectPreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, true)?;
        let result = preparation::subject(&mut tx, case, request, limits, self.hasher.as_ref())?;
        audit(
            &mut tx,
            &principal,
            "participant.subject_prepared",
            &format!("case:{case}:subject:{}", request.id),
            self.clock.now(),
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
    fn commit_subject(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedSubjectChange,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        self.commit_identity(actor, case, prepared)
    }
    fn list_subjects(
        &self,
        actor: UserId,
        case: CaseId,
        query: SubjectQuery,
        at: OffsetDateTime,
    ) -> Result<SubjectPage, ApplicationError> {
        self.subject_list(actor, case, query, at)
    }
    fn get_subject(
        &self,
        actor: UserId,
        case: CaseId,
        id: CaseSubjectId,
        at: OffsetDateTime,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        self.subject_get(actor, case, id, None, at)
    }
    fn get_subject_revision(
        &self,
        actor: UserId,
        case: CaseId,
        id: CaseSubjectId,
        revision: SubjectRevision,
        at: OffsetDateTime,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        self.subject_get(actor, case, id, Some(revision), at)
    }
    fn subject_history(
        &self,
        actor: UserId,
        case: CaseId,
        id: CaseSubjectId,
        query: SubjectHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<SubjectHistoryPage, ApplicationError> {
        self.subject_history_page(actor, case, id, query, at)
    }
    fn credential(
        &self,
        actor: UserId,
        case: CaseId,
        id: ParticipantId,
        revision: ParticipantRevision,
        at: OffsetDateTime,
    ) -> Result<ParticipantCredentialEvidence, ApplicationError> {
        self.credential_get(actor, case, id, revision, at)
    }
}
