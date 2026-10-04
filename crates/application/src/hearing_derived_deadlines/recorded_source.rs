use super::*;
use crate::{
    deadline_inputs::DeadlineSourceDetail,
    deadline_reevaluation::{DependencyFamily, SourceEventReference},
    deadlines::evidence,
    hearing_results::*,
};
use domain::crypto::DocumentHasher;

pub(super) fn validate(
    hasher: &dyn DocumentHasher,
    draft: &HearingDerivedDeadlineDraft,
    actual: &HearingResultDetail,
    event: SourceEventReference,
) -> Result<(), ApplicationError> {
    validate_parts(
        hasher,
        draft.actor(),
        &draft.command().result,
        draft.result(),
        actual,
        event,
    )
}

pub(super) fn validate_parts(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    command: &HearingResultCommand,
    proposed: &HearingResultDraft,
    actual: &HearingResultDetail,
    event: SourceEventReference,
) -> Result<(), ApplicationError> {
    hearing_result_receipt_matches(hasher, actual)?;
    let admin = proposed
        .observed_administration
        .snapshot()
        .ok_or_else(|| invalid("derived.administration"))?;
    let at = actual.snapshot.recorded_at;
    if !(1..=9999).contains(&at.year())
        || at
            .checked_to_offset(time::UtcOffset::UTC)
            .is_none_or(|utc| !(1..=9999).contains(&utc.year()))
    {
        return Err(invalid("derived.capture.time"));
    }
    validate_record_draft(hasher, actor.id, proposed.case_id, command, proposed, at)?;
    let expected = HearingResultDetail {
        snapshot: HearingResultSnapshot {
            case_id: proposed.case_id,
            hearing_id: proposed.command.hearing_id,
            id: proposed.command.result_id,
            revision: HearingResultRevision::initial(),
            values: proposed.values.clone(),
            values_digest: proposed.values_digest,
            status: HearingResultStatus::Recorded,
            reason: None,
            receipt: HearingResultReceipt {
                operation_id: proposed.command.operation_id,
                action: HearingResultAction::Record,
                expected_revision: 0,
                submission_digest: proposed.submission_digest,
            },
            anchor: proposed.anchor.reference,
            continuation: proposed.continuation.map(|v| v.reference),
            recorded_administration_revision: admin.revision,
            recorded_administration_digest: admin.values_digest,
            recorded_at: at,
            recorded_by: crate::cases::CaseActorSnapshot {
                id: actor.id,
                email: actor.email.clone(),
            },
        },
        anchor: proposed.anchor,
        continuation: proposed.continuation,
        attendees: proposed.attendees.clone(),
        support: proposed.support.clone(),
    };
    let mut expected_bytes = Vec::new();
    let mut actual_bytes = Vec::new();
    evidence::source(
        &mut expected_bytes,
        hasher,
        Some(&DeadlineSourceDetail::HearingResult(Box::new(expected))),
    );
    evidence::source(
        &mut actual_bytes,
        hasher,
        Some(&DeadlineSourceDetail::HearingResult(Box::new(
            actual.clone(),
        ))),
    );
    if expected_bytes != actual_bytes {
        return Err(DeadlineError::SubmissionMismatch.into());
    }
    let source = &actual.snapshot;
    if event.sequence == 0
        || event.sequence > i64::MAX as u64
        || event.family != DependencyFamily::HearingResult
        || event.source_id != source.id.as_uuid()
        || event.revision != source.revision.get()
        || event.case_id != Some(source.case_id)
        || event.hearing_id != Some(source.hearing_id.as_uuid())
        || event.operation_id != source.receipt.operation_id.as_uuid()
    {
        return Err(invalid("derived.source.event"));
    }
    Ok(())
}
