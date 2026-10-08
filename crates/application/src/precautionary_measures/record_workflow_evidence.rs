use super::{workflow_evidence::invalid, *};
use crate::ApplicationError;
use domain::crypto::DocumentHasher;

pub(super) fn limits(
    history: &MeasureDecisionRecordHistoryEvidence,
    extra_owners: usize,
    extra_rows: usize,
) -> Result<(), ApplicationError> {
    crate::measure_corrections::record_history_bounds(history.into())?;
    let owners = history
        .records
        .judicial
        .groups
        .len()
        .checked_add(history.records.administrative.len())
        .and_then(|n| n.checked_add(history.decisions.len()))
        .and_then(|n| n.checked_add(extra_owners))
        .ok_or_else(|| invalid("mixed decision owner count overflows"))?;
    let rows = history
        .records
        .judicial
        .groups
        .iter()
        .map(|g| g.capture.measures.len())
        .chain(history.decisions.iter().map(|g| g.capture.measures.len()))
        .chain(
            history
                .records
                .administrative
                .iter()
                .map(|a| a.capture.records.len()),
        )
        .try_fold(extra_rows, |sum, rows| sum.checked_add(rows))
        .ok_or_else(|| invalid("mixed decision member count overflows"))?;
    if owners > 256 || rows > 8192 {
        return Err(invalid(
            "candidate and ancestors exceed the mixed decision budget",
        ));
    }
    Ok(())
}

pub(super) fn operation(
    hasher: &dyn DocumentHasher,
    value: &MeasureDecisionRecordStoredOperation,
) -> Result<(), ApplicationError> {
    if value.origin != measure_group_origin_v2(hasher, &value.group, &value.record_history)? {
        return Err(invalid("stored V2 origin differs from its complete group"));
    }
    Ok(())
}

pub(super) fn receipt(
    hasher: &dyn DocumentHasher,
    value: &MeasureDecisionRecordReceipt,
) -> Result<(), ApplicationError> {
    match value {
        MeasureDecisionRecordReceipt::V1(value) => {
            super::workflow_evidence::operation(hasher, value)
        }
        MeasureDecisionRecordReceipt::V2(value) => operation(hasher, value),
    }
}

/// Compares complete validated closures independently of their transport order.
pub(super) fn history_matches(
    a: &MeasureDecisionRecordHistoryEvidence,
    b: &MeasureDecisionRecordHistoryEvidence,
) -> bool {
    let mut ga: Vec<_> = a.decisions.iter().collect();
    let mut gb: Vec<_> = b.decisions.iter().collect();
    ga.sort_by_key(|g| g.origin.operation_id.as_uuid());
    gb.sort_by_key(|g| g.origin.operation_id.as_uuid());
    let mut aa: Vec<_> = a.records.administrative.iter().collect();
    let mut ab: Vec<_> = b.records.administrative.iter().collect();
    aa.sort_by_key(|a| a.origin.operation_id.as_uuid());
    ab.sort_by_key(|a| a.origin.operation_id.as_uuid());
    super::workflow_evidence::history_matches(&a.records.judicial, &b.records.judicial)
        && ga == gb
        && aa == ab
}
