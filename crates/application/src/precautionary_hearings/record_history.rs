use super::{
    capture_validation::*,
    record_evidence::{check_captures, shape},
    *,
};
use crate::{
    measure_corrections::{MeasureRecordHistoryEvidence, RecordHistoryView},
    ApplicationError,
};
use domain::crypto::DocumentHasher;
use std::collections::BTreeSet;

pub fn precautionary_hearing_receipt_with_record_history_matches(
    hasher: &dyn DocumentHasher,
    capture: &PrecautionaryHearingCapture,
    evidence: &MeasureRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    check_captures(hasher, &[capture], evidence.into())
}

pub fn precautionary_hearing_transition_with_record_history_matches(
    hasher: &dyn DocumentHasher,
    previous: &PrecautionaryHearingCapture,
    next: &PrecautionaryHearingCapture,
    evidence: &MeasureRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    transition_with_view(hasher, previous, next, evidence.into())
}
pub(super) fn transition_with_view(
    hasher: &dyn DocumentHasher,
    previous: &PrecautionaryHearingCapture,
    next: &PrecautionaryHearingCapture,
    evidence: RecordHistoryView<'_>,
) -> Result<(), ApplicationError> {
    check_captures(hasher, &[previous, next], evidence)?;
    transition(previous, &next.review)?;
    if next.recorded_at < previous.recorded_at {
        return Err(invalid("capture predates its predecessor"));
    }
    Ok(())
}

pub fn precautionary_hearing_origin_with_record_history(
    hasher: &dyn DocumentHasher,
    capture: &PrecautionaryHearingCapture,
    evidence: &MeasureRecordHistoryEvidence,
) -> Result<PrecautionaryHearingOrigin, ApplicationError> {
    precautionary_hearing_receipt_with_record_history_matches(hasher, capture, evidence)?;
    super::history::origin_metadata(capture)
}

pub fn precautionary_hearing_history_with_record_history_matches(
    hasher: &dyn DocumentHasher,
    captures: &[PrecautionaryHearingCapture],
    origin: &PrecautionaryHearingOrigin,
    evidence: &MeasureRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    history_with_view(hasher, captures, origin, evidence.into())
}
pub(super) fn history_with_view(
    hasher: &dyn DocumentHasher,
    captures: &[PrecautionaryHearingCapture],
    origin: &PrecautionaryHearingOrigin,
    evidence: RecordHistoryView<'_>,
) -> Result<(), ApplicationError> {
    if captures.len() > 256 {
        return Err(invalid("appointment history budget exceeded"));
    }
    for capture in captures {
        shape(capture)?;
    }
    let first = captures
        .first()
        .ok_or_else(|| invalid("history lacks its initial capture"))?;
    if super::history::origin_metadata(first)? != *origin {
        return Err(invalid("history differs from original capture"));
    }
    check_captures(hasher, &captures.iter().collect::<Vec<_>>(), evidence)?;
    sequence(captures, origin)
}

fn sequence(
    captures: &[PrecautionaryHearingCapture],
    origin: &PrecautionaryHearingOrigin,
) -> Result<(), ApplicationError> {
    let first = captures
        .first()
        .ok_or_else(|| invalid("history lacks its initial capture"))?;
    if super::history::origin_metadata(first)? != *origin {
        return Err(invalid("history differs from original capture"));
    }
    let mut operations = BTreeSet::new();
    for (index, capture) in captures.iter().enumerate() {
        if !operations.insert(capture.review.command.operation_id.as_uuid()) {
            return Err(invalid("operation identity recurs in history"));
        }
        if index > 0 {
            let previous = &captures[index - 1];
            transition(previous, &capture.review)?;
            if capture.recorded_at < previous.recorded_at {
                return Err(invalid("capture predates its predecessor"));
            }
        }
    }
    Ok(())
}

/// Verifies hearing prefixes using an already reconstructed shared record forest.
pub(crate) fn check_history_with_records<'a>(
    hasher: &dyn DocumentHasher,
    case: domain::cases::CaseId,
    captures: &'a [PrecautionaryHearingCapture],
    origin: &PrecautionaryHearingOrigin,
    inventory: &mut source_inventory::SourceInventory<'a>,
    lookup: &impl Fn(
        domain::precautionary_hearings::PrecautionaryMeasureRef,
    ) -> Result<crate::measure_corrections::RecordView<'a>, ApplicationError>,
) -> Result<(), ApplicationError> {
    sequence(captures, origin)?;
    for capture in captures {
        inventory.capture(capture)?;
        receipt_flat(hasher, capture)?;
        if capture.review.case_id != case
            || capture.recorded_at
                < super::record_evidence::target_clock_with_lookup(&capture.review, lookup)?
        {
            return Err(invalid("appointment case or record chronology differs"));
        }
    }
    Ok(())
}
