use super::{
    decision_wire::{bounded, invalid},
    history_inventory::{self, GroupIndex},
    *,
};
use crate::{precautionary_hearings::source_inventory::SourceInventory, ApplicationError};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryMeasureRef,
};
use std::collections::BTreeSet;

/// Verifies the exact supplied owning-group closure, not durable existence or current heads.
pub fn resolve_measure_targets<'a>(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    selections: &[PrecautionaryMeasureRef],
    evidence: &'a MeasureHistoryEvidence,
) -> Result<CheckedMeasureTargets<'a>, ApplicationError> {
    bounded(selections.len())?;
    let mut unique = BTreeSet::new();
    for selection in selections {
        if !unique.insert(selection.id().as_uuid()) {
            return Err(invalid("duplicate selected measure identity"));
        }
    }
    resolve_measure_closure(hasher, case_id, selections, evidence)
}

pub(crate) fn resolve_measure_closure<'a>(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    selections: &[PrecautionaryMeasureRef],
    evidence: &'a MeasureHistoryEvidence,
) -> Result<CheckedMeasureTargets<'a>, ApplicationError> {
    if selections.len() > history_inventory::MAX_ROWS {
        return Err(invalid("too many history roots"));
    }
    let index = GroupIndex::new(case_id, evidence)?;
    let mut unique = BTreeSet::new();
    let mut selected = Vec::new();
    for reference in selections {
        let key = (
            reference.id().as_uuid(),
            reference.revision().get(),
            *reference.digest().as_bytes(),
        );
        if unique.insert(key) {
            selected.push(index.selected(*reference)?);
        }
    }
    let mut state = vec![0u8; evidence.groups.len()];
    let mut order = Vec::new();
    for &(root, _) in &selected {
        let mut stack = vec![(root, false)];
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
                return Err(invalid("cyclic measure group dependencies"));
            }
            state[position] = 1;
            stack.push((position, true));
            let material = &evidence.groups[position].capture.review.material;
            for reference in super::anchor_validation::selections(&material.anchor)
                .iter()
                .rev()
            {
                stack.push((index.selected(*reference)?.0, false));
            }
            for previous in material.predecessors.iter().rev() {
                stack.push((index.dependency(previous)?, false));
            }
        }
    }
    if order.len() != evidence.groups.len() {
        return Err(invalid("unreachable extra group evidence"));
    }
    let mut inventory = SourceInventory::default();
    let mut groups = Vec::with_capacity(order.len());
    for position in order {
        let group = &evidence.groups[position].capture;
        let dependencies = dependency_groups(&index, group)?;
        super::history_preparation::check_prior_contexts(&group.review.material, &dependencies)?;
        super::history_preparation::check_new_identities(&group.review.command, &groups)?;
        let anchor_targets = super::anchor_validation::selections(&group.review.material.anchor)
            .iter()
            .map(|reference| {
                index
                    .selected(*reference)
                    .map(|(owner, row)| owned(&evidence.groups[owner].capture, row))
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Only already reconstructed parents are available to the flat anchor validator.
        let proof = CheckedMeasureTargets {
            targets: anchor_targets,
            groups: groups.clone(),
        };
        let checked = super::decision_preparation::prepare_flat(
            hasher,
            &group.review.actor,
            case_id,
            group.review.command.clone(),
            group.review.material.clone(),
            &proof,
        )?;
        if checked.into_group_capture(hasher, group.recorded_at)? != *group {
            return Err(invalid("owning group differs from complete reconstruction"));
        }
        history_inventory::add_sources(&mut inventory, &group.review)?;
        groups.push(group);
    }
    let mut targets: Vec<_> = selected
        .into_iter()
        .map(|(owner, row)| owned(&evidence.groups[owner].capture, row))
        .collect();
    targets.sort_by_key(|item| {
        (
            item.capture.result.id.as_uuid(),
            item.capture.result.revision.get(),
        )
    });
    Ok(CheckedMeasureTargets { targets, groups })
}

fn owned(group: &MeasureDecisionGroupCapture, row: usize) -> OwnedMeasureMaterial {
    OwnedMeasureMaterial {
        owner: MeasureGroupRef {
            operation_id: group.review.command.operation_id,
            decision_id: group.review.command.decision_id,
            group_digest: group.capture_digest,
        },
        capture: group.measures[row].clone(),
    }
}

fn dependency_groups<'a>(
    index: &GroupIndex<'a>,
    group: &MeasureDecisionGroupCapture,
) -> Result<Vec<&'a MeasureDecisionGroupCapture>, ApplicationError> {
    group
        .review
        .material
        .predecessors
        .iter()
        .map(|previous| {
            let owner = index.dependency(previous)?;
            Ok(&index.evidence.groups[owner].capture)
        })
        .collect()
}

pub fn measure_decision_group_with_history_matches(
    hasher: &dyn DocumentHasher,
    group: &MeasureDecisionGroupCapture,
    evidence: &MeasureHistoryEvidence,
) -> Result<(), ApplicationError> {
    history_inventory::shape(group)?;
    history_inventory::limits(evidence, 1, group.measures.len())?;
    let checked = prepare_measure_decision_with_history(
        hasher,
        &group.review.actor,
        group.review.case_id,
        group.review.command.clone(),
        group.review.material.clone(),
        evidence,
    )?;
    if checked.into_group_capture(hasher, group.recorded_at)? != *group {
        return Err(invalid("group differs from complete reviewed capture"));
    }
    Ok(())
}

pub fn measure_group_origin(
    hasher: &dyn DocumentHasher,
    group: &MeasureDecisionGroupCapture,
    evidence: &MeasureHistoryEvidence,
) -> Result<MeasureGroupOrigin, ApplicationError> {
    measure_decision_group_with_history_matches(hasher, group, evidence)?;
    Ok(super::history_model::origin(group))
}
