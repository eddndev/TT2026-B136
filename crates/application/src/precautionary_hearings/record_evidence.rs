use super::{capture_validation::*, source_inventory::SourceInventory, *};
use crate::{
    identity::Principal,
    measure_corrections::{
        checked_record_closure, record_history_bounds, CheckedRecordClosure,
        MeasureCaptureValidity, MeasureRecordHistoryEvidence,
    },
    ApplicationError,
};
use domain::{
    cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher,
    precautionary_hearings::PrecautionaryMeasureRef,
};
use std::collections::BTreeMap;

pub struct PrecautionaryHearingRecordPreparationMaterial<'a> {
    pub observed_context: PrecautionaryContext,
    pub sources: PrecautionaryHearingSources,
    pub predecessor: Option<&'a PrecautionaryHearingCapture>,
    pub record_history: &'a MeasureRecordHistoryEvidence,
}

pub fn prepare_precautionary_hearing_with_record_history(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: PrecautionaryHearingCommand,
    material: PrecautionaryHearingRecordPreparationMaterial<'_>,
) -> Result<CheckedPrecautionaryHearingReview, ApplicationError> {
    record_history_bounds(material.record_history)?;
    if material.sources.participants.len() > 32 {
        return Err(invalid("too many participant sources"));
    }
    if let Some(previous) = material.predecessor {
        shape(previous)?;
    }
    let new_targets = match &command.change {
        PrecautionaryHearingChange::Schedule { values, .. }
        | PrecautionaryHearingChange::Replace { values, .. } => values.review_targets(),
        PrecautionaryHearingChange::Cancel { .. } => &[],
    };
    let old_targets = material
        .predecessor
        .map_or(&[][..], |p| p.review.resolved_values.review_targets());
    let refs = union(new_targets.iter().chain(old_targets))?;
    let mut inventory = SourceInventory::default();
    let proof = checked_record_closure(
        hasher,
        case_id,
        &refs,
        material.record_history,
        &mut inventory,
    )?;
    if let Some(previous) = material.predecessor {
        inventory.capture(previous)?;
    }
    let mut checked = super::capture_preparation::prepare_flat(
        hasher,
        actor,
        case_id,
        command,
        material.observed_context,
        material.sources,
        material.predecessor,
    )?;
    inventory.add(checked.review())?;
    if let Some(previous) = material.predecessor {
        if previous.recorded_at < target_clock(&previous.review, &proof)? {
            return Err(invalid("predecessor predates selected record evidence"));
        }
    }
    checked.earliest_capture = checked
        .earliest_capture
        .max(target_clock(checked.review(), &proof)?);
    Ok(checked)
}

fn union<'a>(
    refs: impl IntoIterator<Item = &'a PrecautionaryMeasureRef>,
) -> Result<Vec<PrecautionaryMeasureRef>, ApplicationError> {
    let mut unique = BTreeMap::new();
    for reference in refs {
        let key = (reference.id().as_uuid(), reference.revision().get());
        if let Some(old) = unique.insert(key, *reference) {
            if old != *reference {
                return Err(invalid("one target revision has conflicting digests"));
            }
        }
        if unique.len() > 8192 {
            return Err(invalid("record target union budget exceeded"));
        }
    }
    Ok(unique.into_values().collect())
}

fn target_clock(
    review: &PrecautionaryHearingReview,
    proof: &CheckedRecordClosure<'_>,
) -> Result<OffsetDateTime, ApplicationError> {
    let mut floor = latest_source_time(review)?;
    for reference in review.resolved_values.review_targets() {
        let target = proof.member(*reference)?;
        if target.validity() != MeasureCaptureValidity::Valid {
            return Err(invalid(
                "review selects a record captured as entered in error",
            ));
        }
        context_advances(target.context(), &review.scheduling_context)?;
        floor = floor.max(target.recorded_at());
    }
    Ok(floor)
}

pub(super) fn shape(capture: &PrecautionaryHearingCapture) -> Result<(), ApplicationError> {
    if capture.review.sources.participants.len() > 32 || capture.review.participants.len() > 32 {
        return Err(invalid("appointment participant evidence budget exceeded"));
    }
    Ok(())
}

pub(super) fn check_captures(
    hasher: &dyn DocumentHasher,
    captures: &[&PrecautionaryHearingCapture],
    evidence: &MeasureRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    if captures.len() > 256 {
        return Err(invalid("appointment history budget exceeded"));
    }
    record_history_bounds(evidence)?;
    for capture in captures {
        shape(capture)?;
    }
    let first = captures
        .first()
        .ok_or_else(|| invalid("empty appointment evidence"))?;
    let refs = union(
        captures
            .iter()
            .flat_map(|c| c.review.resolved_values.review_targets()),
    )?;
    let mut inventory = SourceInventory::default();
    let proof = checked_record_closure(
        hasher,
        first.review.case_id,
        &refs,
        evidence,
        &mut inventory,
    )?;
    for capture in captures {
        inventory.capture(capture)?;
        receipt_flat(hasher, capture)?;
        if capture.review.case_id != first.review.case_id
            || capture.recorded_at < target_clock(&capture.review, &proof)?
        {
            return Err(invalid("appointment case or record chronology differs"));
        }
    }
    Ok(())
}
