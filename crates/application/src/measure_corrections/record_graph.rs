use super::{
    record_index::{HistoryView, Member, RecordIndex},
    wire::invalid,
};
use crate::{
    precautionary_hearings::source_inventory::SourceInventory, precautionary_measures::*,
    ApplicationError,
};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryMeasureRef,
};

pub(super) fn validate<'a>(
    hasher: &dyn DocumentHasher,
    case: CaseId,
    selections: &[PrecautionaryMeasureRef],
    evidence: HistoryView<'a>,
    reserve: usize,
    inventory: &mut SourceInventory<'a>,
) -> Result<RecordIndex<'a>, ApplicationError> {
    if selections.len() > 8192 {
        return Err(invalid("record target union budget exceeded"));
    }
    let index = RecordIndex::new(case, evidence, reserve)?;
    let roots = selections
        .iter()
        .map(|r| index.selected(*r).map(|m| index.owner(m)))
        .collect::<Result<Vec<_>, _>>()?;
    reconstruct(hasher, case, index, roots, &[], inventory)
}

pub(super) fn validate_forest<'a>(
    hasher: &dyn DocumentHasher,
    case: CaseId,
    index: RecordIndex<'a>,
    additional: &[Vec<usize>],
    inventory: &mut SourceInventory<'a>,
) -> Result<RecordIndex<'a>, ApplicationError> {
    let roots = (0..index.owners()).collect();
    reconstruct(hasher, case, index, roots, additional, inventory)
}

fn reconstruct<'a>(
    hasher: &dyn DocumentHasher,
    case: CaseId,
    index: RecordIndex<'a>,
    roots: Vec<usize>,
    additional: &[Vec<usize>],
    inventory: &mut SourceInventory<'a>,
) -> Result<RecordIndex<'a>, ApplicationError> {
    let evidence = index.evidence;
    let mut state = vec![0u8; index.owners()];
    let mut order = Vec::new();
    for root in roots {
        let mut stack = vec![(root, false)];
        while let Some((owner, finish)) = stack.pop() {
            if finish {
                state[owner] = 2;
                order.push(owner);
                continue;
            }
            if state[owner] == 2 {
                continue;
            }
            if state[owner] == 1 {
                return Err(invalid("cyclic record owner dependencies"));
            }
            state[owner] = 1;
            stack.push((owner, true));
            let mut parents = dependencies(&index, owner)?;
            if let Some(extra) = additional.get(owner) {
                parents.extend_from_slice(extra);
            }
            for parent in parents.into_iter().rev() {
                stack.push((parent, false));
            }
        }
    }
    if order.len() != index.owners() {
        return Err(invalid("unreachable extra record evidence"));
    }
    let mut legacy = Vec::new();
    for owner in order {
        if owner < evidence.judicial.groups.len() {
            let g = &evidence.judicial.groups[owner].capture;
            index.decision_candidate(&g.review.command, Some(owner))?;
            let targets = record_anchor_selections(&g.review.material.anchor)
                .iter()
                .map(|r| legacy_owned(&index, *r))
                .collect::<Result<Vec<_>, _>>()?;
            for prior in &g.review.material.predecessors {
                if legacy_owned(
                    &index,
                    PrecautionaryMeasureRef::new(
                        prior.capture.result.id,
                        prior.capture.result.revision,
                        prior.capture.capture_digest,
                    ),
                )? != *prior
                {
                    return Err(invalid("legacy predecessor differs from its owner"));
                }
            }
            validate_legacy_record_node(hasher, g, &legacy, targets)?;
            add_measure_group_sources(inventory, &g.review)?;
            legacy.push(g);
        } else if owner < evidence.judicial.groups.len() + evidence.decisions.len() {
            let g = &evidence.decisions[owner - evidence.judicial.groups.len()].capture;
            index.decision_candidate(&g.review.command, Some(owner))?;
            let checked = super::decision_history::prepare_node(
                hasher,
                &index,
                &g.review.actor,
                case,
                g.review.command.clone(),
                g.review.material.clone(),
            )?;
            if checked.into_group_capture(hasher, g.recorded_at)? != *g {
                return Err(invalid(
                    "decision owner differs from complete reconstruction",
                ));
            }
            super::decision_history::add_sources(inventory, &g.review)?;
        } else {
            let a = &evidence.administrative
                [owner - evidence.judicial.groups.len() - evidence.decisions.len()]
            .capture;
            let previous = index.view(index.selected(a.review.command.target)?)?;
            let checked = super::preparation::prepare_from_record(
                hasher,
                &a.review.actor,
                case,
                a.review.command.clone(),
                a.review.context.clone(),
                previous,
            )?;
            if checked.into_capture(hasher, a.recorded_at)? != *a {
                return Err(invalid(
                    "administrative owner differs from complete reconstruction",
                ));
            }
            super::record_history::add_sources(inventory, &a.review)?;
        }
    }
    Ok(index)
}
fn legacy_owned(
    index: &RecordIndex<'_>,
    r: PrecautionaryMeasureRef,
) -> Result<OwnedMeasureMaterial, ApplicationError> {
    let Member::Judicial(g, m) = index.selected(r)? else {
        return Err(invalid(
            "legacy judicial dependency selects another capture family",
        ));
    };
    let g = &index.evidence.judicial.groups[g].capture;
    Ok(OwnedMeasureMaterial {
        owner: MeasureGroupRef {
            operation_id: g.review.command.operation_id,
            decision_id: g.review.command.decision_id,
            group_digest: g.capture_digest,
        },
        capture: g.measures[m].clone(),
    })
}
fn dependencies(index: &RecordIndex<'_>, owner: usize) -> Result<Vec<usize>, ApplicationError> {
    let e = index.evidence;
    let mut refs = Vec::new();
    if owner < e.judicial.groups.len() {
        let m = &e.judicial.groups[owner].capture.review.material;
        refs.extend_from_slice(record_anchor_selections(&m.anchor));
        refs.extend(m.predecessors.iter().map(|p| {
            PrecautionaryMeasureRef::new(
                p.capture.result.id,
                p.capture.result.revision,
                p.capture.capture_digest,
            )
        }));
        for r in &refs {
            if !matches!(index.selected(*r)?, Member::Judicial(..)) {
                return Err(invalid("legacy dependency family differs"));
            }
        }
    } else if owner < e.judicial.groups.len() + e.decisions.len() {
        let g = &e.decisions[owner - e.judicial.groups.len()].capture;
        refs.extend(record_effect_selections(&g.review.command.outcome));
        refs.extend_from_slice(record_anchor_selections(&g.review.material.anchor));
    } else {
        let a = &e.administrative[owner - e.judicial.groups.len() - e.decisions.len()].capture;
        index.judicial(&a.review.result.last_judicial)?;
        if a.records[0].result.last_judicial != a.review.result.last_judicial {
            return Err(invalid("administrative judicial claims differ"));
        }
        refs.push(a.review.command.target);
        refs.push(a.review.result.last_judicial.reference);
    }
    refs.into_iter()
        .map(|r| index.selected(r).map(|m| index.owner(m)))
        .collect()
}

/// Extracts the old exact record closure from an already checked forest.
pub(super) fn extract_closure(
    index: &RecordIndex<'_>,
    target: PrecautionaryMeasureRef,
) -> Result<MeasureDecisionRecordHistoryEvidence, ApplicationError> {
    let mut owners = std::collections::BTreeSet::new();
    let mut stack = vec![index.owner(index.selected(target)?)];
    while let Some(owner) = stack.pop() {
        if owners.insert(owner) {
            stack.extend(dependencies(index, owner)?);
        }
    }
    // Reserve the candidate before cloning already bounded owner material.
    if owners.len() >= 256 {
        return Err(invalid("combined owner budget exceeded"));
    }
    let evidence = index.evidence;
    let mut result = MeasureDecisionRecordHistoryEvidence {
        records: super::MeasureRecordHistoryEvidence {
            judicial: MeasureHistoryEvidence { groups: Vec::new() },
            administrative: Vec::new(),
        },
        decisions: Vec::new(),
    };
    for owner in owners {
        if owner < evidence.judicial.groups.len() {
            result
                .records
                .judicial
                .groups
                .push(evidence.judicial.groups[owner].clone());
        } else if owner < evidence.judicial.groups.len() + evidence.decisions.len() {
            result
                .decisions
                .push(evidence.decisions[owner - evidence.judicial.groups.len()].clone());
        } else {
            result.records.administrative.push(
                evidence.administrative
                    [owner - evidence.judicial.groups.len() - evidence.decisions.len()]
                .clone(),
            );
        }
    }
    super::record_bounds::bounds((&result).into(), 1)?;
    Ok(result)
}
