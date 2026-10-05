use super::{record_index::RecordIndex, wire::invalid, *};
use crate::{precautionary_hearings::source_inventory::SourceInventory, ApplicationError};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryMeasureRef,
};
use std::collections::BTreeSet;

pub(crate) struct CheckedRecordClosure<'a> {
    index: RecordIndex<'a>,
}
impl<'a> CheckedRecordClosure<'a> {
    pub(crate) fn member(
        &self,
        reference: PrecautionaryMeasureRef,
    ) -> Result<super::record_view::RecordView<'a>, ApplicationError> {
        self.index.view(self.index.selected(reference)?)
    }
}

pub(crate) fn record_history_bounds(
    evidence: &MeasureRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    super::record_index::bounds(evidence.into(), 0)
}

pub(crate) fn checked_record_closure<'a>(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    selections: &[PrecautionaryMeasureRef],
    evidence: &'a MeasureRecordHistoryEvidence,
    inventory: &mut SourceInventory<'a>,
) -> Result<CheckedRecordClosure<'a>, ApplicationError> {
    if selections.len() > 8192 {
        return Err(invalid("record target union budget exceeded"));
    }
    Ok(CheckedRecordClosure {
        index: validate(hasher, case_id, selections, evidence.into(), 0, inventory)?,
    })
}

pub fn resolve_measure_records(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    selections: &[PrecautionaryMeasureRef],
    evidence: &MeasureRecordHistoryEvidence,
) -> Result<CheckedMeasureRecords, ApplicationError> {
    resolve_view(hasher, case_id, selections, evidence.into())
}

pub fn resolve_measure_records_with_decision_history(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    selections: &[PrecautionaryMeasureRef],
    evidence: &crate::precautionary_measures::MeasureDecisionRecordHistoryEvidence,
) -> Result<CheckedMeasureRecords, ApplicationError> {
    resolve_view(hasher, case_id, selections, evidence.into())
}
fn resolve_view(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    selections: &[PrecautionaryMeasureRef],
    evidence: super::record_index::HistoryView<'_>,
) -> Result<CheckedMeasureRecords, ApplicationError> {
    if selections.len() > 32 {
        return Err(invalid("too many selected measures"));
    }
    let mut unique = BTreeSet::new();
    for r in selections {
        if !unique.insert(r.id().as_uuid()) {
            return Err(invalid("duplicate selected measure identity"));
        }
    }
    let mut inventory = SourceInventory::default();
    let index = validate(hasher, case_id, selections, evidence, 0, &mut inventory)?;
    let mut targets = selections
        .iter()
        .map(|r| index.view(index.selected(*r)?).map(|v| v.to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    targets.sort_by_key(|r| (r.reference().id().as_uuid(), r.reference().revision().get()));
    Ok(CheckedMeasureRecords { targets })
}

pub(super) use super::record_graph::validate;

pub(super) fn add_sources<'a>(
    inventory: &mut SourceInventory<'a>,
    r: &'a MeasureAdministrativeReview,
) -> Result<(), ApplicationError> {
    inventory.context(&r.context)?;
    inventory.support(&r.support)?;
    inventory.subject(&r.result.sources.subject)?;
    if let Some(p) = &r.result.sources.supervisor {
        inventory.participant(p)?;
    }
    if let Some(p) = &r.result.projection.supervisor {
        inventory.projection(&p.overview, p.snapshot.values_digest)?;
    }
    Ok(())
}
