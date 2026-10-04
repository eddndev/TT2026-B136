use super::{validation_receipt::validate_attendees, *};
use crate::{cases::case_administration_digest, ApplicationError};
use domain::{
    case_administration::CaseAdministrativeStatus, cases::CaseId, crypto::DocumentHasher,
    identity::UserId,
};
use time::OffsetDateTime;

/// Check a prospective Record envelope without inventing a persisted capture.
/// The service still resolves authorized sources and admits actual support bytes.
pub(crate) fn validate_record_draft(
    hasher: &dyn DocumentHasher,
    actor: UserId,
    case_id: CaseId,
    command: &HearingResultCommand,
    draft: &HearingResultDraft,
    observed_at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    let HearingResultChange::Record {
        anchor_revision,
        continuation,
        values,
    } = &command.change
    else {
        return Err(HearingResultError::InvalidReference.into());
    };
    let invalid =
        || HearingResultError::StoredInconsistent("prospective result material differs".into());
    let digest = hearing_result_values_digest(hasher, values);
    if draft.case_id != case_id
        || draft.actor != actor
        || &draft.command != command
        || draft.result_revision != HearingResultRevision::initial()
        || &draft.values != values
        || hearing_result_values_digest(hasher, &draft.values) != digest
        || draft.values_digest != digest
        || draft.anchor.reference.hearing_id != command.hearing_id
        || draft.anchor.reference.revision != *anchor_revision
        || continuation.map(|v| (v.id(), v.revision()))
            != draft
                .continuation
                .map(|v| (v.reference.result_id, v.reference.revision))
        || draft
            .continuation
            .is_some_and(|v| v.reference.result_id == command.result_id)
        || hearing_result_submission_digest(
            hasher,
            actor,
            case_id,
            command,
            &draft.anchor.reference,
            draft.continuation.as_ref().map(|v| &v.reference),
            digest,
        ) != draft.submission_digest
    {
        return Err(invalid().into());
    }
    if values.event_time().lower_bound() > observed_at {
        return Err(HearingResultError::FutureTime.into());
    }
    let admin = draft
        .observed_administration
        .snapshot()
        .ok_or_else(invalid)?;
    let context = draft.anchor.scheduling_context;
    if admin.case_id != case_id
        || case_administration_digest(hasher, &admin.values) != admin.values_digest
        || context.stage != draft.anchor.kind.required_stage()
        || admin.revision < context.administration_revision
        || (admin.revision == context.administration_revision
            && admin.values_digest != context.administration_digest)
    {
        return Err(invalid().into());
    }
    if admin.values.status() != CaseAdministrativeStatus::Active {
        return Err(ApplicationError::CaseClosed);
    }
    validate_attendees(case_id, values, &draft.attendees)?;
    match (values.provenance().support(), &draft.support) {
        (None, None) => {}
        (Some(reference), Some(support))
            if reference.reference() == support.reference
                && reference.digest() == support.digest => {}
        _ => return Err(invalid().into()),
    }
    Ok(())
}
