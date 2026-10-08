use super::{
    record_index::{HistoryView, RecordIndex},
    wire::invalid,
};
use crate::{
    identity::Principal, precautionary_hearings::source_inventory::SourceInventory,
    precautionary_measures::*, ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    precautionary_hearings::PrecautionaryMeasureRef,
};

/// Prepare from the complete supplied history; live authorization and heads remain separate.
pub fn prepare_measure_decision_with_record_history(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureDecisionCommand,
    material: MeasureDecisionMaterialV2,
    evidence: &MeasureDecisionRecordHistoryEvidence,
) -> Result<CheckedMeasureDecisionReviewV2, ApplicationError> {
    super::record_bounds::material(&material)?;
    let view: HistoryView<'_> = evidence.into();
    super::record_bounds::limits(view, 1, command.outcome.affected_ids().len())?;
    let mut selections = record_effect_selections(&command.outcome);
    selections.extend_from_slice(record_anchor_selections(&material.anchor));
    let mut inventory = SourceInventory::default();
    let index =
        super::record_graph::validate(hasher, case_id, &selections, view, 0, &mut inventory)?;
    index.decision_candidate(&command, None)?;
    let checked = prepare_node(hasher, &index, actor, case_id, command, material)?;
    for r in &checked.review().results {
        index.result_available(
            PrecautionaryMeasureRef::new(r.id, r.revision, Sha256Digest::from_array([0; 32])),
            None,
        )?;
    }
    add_sources(&mut inventory, checked.review())?;
    Ok(checked)
}
pub(super) fn prepare_node(
    hasher: &dyn DocumentHasher,
    index: &RecordIndex<'_>,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureDecisionCommand,
    material: MeasureDecisionMaterialV2,
) -> Result<CheckedMeasureDecisionReviewV2, ApplicationError> {
    let predecessors = record_effect_selections(&command.outcome)
        .iter()
        .map(|r| index.view(index.selected(*r)?))
        .collect::<Result<Vec<_>, _>>()?;
    let anchors = record_anchor_selections(&material.anchor)
        .iter()
        .map(|r| index.view(index.selected(*r)?))
        .collect::<Result<Vec<_>, _>>()?;
    prepare_record_flat(
        hasher,
        actor,
        case_id,
        command,
        material,
        &predecessors,
        &anchors,
    )
}
pub fn measure_decision_group_v2_matches(
    hasher: &dyn DocumentHasher,
    group: &MeasureDecisionGroupCaptureV2,
    evidence: &MeasureDecisionRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    super::record_bounds::shape(group)?;
    super::record_bounds::limits(evidence.into(), 1, group.measures.len())?;
    let r = &group.review;
    let checked = prepare_measure_decision_with_record_history(
        hasher,
        &r.actor,
        r.case_id,
        r.command.clone(),
        r.material.clone(),
        evidence,
    )?;
    if checked.into_group_capture(hasher, group.recorded_at)? != *group {
        return Err(invalid("group differs from complete reviewed capture"));
    }
    Ok(())
}
pub fn measure_group_origin_v2(
    hasher: &dyn DocumentHasher,
    group: &MeasureDecisionGroupCaptureV2,
    evidence: &MeasureDecisionRecordHistoryEvidence,
) -> Result<MeasureGroupOrigin, ApplicationError> {
    measure_decision_group_v2_matches(hasher, group, evidence)?;
    Ok(origin(group))
}
pub(super) fn origin(g: &MeasureDecisionGroupCaptureV2) -> MeasureGroupOrigin {
    MeasureGroupOrigin {
        case_id: g.review.case_id,
        operation_id: g.review.command.operation_id,
        decision_id: g.review.command.decision_id,
        submission_digest: g.review.submission_digest,
        review_digest: g.review.review_digest,
        decision_digest: g.decision.capture_digest,
        group_digest: g.capture_digest,
    }
}
pub(super) use crate::precautionary_measures::add_record_decision_sources as add_sources;
