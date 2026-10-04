use super::*;
use crate::ApplicationError;
use domain::{clock::OffsetDateTime, crypto::DocumentHasher};

pub(super) fn invalid(message: &str) -> ApplicationError {
    MeasureDecisionError::StoredInconsistent(message.into()).into()
}

pub(super) fn clock(at: OffsetDateTime) -> Result<(), ApplicationError> {
    if at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
        return Err(invalid("service clock must use supported UTC"));
    }
    Ok(())
}

pub(super) fn operation(
    hasher: &dyn DocumentHasher,
    result: &MeasureDecisionStoredOperation,
) -> Result<(), ApplicationError> {
    if result.origin != measure_group_origin(hasher, &result.group, &result.measure_history)? {
        return Err(invalid("stored group origin differs from complete capture"));
    }
    Ok(())
}

/// Compare complete validated closures independently of transport order.
pub(super) fn history_matches(a: &MeasureHistoryEvidence, b: &MeasureHistoryEvidence) -> bool {
    let mut a: Vec<_> = a.groups.iter().collect();
    let mut b: Vec<_> = b.groups.iter().collect();
    a.sort_by_key(|g| g.origin.operation_id.as_uuid());
    b.sort_by_key(|g| g.origin.operation_id.as_uuid());
    a == b
}
