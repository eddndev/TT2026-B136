use super::*;
use crate::ApplicationError;
use domain::crypto::DocumentHasher;

/// The graph coordinator supplies only complete parents reconstructed earlier.
pub(crate) fn validate_legacy_record_node(
    hasher: &dyn DocumentHasher,
    group: &MeasureDecisionGroupCapture,
    groups: &[&MeasureDecisionGroupCapture],
    targets: Vec<OwnedMeasureMaterial>,
) -> Result<(), ApplicationError> {
    let direct: Vec<_> = groups
        .iter()
        .copied()
        .filter(|g| {
            group
                .review
                .material
                .predecessors
                .iter()
                .any(|p| p.owner.operation_id == g.review.command.operation_id)
        })
        .collect();
    super::history_preparation::check_prior_contexts(&group.review.material, &direct)?;
    super::history_preparation::check_new_identities(&group.review.command, groups)?;
    let proof = CheckedMeasureTargets {
        targets,
        groups: groups.to_vec(),
    };
    let checked = super::decision_preparation::prepare_flat(
        hasher,
        &group.review.actor,
        group.review.case_id,
        group.review.command.clone(),
        group.review.material.clone(),
        &proof,
    )?;
    if checked.into_group_capture(hasher, group.recorded_at)? != *group {
        return Err(super::decision_wire::invalid(
            "legacy owner differs from complete reconstruction",
        ));
    }
    Ok(())
}
