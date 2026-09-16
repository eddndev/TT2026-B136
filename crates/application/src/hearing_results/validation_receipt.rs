use super::*;
use crate::ApplicationError;
use domain::{cases::CaseId, crypto::DocumentHasher};
pub(super) fn inconsistent(message: &str) -> ApplicationError {
    HearingResultError::StoredInconsistent(message.into()).into()
}
/// Checks the self-contained operation envelope; storage also resolves exact sources.
pub fn hearing_result_history_receipt_matches(
    hasher: &dyn DocumentHasher,
    entry: &HearingResultHistoryEntry,
) -> Result<(), ApplicationError> {
    let receipt = &entry.receipt;
    let valid = match receipt.action {
        HearingResultAction::Record => {
            receipt.expected_revision == 0
                && entry.reason.is_none()
                && entry.status == HearingResultStatus::Recorded
        }
        HearingResultAction::Correct => {
            receipt.expected_revision > 0
                && entry.reason.is_some()
                && entry.status == HearingResultStatus::Recorded
        }
        HearingResultAction::Withdraw => {
            receipt.expected_revision > 0
                && entry.reason.is_some()
                && entry.status == HearingResultStatus::Withdrawn
        }
    };
    if !valid
        || receipt.expected_revision.checked_add(1) != Some(entry.revision.get())
        || entry.anchor.hearing_id != entry.hearing_id
        || entry.continuation.is_some_and(|p| p.result_id == entry.id)
    {
        return Err(inconsistent(
            "receipt action, revision or fixed sources disagree",
        ));
    }
    if hasher.hash_bytes(&super::canonical::history_submission_bytes(entry))
        != receipt.submission_digest
    {
        return Err(inconsistent(
            "receipt digest differs from reconstructed submission",
        ));
    }
    Ok(())
}
pub fn hearing_result_snapshot_receipt_matches(
    hasher: &dyn DocumentHasher,
    snapshot: &HearingResultSnapshot,
) -> Result<(), ApplicationError> {
    hearing_result_history_receipt_matches(hasher, &HearingResultHistoryEntry::from(snapshot))?;
    if hearing_result_values_digest(hasher, &snapshot.values) != snapshot.values_digest {
        return Err(inconsistent("value digest differs from declared session"));
    }
    if snapshot.receipt.action != HearingResultAction::Withdraw
        && snapshot.values.event_time().lower_bound() > snapshot.recorded_at
    {
        return Err(inconsistent("declared time follows its original capture"));
    }
    Ok(())
}
pub fn hearing_result_receipt_matches(
    hasher: &dyn DocumentHasher,
    detail: &HearingResultDetail,
) -> Result<(), ApplicationError> {
    let snapshot = &detail.snapshot;
    hearing_result_snapshot_receipt_matches(hasher, snapshot)?;
    if detail.anchor.reference != snapshot.anchor
        || detail.continuation.map(|v| v.reference) != snapshot.continuation
    {
        return Err(inconsistent(
            "resolved source projection differs from fixed reference",
        ));
    }
    let context = detail.anchor.scheduling_context;
    if context.stage != detail.anchor.kind.required_stage()
        || snapshot.recorded_administration_revision < context.administration_revision
        || (snapshot.recorded_administration_revision == context.administration_revision
            && snapshot.recorded_administration_digest != context.administration_digest)
    {
        return Err(inconsistent(
            "captured administration contradicts scheduling context",
        ));
    }
    validate_attendees(snapshot.case_id, &snapshot.values, &detail.attendees)?;
    match (snapshot.values.provenance().support(), &detail.support) {
        (None, None) => {}
        (Some(reference), Some(support))
            if reference.reference() == support.reference
                && reference.digest() == support.digest => {}
        _ => {
            return Err(inconsistent(
                "historical support differs from declared provenance",
            ))
        }
    }
    Ok(())
}
pub(super) fn validate_attendees(
    case_id: CaseId,
    values: &HearingResultValues,
    attendees: &[HearingResultAttendeeSnapshot],
) -> Result<(), ApplicationError> {
    if attendees.len() != values.attendees().len() {
        return Err(inconsistent("attendee projection count differs"));
    }
    for (reference, snapshot) in values.attendees().iter().zip(attendees) {
        let overview = &snapshot.participant.overview;
        if overview.case_id != case_id
            || overview.id != reference.participant_id()
            || overview.revision != reference.revision()
            || overview.kind.is_some() != overview.subject.is_some()
            || overview.subject.map(|subject| subject.values_digest) != snapshot.subject_digest
            || overview.display_name.trim().is_empty()
            || overview.procedural_role.trim().is_empty()
        {
            return Err(inconsistent(
                "attendee scope, exact identity or projection differs",
            ));
        }
    }
    Ok(())
}
