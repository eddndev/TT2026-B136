use super::validation::validate_preparation;
use super::validation_receipt::inconsistent;
use super::*;
use crate::{
    case_stages::StageSupportSnapshot,
    documents::{
        DocumentFormatBatchValidator, DocumentProcessor, StageFormatPolicy, StageSupportReadLimits,
    },
    identity::IdentityWorkflow,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{DocumentHasher, DocumentVersionRef, Sha256Digest},
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
        let mut preparation = self.store.prepare(actor, case_id, &command, &self.limits)?;
        let values =
            validate_preparation(self.hasher.as_ref(), case_id, &command, &mut preparation)?;
        if command.action() != HearingResultAction::Withdraw
            && values.event_time().lower_bound() > self.clock.now()
        {
            return Err(HearingResultError::FutureTime.into());
        }
        let formats = if preparation.records.is_empty() {
            vec![]
        } else {
            self.processor.validate_support_batch(
                &preparation.records,
                &self.limits,
                self.validator.as_ref(),
            )?
        };
        let values_digest = hearing_result_values_digest(self.hasher.as_ref(), &values);
        let anchor = HearingResultAnchorSnapshot::from(&preparation.anchor).reference;
        let continuation = preparation
            .continuation
            .as_ref()
            .map(HearingResultContinuationSnapshot::from)
            .map(|p| p.reference);
        let submission_digest = hearing_result_submission_digest(
            self.hasher.as_ref(),
            actor,
            case_id,
            &command,
            &anchor,
            continuation.as_ref(),
            values_digest,
        );
        Ok(PreparedHearingResultChange {
            command,
            preparation,
            values,
            formats,
            values_digest,
            submission_digest,
            anchor,
            continuation,
        })
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
        let support = prepared_support(&prepared)?;
        Ok(HearingResultDraft {
            case_id,
            actor,
            result_revision: prepared.command.result_revision()?,
            command: prepared.command,
            values: prepared.values,
            values_digest: prepared.values_digest,
            submission_digest: prepared.submission_digest,
            anchor: HearingResultAnchorSnapshot::from(&prepared.preparation.anchor),
            continuation: prepared
                .preparation
                .continuation
                .as_ref()
                .map(HearingResultContinuationSnapshot::from),
            observed_administration: prepared.preparation.administration,
            attendees: prepared.preparation.attendees,
            support,
        })
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
fn prepared_support(
    prepared: &PreparedHearingResultChange,
) -> Result<Option<StageSupportSnapshot>, ApplicationError> {
    if prepared.command.action() == HearingResultAction::Withdraw {
        return Ok(prepared
            .preparation
            .base
            .as_ref()
            .and_then(|base| base.support.clone()));
    }
    match (
        prepared.preparation.records.as_slice(),
        prepared.formats.as_slice(),
    ) {
        ([], []) => Ok(None),
        ([record], [format]) => Ok(Some(StageSupportSnapshot {
            reference: DocumentVersionRef {
                id: record.id,
                version: record.version,
            },
            digest: record.digest,
            name: record.name.clone(),
            format: *format,
            policy: StageFormatPolicy::PdfDocxV1,
        })),
        _ => Err(inconsistent("admitted support projection count differs")),
    }
}
pub(super) fn support_error(error: ApplicationError) -> ApplicationError {
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
