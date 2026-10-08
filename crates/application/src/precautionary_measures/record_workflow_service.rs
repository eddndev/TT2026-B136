use super::{
    record_workflow_evidence as evidence, record_workflow_prepared,
    workflow_evidence::{clock, invalid},
    *,
};
use crate::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor, StageSupportReadLimits},
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::{DocumentHasher, Sha256Digest},
    identity::Role,
};
use std::sync::Arc;

pub struct MeasureDecisionRecordService {
    store: Arc<dyn MeasureDecisionRecordStore>,
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
    limits: StageSupportReadLimits,
}

impl MeasureDecisionRecordService {
    pub fn new(
        store: Arc<dyn MeasureDecisionRecordStore>,
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
        floor: Option<OffsetDateTime>,
    ) -> Result<OffsetDateTime, ApplicationError> {
        let at = self.clock.now();
        clock(at)?;
        if floor.is_some_and(|floor| at < floor) {
            return Err(invalid("service clock regressed"));
        }
        Ok(at)
    }

    fn prepared(
        &self,
        actor: &Principal,
        case_id: CaseId,
        command: MeasureDecisionCommand,
        material: MeasureDecisionRecordReady,
    ) -> Result<PreparedMeasureDecisionRecord, ApplicationError> {
        record_workflow_prepared::prepare(
            record_workflow_prepared::PreparationServices {
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
        value: &MeasureDecisionRecordReceipt,
    ) -> Result<(), ApplicationError> {
        evidence::receipt(self.hasher.as_ref(), value)?;
        if value.case_id() != case_id || value.actor().id != actor.id || value.command() != command
        {
            return Err(MeasureDecisionError::OperationConflict.into());
        }
        Ok(())
    }

    fn confirm(
        submission: Sha256Digest,
        review: Sha256Digest,
        expected: MeasureDecisionConfirmation,
    ) -> Result<(), ApplicationError> {
        if submission != expected.submission_digest {
            return Err(MeasureDecisionError::SubmissionMismatch.into());
        }
        if review != expected.review_digest {
            return Err(MeasureDecisionError::ReviewMismatch.into());
        }
        Ok(())
    }

    pub fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureDecisionCommand,
    ) -> Result<MeasureDecisionRecordReview, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.observation(None)?;
        let (review, earliest) =
            match self
                .store
                .prepare(&actor, case_id, &command, &self.limits)?
            {
                MeasureDecisionRecordPreparation::Ready(material) => {
                    let prepared = self.prepared(&actor, case_id, command, *material)?;
                    (
                        MeasureDecisionRecordReview::V2(Box::new(prepared.review().clone())),
                        prepared.checked.earliest_capture,
                    )
                }
                MeasureDecisionRecordPreparation::Replay(value) => {
                    self.replay(&actor, case_id, &command, &value)?;
                    let at = value.recorded_at();
                    ((*value).into_review(), at)
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
    ) -> Result<MeasureDecisionRecordReceipt, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.observation(None)?;
        let mut observed = started;
        let result = match self
            .store
            .prepare(&actor, case_id, &command, &self.limits)?
        {
            MeasureDecisionRecordPreparation::Replay(value) => {
                self.replay(&actor, case_id, &command, &value)?;
                Self::confirm(
                    value.submission_digest(),
                    value.review_digest(),
                    confirmation,
                )?;
                *value
            }
            MeasureDecisionRecordPreparation::Ready(material) => {
                let mut prepared = self.prepared(&actor, case_id, command.clone(), *material)?;
                Self::confirm(
                    prepared.review().submission_digest,
                    prepared.review().review_digest,
                    confirmation,
                )?;
                self.reauthenticate(token, &actor)?;
                observed =
                    self.observation(Some(started.max(prepared.checked.earliest_capture)))?;
                prepared.checked.earliest_capture = observed;
                let expected = prepared.review().clone();
                let history = prepared.material.record_history.clone();
                let value = self.store.commit(&actor, case_id, prepared)?;
                evidence::operation(self.hasher.as_ref(), &value)?;
                if value.group.review != expected
                    || !evidence::history_matches(&value.record_history, &history)
                {
                    return Err(invalid(
                        "committed V2 group differs from complete reviewed preparation",
                    ));
                }
                MeasureDecisionRecordReceipt::V2(Box::new(value))
            }
        };
        self.reauthenticate(token, &actor)?;
        self.observation(Some(observed.max(result.recorded_at())))?;
        Ok(result)
    }
}

impl MeasureDecisionRecordWorkflow for MeasureDecisionRecordService {
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureDecisionCommand,
    ) -> Result<MeasureDecisionRecordReview, ApplicationError> {
        MeasureDecisionRecordService::prepare(self, token, case_id, command)
    }
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureDecisionCommand,
        confirmation: MeasureDecisionConfirmation,
    ) -> Result<MeasureDecisionRecordReceipt, ApplicationError> {
        MeasureDecisionRecordService::submit(self, token, case_id, command, confirmation)
    }
}
