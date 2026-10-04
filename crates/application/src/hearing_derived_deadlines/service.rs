use super::*;
use crate::{
    deadlines::DeadlineChange,
    documents::{DocumentFormatBatchValidator, DocumentProcessor, StageSupportReadLimits},
    hearing_results::{
        draft_from_prepared, support_error, HearingResultAdmission, HearingResultChange,
    },
    identity::IdentityWorkflow,
};
use domain::{cases::CaseId, clock::Clock, crypto::DocumentHasher, identity::Permission};
use std::sync::Arc;

pub struct HearingDerivedDeadlineService {
    store: Arc<dyn HearingDerivedDeadlineStore>,
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
    limits: StageSupportReadLimits,
}

impl HearingDerivedDeadlineService {
    pub fn new(
        store: Arc<dyn HearingDerivedDeadlineStore>,
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
        if !actor.role.allows(Permission::ManageHearingResult)
            || !actor.role.allows(Permission::ManageDeadline)
        {
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

    fn admit(
        &self,
        actor: &Principal,
        case: CaseId,
        command: HearingDerivedDeadlineCommand,
        inputs: HearingDerivedDeadlineInputs,
    ) -> Result<PreparedHearingDerivedDeadline, ApplicationError> {
        if inputs.actor != *actor {
            return Err(ApplicationError::InvalidSession);
        }
        let result = HearingResultAdmission {
            processor: self.processor.as_ref(),
            validator: self.validator.as_ref(),
            hasher: self.hasher.as_ref(),
            clock: self.clock.as_ref(),
            limits: &self.limits,
        }
        .prepare(actor.id, case, command.result.clone(), inputs.result)
        .map_err(support_error)?;
        let material = HearingDerivedDeadlineMaterial {
            result: draft_from_prepared(actor.id, &result)?,
            profile: inputs.profile,
            profile_head: inputs.profile_head,
            calendar: inputs.calendar,
            calendar_head: inputs.calendar_head,
            responsible: inputs.responsible,
        };
        let draft = prepare_hearing_derived_deadline(
            self.hasher.as_ref(),
            actor,
            case,
            command,
            material,
            self.clock.now(),
        )?;
        Ok(PreparedHearingDerivedDeadline { draft, result })
    }

    fn replay(
        &self,
        actor: &Principal,
        case: CaseId,
        command: &HearingDerivedDeadlineCommand,
        record: HearingDerivedDeadlineRecord,
    ) -> Result<HearingDerivedDeadlineRecord, ApplicationError> {
        let mut evidence = record.evidence().clone();
        if evidence.actor.id != actor.id || evidence.result.snapshot.case_id != case {
            return Err(DeadlineError::OperationConflict.into());
        }
        // Retain original authority and calculation, while binding every incoming
        // command field through canonical verification rather than value equality.
        evidence.command = command.clone();
        let restored = restore_hearing_derived_deadline(self.hasher.as_ref(), evidence)?;
        if restored.review_bytes() != record.review_bytes()
            || restored.capture_bytes() != record.capture_bytes()
        {
            return Err(DeadlineError::SubmissionMismatch.into());
        }
        if record.evidence().deadline.recorded_at > self.clock.now() {
            return Err(invalid("derived.replay.future"));
        }
        Ok(record)
    }
}

impl HearingDerivedDeadlineWorkflow for HearingDerivedDeadlineService {
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        command: HearingDerivedDeadlineCommand,
    ) -> Result<HearingDerivedDeadlineReview, ApplicationError> {
        let actor = self.actor(token)?;
        initial_command(&command)?;
        let review = match self.store.prepare(actor.id, case, &command, &self.limits)? {
            HearingDerivedDeadlinePreparation::Ready(inputs) => {
                let prepared = self.admit(&actor, case, command, *inputs)?;
                HearingDerivedDeadlineReview::Ready(Box::new(prepared.draft))
            }
            HearingDerivedDeadlinePreparation::Replay(record) => {
                HearingDerivedDeadlineReview::Replay(Box::new(
                    self.replay(&actor, case, &command, *record)?,
                ))
            }
        };
        self.reauthenticate(token, &actor)?;
        Ok(review)
    }

    fn submit(
        &self,
        token: &str,
        case: CaseId,
        command: HearingDerivedDeadlineCommand,
        expected_review_digest: Sha256Digest,
    ) -> Result<HearingDerivedDeadlineRecord, ApplicationError> {
        let actor = self.actor(token)?;
        initial_command(&command)?;
        match self.store.prepare(actor.id, case, &command, &self.limits)? {
            HearingDerivedDeadlinePreparation::Replay(record) => {
                let record = self.replay(&actor, case, &command, *record)?;
                require_review(&record, expected_review_digest)?;
                self.reauthenticate(token, &actor)?;
                Ok(record)
            }
            HearingDerivedDeadlinePreparation::Ready(inputs) => {
                let prepared = self.admit(&actor, case, command.clone(), *inputs)?;
                prepared.draft.require_review(expected_review_digest)?;
                self.reauthenticate(token, &actor)?;
                let record = self.store.commit(actor.id, case, prepared)?;
                let record = self.replay(&actor, case, &command, record)?;
                require_review(&record, expected_review_digest)?;
                self.reauthenticate(token, &actor)?;
                Ok(record)
            }
        }
    }
}

fn initial_command(command: &HearingDerivedDeadlineCommand) -> Result<(), ApplicationError> {
    if !matches!(command.result.change, HearingResultChange::Record { .. }) {
        return Err(invalid("derived.result.action"));
    }
    let (deadline, policies) = command.deadline.clone().into_parts();
    if !matches!(deadline.change, DeadlineChange::Register { .. }) || policies.is_none() {
        return Err(invalid("derived.deadline.action"));
    }
    Ok(())
}

fn require_review(
    record: &HearingDerivedDeadlineRecord,
    expected: Sha256Digest,
) -> Result<(), ApplicationError> {
    if record.evidence().review_digest != expected {
        return Err(DeadlineError::SubmissionMismatch.into());
    }
    Ok(())
}
