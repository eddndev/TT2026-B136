use super::{workflow_evidence as evidence, workflow_prepared, *};
use crate::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor, StageSupportReadLimits},
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::DocumentHasher,
    identity::Role,
};
use std::sync::Arc;

pub struct MeasureDecisionService {
    store: Arc<dyn MeasureDecisionStore>,
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
    limits: StageSupportReadLimits,
}

impl MeasureDecisionService {
    pub fn new(
        store: Arc<dyn MeasureDecisionStore>,
        identity: Arc<dyn IdentityWorkflow>,
        processor: Arc<DocumentProcessor>,
        validator: Arc<dyn DocumentFormatBatchValidator>,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            identity,
            processor,
            validator,
            hasher,
            clock,
            limits: StageSupportReadLimits::standard(),
        }
    }

    fn actor(&self, token: &str) -> Result<Principal, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !matches!(actor.role, Role::Owner | Role::Litigator) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor)
    }

    fn reauthenticate(&self, token: &str, actor: &Principal) -> Result<(), ApplicationError> {
        if self.actor(token)? != *actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }

    fn observation(
        &self,
        lower: Option<OffsetDateTime>,
    ) -> Result<OffsetDateTime, ApplicationError> {
        let now = self.clock.now();
        evidence::clock(now)?;
        if lower.is_some_and(|lower| now < lower) {
            return Err(evidence::invalid("service clock regressed"));
        }
        Ok(now)
    }

    fn prepared(
        &self,
        actor: &Principal,
        case_id: CaseId,
        command: MeasureDecisionCommand,
        material: MeasureDecisionReady,
    ) -> Result<PreparedMeasureDecision, ApplicationError> {
        workflow_prepared::prepare(
            workflow_prepared::PreparationServices {
                hasher: self.hasher.clone(),
                processor: &self.processor,
                validator: self.validator.as_ref(),
                limits: &self.limits,
            },
            actor,
            case_id,
            command,
            material,
        )
    }

    fn replay(
        &self,
        actor: &Principal,
        case_id: CaseId,
        command: &MeasureDecisionCommand,
        result: &MeasureDecisionStoredOperation,
    ) -> Result<(), ApplicationError> {
        evidence::operation(self.hasher.as_ref(), result)?;
        let review = &result.group.review;
        if review.case_id != case_id || review.actor.id != actor.id || review.command != *command {
            return Err(MeasureDecisionError::OperationConflict.into());
        }
        Ok(())
    }

    fn confirmation(
        review: &MeasureDecisionReview,
        expected: MeasureDecisionConfirmation,
    ) -> Result<(), ApplicationError> {
        if review.submission_digest != expected.submission_digest {
            return Err(MeasureDecisionError::SubmissionMismatch.into());
        }
        if review.review_digest != expected.review_digest {
            return Err(MeasureDecisionError::ReviewMismatch.into());
        }
        Ok(())
    }

    pub fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureDecisionCommand,
    ) -> Result<MeasureDecisionReview, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.observation(None)?;
        let (review, earliest) =
            match self
                .store
                .prepare(&actor, case_id, &command, &self.limits)?
            {
                MeasureDecisionPreparation::Ready(material) => {
                    let prepared = self.prepared(&actor, case_id, command, *material)?;
                    (prepared.review().clone(), prepared.checked.earliest_capture)
                }
                MeasureDecisionPreparation::Replay(result) => {
                    self.replay(&actor, case_id, &command, &result)?;
                    (result.group.review, result.group.recorded_at)
                }
            };
        self.reauthenticate(token, &actor)?;
        self.observation(Some(started.max(earliest)))?;
        Ok(review)
    }

    pub fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureDecisionCommand,
        confirmation: MeasureDecisionConfirmation,
    ) -> Result<MeasureDecisionStoredOperation, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.observation(None)?;
        let mut observed_at = started;
        let result = match self
            .store
            .prepare(&actor, case_id, &command, &self.limits)?
        {
            MeasureDecisionPreparation::Replay(result) => {
                self.replay(&actor, case_id, &command, &result)?;
                Self::confirmation(&result.group.review, confirmation)?;
                *result
            }
            MeasureDecisionPreparation::Ready(material) => {
                let mut prepared = self.prepared(&actor, case_id, command.clone(), *material)?;
                Self::confirmation(prepared.review(), confirmation)?;
                self.reauthenticate(token, &actor)?;
                observed_at =
                    self.observation(Some(started.max(prepared.checked.earliest_capture)))?;
                prepared.checked.earliest_capture = observed_at;
                let expected_review = prepared.review().clone();
                let history = prepared.material.measure_history.clone();
                let result = self.store.commit(&actor, case_id, prepared)?;
                self.replay(&actor, case_id, &command, &result)?;
                if result.group.review != expected_review
                    || !evidence::history_matches(&result.measure_history, &history)
                {
                    return Err(evidence::invalid(
                        "committed group differs from complete reviewed preparation",
                    ));
                }
                result
            }
        };
        self.reauthenticate(token, &actor)?;
        self.observation(Some(observed_at.max(result.group.recorded_at)))?;
        Ok(result)
    }
}

impl MeasureDecisionWorkflow for MeasureDecisionService {
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureDecisionCommand,
    ) -> Result<MeasureDecisionReview, ApplicationError> {
        MeasureDecisionService::prepare(self, token, case_id, command)
    }
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureDecisionCommand,
        confirmation: MeasureDecisionConfirmation,
    ) -> Result<MeasureDecisionStoredOperation, ApplicationError> {
        MeasureDecisionService::submit(self, token, case_id, command, confirmation)
    }
}
