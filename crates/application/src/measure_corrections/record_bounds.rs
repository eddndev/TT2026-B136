use super::{
    record_index::HistoryView, wire::invalid, MeasureAdministrativeAction,
    MeasureAdministrativeCapture, MeasureAdministrativeCommand,
};
use crate::{precautionary_measures::*, ApplicationError};

pub(super) fn bounds(e: HistoryView<'_>, reserve: usize) -> Result<(), ApplicationError> {
    limits(e, reserve, reserve)
}
pub(super) fn limits(
    e: HistoryView<'_>,
    owners: usize,
    extra_rows: usize,
) -> Result<(), ApplicationError> {
    let total = e
        .judicial
        .groups
        .len()
        .checked_add(e.administrative.len())
        .and_then(|n| n.checked_add(e.decisions.len()))
        .and_then(|n| n.checked_add(owners))
        .ok_or_else(|| invalid("owner count overflow"))?;
    if total > 256 {
        return Err(invalid("combined owner budget exceeded"));
    }
    let mut rows = extra_rows;
    for g in &e.judicial.groups {
        measure_group_shape(&g.capture)?;
        rows += g.capture.measures.len();
    }
    for g in e.decisions {
        shape(&g.capture)?;
        rows += g.capture.measures.len();
    }
    for a in e.administrative {
        administrative_shape(&a.capture)?;
        rows += a.capture.records.len();
    }
    if rows > 8192 {
        return Err(invalid("combined row budget exceeded"));
    }
    Ok(())
}
pub(super) fn material(m: &MeasureDecisionMaterialV2) -> Result<(), ApplicationError> {
    bounded(m.predecessors.len())?;
    bounded(m.result_sources.len())?;
    crate::precautionary_measures::record_anchor_shape(&m.anchor)
}
pub(super) fn shape(g: &MeasureDecisionGroupCaptureV2) -> Result<(), ApplicationError> {
    crate::precautionary_measures::record_decision_group_shape(g)
}
fn bounded(n: usize) -> Result<(), ApplicationError> {
    if n > 32 {
        Err(invalid("group material count exceeds limit"))
    } else {
        Ok(())
    }
}

pub(super) fn candidate_rows(command: &MeasureAdministrativeCommand) -> usize {
    match &command.action {
        MeasureAdministrativeAction::MarkEnteredInErrorAndReplace { .. } => 2,
        MeasureAdministrativeAction::Correct(_)
        | MeasureAdministrativeAction::MarkEnteredInError => 1,
    }
}

pub(super) fn administrative_shape(
    capture: &MeasureAdministrativeCapture,
) -> Result<(), ApplicationError> {
    let replacement = candidate_rows(&capture.review.command) == 2;
    if capture.records.len() != candidate_rows(&capture.review.command)
        || capture.review.replacement.is_some() != replacement
        || capture.replacement_link.is_some() != replacement
    {
        return Err(invalid("administrative action and owned row shape differ"));
    }
    if capture.records.windows(2).any(|rows| {
        (rows[0].result.id.as_uuid(), rows[0].result.revision)
            >= (rows[1].result.id.as_uuid(), rows[1].result.revision)
    }) {
        return Err(invalid("administrative rows are not strictly ordered"));
    }
    Ok(())
}

/// Check bounded proof shape without hashing, resolving sources or validating authority.
/// Success alone does not establish that the history is genuine or consistent.
pub fn validate_measure_record_history_shape(
    history: &MeasureDecisionRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    limits(history.into(), 0, 0)
}
