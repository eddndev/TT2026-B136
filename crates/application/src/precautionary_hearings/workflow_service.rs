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

pub struct PrecautionaryHearingService {
    store: Arc<dyn PrecautionaryHearingStore>,
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
    limits: StageSupportReadLimits,
}

impl PrecautionaryHearingService {
    pub fn new(
        store: Arc<dyn PrecautionaryHearingStore>,
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
        command: PrecautionaryHearingCommand,
        material: PrecautionaryHearingReady,
    ) -> Result<PreparedPrecautionaryHearing, ApplicationError> {
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
        command: &PrecautionaryHearingCommand,
        result: &PrecautionaryHearingStoredOperation,
    ) -> Result<(), ApplicationError> {
        evidence::operation(self.hasher.as_ref(), result)?;
        let review = &result.capture.review;
        if review.case_id != case_id || review.actor.id != actor.id || review.command != *command {
            return Err(PrecautionaryHearingError::OperationConflict.into());
        }
        Ok(())
    }

    fn confirmation(
        review: &PrecautionaryHearingReview,
        expected: PrecautionaryHearingConfirmation,
    ) -> Result<(), ApplicationError> {
        if review.submission_digest != expected.submission_digest {
            return Err(PrecautionaryHearingError::SubmissionMismatch.into());
        }
        if review.review_digest != expected.review_digest {
            return Err(PrecautionaryHearingError::ReviewMismatch.into());
        }
        Ok(())
    }

    pub fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: PrecautionaryHearingCommand,
    ) -> Result<PrecautionaryHearingReview, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.observation(None)?;
        let (review, earliest) =
            match self
                .store
                .prepare(&actor, case_id, &command, &self.limits)?
            {
                PrecautionaryHearingPreparation::Ready(material) => {
                    let prepared = self.prepared(&actor, case_id, command, *material)?;
                    (prepared.review().clone(), prepared.checked.earliest_capture)
                }
                PrecautionaryHearingPreparation::Replay(result) => {
                    self.replay(&actor, case_id, &command, &result)?;
                    (result.capture.review, result.capture.recorded_at)
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
        command: PrecautionaryHearingCommand,
        confirmation: PrecautionaryHearingConfirmation,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.observation(None)?;
        let mut observed_at = started;
        let result = match self
            .store
            .prepare(&actor, case_id, &command, &self.limits)?
        {
            PrecautionaryHearingPreparation::Replay(result) => {
                self.replay(&actor, case_id, &command, &result)?;
                Self::confirmation(&result.capture.review, confirmation)?;
                *result
            }
            PrecautionaryHearingPreparation::Ready(material) => {
                let mut prepared = self.prepared(&actor, case_id, command.clone(), *material)?;
                Self::confirmation(prepared.review(), confirmation)?;
                self.reauthenticate(token, &actor)?;
                observed_at =
                    self.observation(Some(started.max(prepared.checked.earliest_capture)))?;
                prepared.checked.earliest_capture = observed_at;
                let expected_review = prepared.review().clone();
                let prefix = prepared.material.history.clone();
                let merged = prepared.merged.clone();
                let result = self.store.commit(&actor, case_id, prepared)?;
                self.replay(&actor, case_id, &command, &result)?;
                let expected_prefix = prefix
                    .as_ref()
                    .map(|h| h.captures.as_slice())
                    .unwrap_or(&[]);
                if result.capture.review != expected_review
                    || result.history.captures[..result.history.captures.len() - 1]
                        != *expected_prefix
                    || prefix
                        .as_ref()
                        .is_some_and(|h| h.origin != result.history.origin)
                    || evidence::merge(&result.history.measure_history, &merged)? != merged
                {
                    return Err(evidence::invalid(
                        "committed operation differs from complete reviewed preparation",
                    ));
                }
                result
            }
        };
        self.reauthenticate(token, &actor)?;
        self.observation(Some(observed_at.max(result.capture.recorded_at)))?;
        Ok(result)
    }
}

impl PrecautionaryHearingWorkflow for PrecautionaryHearingService {
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: PrecautionaryHearingCommand,
    ) -> Result<PrecautionaryHearingReview, ApplicationError> {
        PrecautionaryHearingService::prepare(self, token, case_id, command)
    }
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: PrecautionaryHearingCommand,
        confirmation: PrecautionaryHearingConfirmation,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError> {
        PrecautionaryHearingService::submit(self, token, case_id, command, confirmation)
    }
}
