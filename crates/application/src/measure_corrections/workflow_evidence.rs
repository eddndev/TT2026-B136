use super::*;
use crate::{precautionary_measures::MeasureDecisionRecordHistoryEvidence, ApplicationError};
use domain::{clock::OffsetDateTime, crypto::DocumentHasher};

pub(super) fn invalid(message: &str) -> ApplicationError {
    MeasureAdministrativeError::StoredInconsistent(message.into()).into()
}

pub(super) fn clock(at: OffsetDateTime) -> Result<(), ApplicationError> {
    if at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
        return Err(invalid("service clock must use supported UTC"));
    }
    Ok(())
}

pub(super) fn operation(
    hasher: &dyn DocumentHasher,
    value: &MeasureAdministrativeStoredOperation,
) -> Result<(), ApplicationError> {
    if value.origin
        != measure_administrative_origin_with_decision_history(
            hasher,
            &value.capture,
            &value.record_history,
        )?
    {
        return Err(invalid("stored operation differs from original capture"));
    }
    Ok(())
}

/// Compares complete validated closures independently of transport order.
pub(super) fn history_matches(
    a: &MeasureDecisionRecordHistoryEvidence,
    b: &MeasureDecisionRecordHistoryEvidence,
) -> bool {
    let mut ga: Vec<_> = a.records.judicial.groups.iter().collect();
    let mut gb: Vec<_> = b.records.judicial.groups.iter().collect();
    ga.sort_by_key(|g| g.origin.operation_id.as_uuid());
    gb.sort_by_key(|g| g.origin.operation_id.as_uuid());
    let mut da: Vec<_> = a.decisions.iter().collect();
    let mut db: Vec<_> = b.decisions.iter().collect();
    da.sort_by_key(|g| g.origin.operation_id.as_uuid());
    db.sort_by_key(|g| g.origin.operation_id.as_uuid());
    let mut aa: Vec<_> = a.records.administrative.iter().collect();
    let mut ab: Vec<_> = b.records.administrative.iter().collect();
    aa.sort_by_key(|a| a.origin.operation_id.as_uuid());
    ab.sort_by_key(|a| a.origin.operation_id.as_uuid());
    ga == gb && da == db && aa == ab
}
