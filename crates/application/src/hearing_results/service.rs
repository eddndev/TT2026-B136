use super::validation_receipt::inconsistent;
use super::*;
use crate::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor, StageSupportReadLimits},
    identity::IdentityWorkflow,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{DocumentHasher, Sha256Digest},
    identity::{Permission, UserId},
};
use std::sync::Arc;
pub struct HearingResultService {
    pub(super) store: Arc<dyn HearingResultStore>,
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub(super) clock: Arc<dyn Clock + Send + Sync>,
    limits: StageSupportReadLimits,
}
impl HearingResultService {
    pub fn new(
        store: Arc<dyn HearingResultStore>,
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
    pub(super) fn actor(
        &self,
        token: &str,
        permission: Permission,
    ) -> Result<UserId, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !actor.role.allows(permission) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor.id)
    }
    fn same_actor(&self, token: &str, actor: UserId) -> Result<(), ApplicationError> {
        if self.actor(token, Permission::ManageHearingResult)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
    fn prepared(
        &self,
        actor: UserId,
        case_id: CaseId,
        command: HearingResultCommand,
    ) -> Result<PreparedHearingResultChange, ApplicationError> {
        command.result_revision()?;
        let preparation = self.store.prepare(actor, case_id, &command, &self.limits)?;
        HearingResultAdmission {
            processor: self.processor.as_ref(),
            validator: self.validator.as_ref(),
            hasher: self.hasher.as_ref(),
            clock: self.clock.as_ref(),
            limits: &self.limits,
        }
        .prepare(actor, case_id, command, preparation)
    }
    pub(super) fn prepare_command(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingResultCommand,
    ) -> Result<HearingResultDraft, ApplicationError> {
        let actor = self.actor(token, Permission::ManageHearingResult)?;
        let prepared = self.prepared(actor, case_id, command)?;
        self.same_actor(token, actor)?;
        draft_from_prepared(actor, &prepared)
    }
    pub(super) fn submit_command(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingResultCommand,
        expected: Sha256Digest,
    ) -> Result<HearingResultDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ManageHearingResult)?;
        let prepared = self.prepared(actor, case_id, command.clone())?;
        if prepared.submission_digest != expected {
            return Err(HearingResultError::SubmissionMismatch.into());
        }
        self.same_actor(token, actor)?;
        let result = self.store.commit(actor, case_id, prepared)?;
        hearing_result_receipt_matches(self.hasher.as_ref(), &result)?;
        let snapshot = &result.snapshot;
        if snapshot.case_id != case_id
            || snapshot.hearing_id != command.hearing_id
            || snapshot.id != command.result_id
            || snapshot.recorded_by.id != actor
            || snapshot.revision != command.result_revision()?
            || snapshot.receipt.operation_id != command.operation_id
            || snapshot.receipt.submission_digest != expected
        {
            return Err(inconsistent(
                "committed receipt differs from submitted command",
            ));
        }
        Ok(result)
    }
}
pub(crate) fn support_error(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::StageSupportTooLarge => HearingResultError::SupportTooLarge.into(),
        ApplicationError::StageSupportFormatRejected => {
            HearingResultError::SupportFormatRejected.into()
        }
        ApplicationError::StageSupportValidationLimit => {
            HearingResultError::SupportValidationLimit.into()
        }
        ApplicationError::StageSupportDigestMismatch => {
            HearingResultError::SupportDigestMismatch.into()
        }
        ApplicationError::StageSupportChanged => HearingResultError::SupportChanged.into(),
        other => other,
    }
}
