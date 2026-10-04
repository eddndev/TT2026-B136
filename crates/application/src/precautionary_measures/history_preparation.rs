use super::{
    decision_wire::{bounded, invalid},
    history_inventory, *,
};
use crate::{
    identity::Principal, precautionary_hearings::source_inventory::SourceInventory,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher, precautionary_measures::MeasureEffect};

/// Prepares effects using the complete supplied ancestry; the store still proves live access and heads.
pub fn prepare_measure_decision_with_history(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureDecisionCommand,
    mut material: MeasureDecisionMaterial,
    evidence: &MeasureHistoryEvidence,
) -> Result<CheckedMeasureDecisionReview, ApplicationError> {
    bounded(material.predecessors.len())?;
    bounded(material.result_sources.len())?;
    if command.anchor.is_some() || material.anchor.is_some() {
        return Err(invalid("linked hearing evidence is not supported"));
    }
    history_inventory::limits(evidence, 1, command.outcome.affected_ids().len())?;
    let selections = super::effect_resolution::selections(&command.outcome);
    let targets = resolve_measure_targets(hasher, case_id, &selections, evidence)?;
    material
        .predecessors
        .sort_by_key(|item| item.capture.result.id.as_uuid());
    if material.predecessors != targets.targets {
        return Err(invalid("supplied predecessors differ from owning groups"));
    }
    check_new_identities(&command, &targets.groups)?;
    let direct: Vec<_> = targets
        .groups
        .iter()
        .copied()
        .filter(|group| {
            material
                .predecessors
                .iter()
                .any(|previous| previous.owner.operation_id == group.review.command.operation_id)
        })
        .collect();
    check_prior_contexts(&material, &direct)?;
    let checked =
        super::decision_preparation::prepare_flat(hasher, actor, case_id, command, material)?;
    for result in &checked.review().results {
        if targets.groups.iter().any(|group| {
            group.measures.iter().any(|member| {
                member.result.id == result.id && member.result.revision == result.revision
            })
        }) {
            return Err(invalid("result revision already belongs to another group"));
        }
    }
    let mut inventory = SourceInventory::default();
    for group in &targets.groups {
        history_inventory::add_sources(&mut inventory, &group.review)?;
    }
    history_inventory::add_sources(&mut inventory, checked.review())?;
    Ok(checked)
}

pub(super) fn check_prior_contexts(
    material: &MeasureDecisionMaterial,
    groups: &[&MeasureDecisionGroupCapture],
) -> Result<(), ApplicationError> {
    for group in groups {
        crate::precautionary_hearings::capture_validation::context_advances(
            &group.review.material.context,
            &material.context,
        )?;
    }
    Ok(())
}

pub(super) fn check_new_identities(
    command: &MeasureDecisionCommand,
    groups: &[&MeasureDecisionGroupCapture],
) -> Result<(), ApplicationError> {
    for group in groups {
        if command.operation_id == group.review.command.operation_id
            || command.decision_id == group.review.command.decision_id
        {
            return Err(invalid(
                "decision or operation identity already occurs in ancestry",
            ));
        }
    }
    let mut new_ids = Vec::new();
    for effect in command.outcome.changes().unwrap_or(&[]) {
        match effect {
            MeasureEffect::Impose(proposal) => new_ids.push(proposal.id),
            MeasureEffect::Substitute { successors, .. } => {
                new_ids.extend(successors.iter().map(|item| item.id))
            }
            _ => {}
        }
    }
    if groups.iter().any(|group| {
        group
            .measures
            .iter()
            .any(|member| new_ids.contains(&member.result.id))
    }) {
        return Err(invalid("new measure identity already occurs in ancestry"));
    }
    Ok(())
}
