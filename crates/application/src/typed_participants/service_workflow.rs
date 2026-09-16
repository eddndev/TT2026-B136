use super::*;
use domain::{cases::CaseId, crypto::CredentialFailure, identity::Permission};

impl TypedParticipantWorkflow for TypedParticipantService {
    fn review_participant(
        &self,
        token: &str,
        case_id: CaseId,
        request: ParticipantProposalRequest,
    ) -> Result<ParticipantProposalReview, ApplicationError> {
        let actor = self.actor(token, Permission::ManageParticipant)?;
        let certificate = self.inspect(request.certificate.as_deref())?;
        self.store.review_participant(
            actor,
            case_id,
            request,
            CaseSubjectId::new(),
            ParticipantId::new(),
            certificate,
            self.clock.now(),
        )
    }
    fn prepare_participant(
        &self,
        token: &str,
        case_id: CaseId,
        request: ParticipantPreparationRequest,
    ) -> Result<ParticipantSigningDraft, ApplicationError> {
        let actor = self.actor(token, Permission::ManageParticipant)?;
        let (preparation, _, certificate) = self.prepare(actor, case_id, &request)?;
        let declaration = self.declaration(
            case_id,
            &request.proposal,
            preparation.trust.as_ref(),
            certificate,
        )?;
        self.same_actor(token, actor)?;
        let submission_digest = if declaration.is_none() {
            Some(participant_submission_digest(
                self.hasher.as_ref(),
                case_id,
                &request,
                None,
            )?)
        } else {
            None
        };
        let submission_revision = request
            .proposal
            .expected_participant()
            .next()
            .ok_or(ApplicationError::ParticipantRevisionExhausted)?;
        Ok(ParticipantSigningDraft {
            submission_digest,
            submission_revision,
            proposal: request.proposal,
            review: request.review,
            declaration,
        })
    }
    fn submit_participant(
        &self,
        token: &str,
        case_id: CaseId,
        submission: ParticipantSubmission,
    ) -> Result<ParticipantDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ManageParticipant)?;
        if submission.prepared.certificate.is_none() && submission.signature.is_some() {
            return Err(ApplicationError::ParticipantCredentialUnexpected);
        }
        if submission.prepared.certificate.is_some() && submission.signature.is_none() {
            return Err(ApplicationError::ParticipantCredentialRequired);
        }
        if submission
            .signature
            .as_ref()
            .is_some_and(|v| v.as_bytes().len() != 384)
        {
            return Err(ApplicationError::ParticipantCredentialRejected(
                CredentialFailure::InvalidSignature,
            ));
        }
        let request = submission.prepared;
        let (preparation, formats, certificate) = self.prepare(actor, case_id, &request)?;
        let declaration = self.declaration(
            case_id,
            &request.proposal,
            preparation.trust.as_ref(),
            certificate,
        )?;
        let check = match (&declaration, &submission.signature) {
            (Some(declaration), Some(signature)) => {
                let trust = preparation
                    .trust
                    .as_ref()
                    .ok_or(ApplicationError::CredentialTrustUnavailable)?;
                let check = self.credentials.verify(
                    &declaration.digest,
                    &declaration.certificate.der,
                    signature,
                    &trust.inspection.root_der,
                    &trust.inspection.crl_der,
                    self.clock.now().unix_timestamp(),
                )?;
                if check.statement_digest != declaration.digest
                    || check.certificate != declaration.certificate
                    || check.signature != *signature
                    || check.trust != trust.inspection
                {
                    return Err(ApplicationError::ParticipantCredentialRejected(
                        CredentialFailure::InvalidSignature,
                    ));
                }
                Some(check)
            }
            (None, None) => None,
            _ => return Err(ApplicationError::ParticipantCredentialRequired),
        };
        self.same_actor(token, actor)?;
        let proof = check
            .as_ref()
            .map(|c| (c.statement_digest, c.certificate.fingerprint, &c.signature));
        let digest = participant_submission_digest(self.hasher.as_ref(), case_id, &request, proof)?;
        let prepared = PreparedTypedParticipantChange::new(
            request,
            preparation,
            formats,
            check,
            declaration.map(|d| d.bytes),
            digest,
        );
        self.store.commit_participant(actor, case_id, prepared)
    }
    fn review_subject(
        &self,
        token: &str,
        case_id: CaseId,
        id: CaseSubjectId,
        expected: SubjectRevision,
        values: SubjectValues,
    ) -> Result<SubjectReplacementReview, ApplicationError> {
        let actor = self.actor(token, Permission::ManageParticipant)?;
        expected
            .next()
            .ok_or(ApplicationError::SubjectRevisionExhausted)?;
        self.store
            .review_subject(actor, case_id, id, expected, values, self.clock.now())
    }
    fn replace_subject(
        &self,
        token: &str,
        case_id: CaseId,
        request: SubjectReplacementRequest,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        let actor = self.actor(token, Permission::ManageParticipant)?;
        request
            .expected_revision
            .next()
            .ok_or(ApplicationError::SubjectRevisionExhausted)?;
        request.review.canonical_bytes()?;
        let refs = super::validation::subject_supports(&request)?;
        let preparation = self
            .store
            .prepare_subject(actor, case_id, &request, &self.limits)?;
        if preparation.directory_stamp != request.review.directory_stamp {
            return Err(ApplicationError::ParticipantIdentityReviewConflict);
        }
        let records = super::validation::ordered_records(&refs, preparation.records)?;
        let formats =
            self.processor
                .validate_support_batch(&records, &self.limits, self.formats.as_ref())?;
        self.same_actor(token, actor)?;
        self.store.commit_subject(
            actor,
            case_id,
            PreparedSubjectChange::new(request, records, formats),
        )
    }
    fn list_subjects(
        &self,
        token: &str,
        case_id: CaseId,
        query: SubjectQuery,
    ) -> Result<SubjectPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadParticipant)?;
        self.store
            .list_subjects(actor, case_id, query, self.clock.now())
    }
    fn get_subject(
        &self,
        token: &str,
        case_id: CaseId,
        id: CaseSubjectId,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        let actor = self.actor(token, Permission::ReadParticipant)?;
        self.store.get_subject(actor, case_id, id, self.clock.now())
    }
    fn get_subject_revision(
        &self,
        token: &str,
        case_id: CaseId,
        id: CaseSubjectId,
        revision: SubjectRevision,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        let actor = self.actor(token, Permission::ReadParticipant)?;
        self.store
            .get_subject_revision(actor, case_id, id, revision, self.clock.now())
    }
    fn subject_history(
        &self,
        token: &str,
        case_id: CaseId,
        id: CaseSubjectId,
        query: SubjectHistoryQuery,
    ) -> Result<SubjectHistoryPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadParticipant)?;
        self.store
            .subject_history(actor, case_id, id, query, self.clock.now())
    }
    fn credential(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
        revision: ParticipantRevision,
    ) -> Result<ParticipantCredentialEvidence, ApplicationError> {
        let actor = self.actor(token, Permission::ReadParticipant)?;
        self.store
            .credential(actor, case_id, id, revision, self.clock.now())
    }
}
