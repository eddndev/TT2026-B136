use super::{
    record_index::{HistoryView, Member, RecordIndex},
    wire::invalid,
    *,
};
use crate::{
    precautionary_hearings::source_inventory::SourceInventory,
    precautionary_measures::{add_measure_group_sources, resolve_measure_closure},
    ApplicationError,
};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryMeasureRef,
};
use std::collections::BTreeSet;

pub fn resolve_measure_records(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    selections: &[PrecautionaryMeasureRef],
    evidence: &MeasureRecordHistoryEvidence,
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
    let index = validate(
        hasher,
        case_id,
        selections,
        evidence.into(),
        0,
        &mut inventory,
    )?;
    let mut targets = selections
        .iter()
        .map(|r| index.view(index.selected(*r)?).map(|v| v.to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    targets.sort_by_key(|r| (r.reference().id().as_uuid(), r.reference().revision().get()));
    Ok(CheckedMeasureRecords { targets })
}

pub(super) fn validate<'a>(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    selections: &[PrecautionaryMeasureRef],
    evidence: HistoryView<'a>,
    reserve: usize,
    inventory: &mut SourceInventory<'a>,
) -> Result<RecordIndex<'a>, ApplicationError> {
    let index = RecordIndex::new(case_id, evidence, reserve)?;
    let mut judicial_roots = Vec::new();
    let mut stack = Vec::new();
    for reference in selections {
        match index.selected(*reference)? {
            Member::Judicial(_, _) => judicial_roots.push(*reference),
            Member::Administrative(a) => stack.push((a, false)),
        }
    }
    let mut state = vec![0u8; evidence.administrative.len()];
    let mut order = Vec::new();
    while let Some((position, finish)) = stack.pop() {
        if finish {
            state[position] = 2;
            order.push(position);
            continue;
        }
        if state[position] == 2 {
            continue;
        }
        if state[position] == 1 {
            return Err(invalid("cyclic administrative dependencies"));
        }
        state[position] = 1;
        stack.push((position, true));
        let a = &evidence.administrative[position].capture;
        index.judicial(&a.review.result.last_judicial)?;
        judicial_roots.push(a.review.result.last_judicial.reference);
        match index.selected(a.review.command.target)? {
            Member::Judicial(_, _) => judicial_roots.push(a.review.command.target),
            Member::Administrative(parent) => stack.push((parent, false)),
        }
    }
    if order.len() != evidence.administrative.len() {
        return Err(invalid("unreachable extra administrative evidence"));
    }
    // Legacy judicial dependencies remain judicial-only, and are reconstructed once.
    resolve_measure_closure(hasher, case_id, &judicial_roots, evidence.judicial)?;
    for g in &evidence.judicial.groups {
        add_measure_group_sources(inventory, &g.capture.review)?;
    }
    for position in order {
        let a = &evidence.administrative[position].capture;
        let previous = index.view(index.selected(a.review.command.target)?)?;
        let checked = super::preparation::prepare_from_record(
            hasher,
            &a.review.actor,
            case_id,
            a.review.command.clone(),
            a.review.context.clone(),
            previous,
        )?;
        if checked.into_capture(hasher, a.recorded_at)? != *a {
            return Err(invalid(
                "administrative owner differs from complete reconstruction",
            ));
        }
        add_sources(inventory, &a.review)?;
    }
    Ok(index)
}

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
