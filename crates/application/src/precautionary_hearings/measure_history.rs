use super::{capture_validation::*, measure_evidence::check_captures, *};
use crate::{precautionary_measures::MeasureHistoryEvidence, ApplicationError};
use domain::crypto::DocumentHasher;
use std::collections::BTreeSet;

pub fn precautionary_hearing_receipt_with_measure_history_matches(
    hasher: &dyn DocumentHasher,
    capture: &PrecautionaryHearingCapture,
    evidence: &MeasureHistoryEvidence,
) -> Result<(), ApplicationError> {
    check_captures(hasher, &[capture], evidence)
}

pub fn precautionary_hearing_transition_with_measure_history_matches(
    hasher: &dyn DocumentHasher,
    previous: &PrecautionaryHearingCapture,
    next: &PrecautionaryHearingCapture,
    evidence: &MeasureHistoryEvidence,
) -> Result<(), ApplicationError> {
    check_captures(hasher, &[previous, next], evidence)?;
    transition(previous, &next.review)?;
    if next.recorded_at < previous.recorded_at {
        return Err(invalid("capture predates its predecessor"));
    }
    Ok(())
}

pub fn precautionary_hearing_origin_with_measure_history(
    hasher: &dyn DocumentHasher,
    capture: &PrecautionaryHearingCapture,
    evidence: &MeasureHistoryEvidence,
) -> Result<PrecautionaryHearingOrigin, ApplicationError> {
    precautionary_hearing_receipt_with_measure_history_matches(hasher, capture, evidence)?;
    super::history::origin_metadata(capture)
}

pub fn precautionary_hearing_history_with_measure_history_matches(
    hasher: &dyn DocumentHasher,
    captures: &[PrecautionaryHearingCapture],
    origin: &PrecautionaryHearingOrigin,
    evidence: &MeasureHistoryEvidence,
) -> Result<(), ApplicationError> {
    let first = captures
        .first()
        .ok_or_else(|| invalid("history lacks its initial capture"))?;
    if super::history::origin_metadata(first)? != *origin {
        return Err(invalid("history differs from original capture"));
    }
    check_captures(hasher, &captures.iter().collect::<Vec<_>>(), evidence)?;
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
