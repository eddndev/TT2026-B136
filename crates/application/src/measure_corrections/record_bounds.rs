use super::{record_index::HistoryView, wire::invalid};
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
        if a.capture.records.len() != 1 {
            return Err(invalid("administrative owner must contain exactly one row"));
        }
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
