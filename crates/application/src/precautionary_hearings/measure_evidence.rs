use super::{capture_validation::*, source_inventory::SourceInventory, *};
use crate::{
    identity::Principal,
    precautionary_measures::{
        add_measure_group_sources, resolve_measure_closure, CheckedMeasureTargets,
        MeasureHistoryEvidence,
    },
    ApplicationError,
};
use domain::{
    cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher,
    precautionary_hearings::PrecautionaryMeasureRef,
};

pub struct PrecautionaryHearingPreparationMaterial<'a> {
    pub observed_context: PrecautionaryContext,
    pub sources: PrecautionaryHearingSources,
    pub predecessor: Option<&'a PrecautionaryHearingCapture>,
    pub measure_history: &'a MeasureHistoryEvidence,
}

pub fn prepare_precautionary_hearing_with_history(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: PrecautionaryHearingCommand,
    material: PrecautionaryHearingPreparationMaterial<'_>,
) -> Result<CheckedPrecautionaryHearingReview, ApplicationError> {
    let mut checked = super::capture_preparation::prepare_flat(
        hasher,
        actor,
        case_id,
        command,
        material.observed_context,
        material.sources,
        material.predecessor,
    )?;
    let mut reviews = vec![checked.review()];
    if let Some(previous) = material.predecessor {
        reviews.push(&previous.review);
    }
    let selections = target_union(reviews.iter().copied())?;
    let proof = resolve_measure_closure(hasher, case_id, &selections, material.measure_history)?;
    check_sources(&proof, &reviews)?;
    if let Some(previous) = material.predecessor {
        if previous.recorded_at < target_clock(&previous.review, &proof)? {
            return Err(invalid(
                "predecessor predates its selected measure evidence",
            ));
        }
    }
    let mut floor = checked.earliest_capture;
    for review in reviews {
        floor = floor.max(target_clock(review, &proof)?);
    }
    checked.earliest_capture = floor;
    Ok(checked)
}

pub(crate) fn target_union<'a>(
    reviews: impl IntoIterator<Item = &'a PrecautionaryHearingReview>,
) -> Result<Vec<PrecautionaryMeasureRef>, ApplicationError> {
    let mut unique = std::collections::BTreeMap::new();
    for review in reviews {
        for reference in review.resolved_values.review_targets() {
            let key = (reference.id().as_uuid(), reference.revision().get());
            if let Some(old) = unique.insert(key, *reference) {
                if old != *reference {
                    return Err(invalid("one target revision has conflicting digests"));
                }
            }
            if unique.len() > 8192 {
                return Err(invalid("measure target history exceeds bounded evidence"));
            }
        }
    }
    Ok(unique.into_values().collect())
}

pub(crate) fn target_clock(
    review: &PrecautionaryHearingReview,
    proof: &CheckedMeasureTargets<'_>,
) -> Result<OffsetDateTime, ApplicationError> {
    let mut floor = latest_source_time(review)?;
    for reference in review.resolved_values.review_targets() {
        let target = proof.member(*reference)?;
        if target.capture.case_id != review.case_id {
            return Err(invalid("review target belongs to another case"));
        }
        let group = proof
            .groups()
            .iter()
            .find(|group| group.review.command.operation_id == target.owner.operation_id)
            .ok_or_else(|| invalid("target owner context missing"))?;
        context_advances(&group.review.material.context, &review.scheduling_context)?;
        floor = floor.max(target.capture.recorded_at);
    }
    Ok(floor)
}

pub(crate) fn check_sources(
    proof: &CheckedMeasureTargets<'_>,
    reviews: &[&PrecautionaryHearingReview],
) -> Result<(), ApplicationError> {
    let mut inventory = SourceInventory::default();
    for group in proof.groups() {
        add_measure_group_sources(&mut inventory, &group.review)?;
    }
    for review in reviews {
        inventory.add(review)?;
    }
    Ok(())
}

pub(super) fn check_captures(
    hasher: &dyn DocumentHasher,
    captures: &[&PrecautionaryHearingCapture],
    evidence: &MeasureHistoryEvidence,
) -> Result<(), ApplicationError> {
    let first = captures
        .first()
        .ok_or_else(|| invalid("empty appointment evidence"))?;
    let refs = target_union(captures.iter().map(|capture| &capture.review))?;
    let proof = resolve_measure_closure(hasher, first.review.case_id, &refs, evidence)?;
    let mut capture_inventory = SourceInventory::default();
    for group in proof.groups() {
        add_measure_group_sources(&mut capture_inventory, &group.review)?;
    }
    for capture in captures {
        capture_inventory.capture(capture)?;
        receipt_flat(hasher, capture)?;
        if capture.review.case_id != first.review.case_id
            || capture.recorded_at < target_clock(&capture.review, &proof)?
        {
            return Err(invalid("appointment case or target chronology differs"));
        }
    }
    Ok(())
}
