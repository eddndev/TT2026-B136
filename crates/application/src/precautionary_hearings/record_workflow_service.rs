use super::{record_workflow_checks as checks, record_workflow_prepared, *};
use crate::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor, StageSupportReadLimits},
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::DocumentHasher,
};
use std::sync::Arc;

pub struct PrecautionaryHearingRecordService {
    store: Arc<dyn PrecautionaryHearingRecordStore>,
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
    limits: StageSupportReadLimits,
}

enum Loaded {
    Ready(Box<PreparedPrecautionaryHearingRecord>),
    Replay(Box<PrecautionaryHearingRecordStoredOperation>),
}
impl Loaded {
    fn review(&self) -> &PrecautionaryHearingReview {
        match self {
            Self::Ready(p) => p.review(),
            Self::Replay(r) => &r.capture.review,
        }
    }
    fn earliest(&self) -> OffsetDateTime {
        match self {
            Self::Ready(p) => p.checked.earliest_capture,
            Self::Replay(r) => r.capture.recorded_at,
        }
    }
}

impl PrecautionaryHearingRecordService {
    pub fn new(
        store: Arc<dyn PrecautionaryHearingRecordStore>,
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

    fn load(
        &self,
        actor: &Principal,
        case_id: CaseId,
        command: PrecautionaryHearingCommand,
    ) -> Result<Loaded, ApplicationError> {
        match self.store.prepare(actor, case_id, &command, &self.limits)? {
            PrecautionaryHearingRecordPreparation::Ready(material) => {
                let prepared = record_workflow_prepared::prepare(
                    super::workflow_prepared::PreparationServices {
                        hasher: self.hasher.clone(),
                        processor: &self.processor,
                        validator: self.validator.as_ref(),
                        limits: &self.limits,
                    },
                    actor,
                    case_id,
                    command,
                    *material,
                )?;
                Ok(Loaded::Ready(Box::new(prepared)))
            }
            PrecautionaryHearingRecordPreparation::Replay(result) => {
                checks::replay(self.hasher.as_ref(), actor, case_id, &command, &result)?;
                Ok(Loaded::Replay(result))
            }
        }
    }

    fn finish(
        &self,
        token: &str,
        actor: &Principal,
        floor: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        checks::reauthenticate(self.identity.as_ref(), token, actor)?;
        checks::observation(self.clock.as_ref(), Some(floor))?;
        Ok(())
    }

    pub fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: PrecautionaryHearingCommand,
    ) -> Result<PrecautionaryHearingReview, ApplicationError> {
        let actor = checks::actor(self.identity.as_ref(), token)?;
        let started = checks::observation(self.clock.as_ref(), None)?;
        let loaded = self.load(&actor, case_id, command)?;
        self.finish(token, &actor, started.max(loaded.earliest()))?;
        Ok(loaded.review().clone())
    }

    pub fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: PrecautionaryHearingCommand,
        confirmation: PrecautionaryHearingConfirmation,
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError> {
        let actor = checks::actor(self.identity.as_ref(), token)?;
        let started = checks::observation(self.clock.as_ref(), None)?;
        let loaded = self.load(&actor, case_id, command.clone())?;
        checks::confirmation(loaded.review(), confirmation)?;
        let mut observed = started;
        let result = match loaded {
            Loaded::Replay(result) => *result,
            Loaded::Ready(mut prepared) => {
                checks::reauthenticate(self.identity.as_ref(), token, &actor)?;
                observed = checks::observation(
                    self.clock.as_ref(),
                    Some(started.max(prepared.checked.earliest_capture)),
                )?;
                prepared.checked.earliest_capture = observed;
                let expected = checks::CommitExpectation::new(&prepared);
                let result = self.store.commit(&actor, case_id, *prepared)?;
                checks::replay(self.hasher.as_ref(), &actor, case_id, &command, &result)?;
                expected.matches(&result)?;
                result
            }
        };
        self.finish(token, &actor, observed.max(result.capture.recorded_at))?;
        Ok(result)
    }
}

impl PrecautionaryHearingRecordWorkflow for PrecautionaryHearingRecordService {
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: PrecautionaryHearingCommand,
    ) -> Result<PrecautionaryHearingReview, ApplicationError> {
        PrecautionaryHearingRecordService::prepare(self, token, case_id, command)
    }
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: PrecautionaryHearingCommand,
        confirmation: PrecautionaryHearingConfirmation,
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError> {
        PrecautionaryHearingRecordService::submit(self, token, case_id, command, confirmation)
    }
}
