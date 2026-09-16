use super::validation::validate_preparation;
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

pub struct HearingService {
    pub(super) store: Arc<dyn HearingStore>,
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub(super) clock: Arc<dyn Clock + Send + Sync>,
    limits: StageSupportReadLimits,
}

impl HearingService {
    pub fn new(
        store: Arc<dyn HearingStore>,
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
        if self.actor(token, Permission::ManageHearing)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
    fn prepared(
        &self,
        actor: UserId,
        case_id: CaseId,
        command: HearingCommand,
    ) -> Result<PreparedHearingChange, ApplicationError> {
        command.result_revision()?;
        let mut preparation = self.store.prepare(actor, case_id, &command, &self.limits)?;
        let (values, scheduling_context) =
            validate_preparation(self.hasher.as_ref(), case_id, &command, &mut preparation)?;
        let formats = if preparation.records.is_empty() {
            Vec::new()
        } else {
            self.processor.validate_support_batch(
                &preparation.records,
                &self.limits,
                self.validator.as_ref(),
            )?
        };
        let values_digest = hearing_values_digest(self.hasher.as_ref(), &values);
        let submission_digest = hearing_submission_digest(
            self.hasher.as_ref(),
            actor,
            case_id,
            &command,
            values_digest,
        );
        Ok(PreparedHearingChange {
            command,
            preparation,
            values,
            formats,
            submission_digest,
            values_digest,
            scheduling_context,
        })
    }
    pub(super) fn prepare_command(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingCommand,
    ) -> Result<HearingDraft, ApplicationError> {
        let actor = self.actor(token, Permission::ManageHearing)?;
        let prepared = self.prepared(actor, case_id, command)?;
        self.same_actor(token, actor)?;
        Ok(HearingDraft {
            result_revision: prepared.command.result_revision()?,
            command: prepared.command,
            actor,
            submission_digest: prepared.submission_digest,
            values: prepared.values,
            values_digest: prepared.values_digest,
        })
    }
    pub(super) fn submit_command(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingCommand,
        expected: Sha256Digest,
    ) -> Result<HearingDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ManageHearing)?;
        let prepared = self.prepared(actor, case_id, command.clone())?;
        if prepared.submission_digest != expected {
            return Err(HearingError::SubmissionMismatch.into());
        }
        self.same_actor(token, actor)?;
        let result = self.store.commit(actor, case_id, prepared)?;
        hearing_receipt_matches(self.hasher.as_ref(), &result)?;
        let snapshot = &result.snapshot;
        if snapshot.case_id != case_id
            || snapshot.id != command.hearing_id
            || snapshot.recorded_by.id != actor
            || snapshot.revision != command.result_revision()?
            || snapshot.receipt.operation_id != command.operation_id
            || snapshot.receipt.submission_digest != expected
        {
            return Err(inconsistent(
                "committed receipt differs from the submitted command",
            ));
        }
        Ok(result)
    }
}

pub(super) fn support_error(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::StageSupportTooLarge => HearingError::SupportTooLarge.into(),
        ApplicationError::StageSupportFormatRejected => HearingError::SupportFormatRejected.into(),
        ApplicationError::StageSupportValidationLimit => {
            HearingError::SupportValidationLimit.into()
        }
        ApplicationError::StageSupportDigestMismatch => HearingError::SupportDigestMismatch.into(),
        ApplicationError::StageSupportChanged => HearingError::SupportChanged.into(),
        other => other,
    }
}
