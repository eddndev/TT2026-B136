use super::{validation::validate_preparation, validation_receipt::inconsistent, *};
use crate::{
    case_stages::StageSupportSnapshot,
    documents::{
        DocumentFormatBatchValidator, DocumentProcessor, StageFormatPolicy, StageSupportReadLimits,
    },
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{DocumentHasher, DocumentVersionRef},
    identity::UserId,
};

/// Admit encrypted support after preparation has released its database locks.
pub(crate) struct HearingResultAdmission<'a> {
    pub processor: &'a DocumentProcessor,
    pub validator: &'a dyn DocumentFormatBatchValidator,
    pub hasher: &'a dyn DocumentHasher,
    pub clock: &'a dyn Clock,
    pub limits: &'a StageSupportReadLimits,
}

impl HearingResultAdmission<'_> {
    pub fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        command: HearingResultCommand,
        mut preparation: HearingResultPreparation,
    ) -> Result<PreparedHearingResultChange, ApplicationError> {
        command.result_revision()?;
        let values = validate_preparation(self.hasher, case, &command, &mut preparation)?;
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
                self.limits,
                self.validator,
            )?
        };
        let values_digest = hearing_result_values_digest(self.hasher, &values);
        let anchor = HearingResultAnchorSnapshot::from(&preparation.anchor).reference;
        let continuation = preparation
            .continuation
            .as_ref()
            .map(HearingResultContinuationSnapshot::from)
            .map(|value| value.reference);
        let submission_digest = hearing_result_submission_digest(
            self.hasher,
            actor,
            case,
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
}

pub(crate) fn draft_from_prepared(
    actor: UserId,
    prepared: &PreparedHearingResultChange,
) -> Result<HearingResultDraft, ApplicationError> {
    Ok(HearingResultDraft {
        case_id: prepared.preparation.case_id,
        actor,
        command: prepared.command.clone(),
        result_revision: prepared.command.result_revision()?,
        values: prepared.values.clone(),
        values_digest: prepared.values_digest,
        submission_digest: prepared.submission_digest,
        anchor: HearingResultAnchorSnapshot::from(&prepared.preparation.anchor),
        continuation: prepared
            .preparation
            .continuation
            .as_ref()
            .map(HearingResultContinuationSnapshot::from),
        observed_administration: prepared.preparation.administration.clone(),
        attendees: prepared.preparation.attendees.clone(),
        support: support(prepared)?,
    })
}

fn support(
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
