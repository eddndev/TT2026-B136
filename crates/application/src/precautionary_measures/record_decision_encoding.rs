use super::{decision_wire::*, record_decision_wire, *};
use crate::ApplicationError;

pub fn measure_decision_review_v2_bytes(
    review: &MeasureDecisionReviewV2,
) -> Result<Vec<u8>, ApplicationError> {
    bounded(review.results.len())?;
    bounded(review.material.result_sources.len())?;
    bounded(review.material.predecessors.len())?;
    super::anchor_validation::shape(&review.material.anchor)?;
    let mut bytes = b"MDPR2".to_vec();
    blob(
        &mut bytes,
        &measure_decision_submission_bytes(&review.actor, review.case_id, &review.command)?,
    )?;
    bytes.extend_from_slice(review.submission_digest.as_bytes());
    blob(&mut bytes, &review.material.context.canonical_bytes())?;
    support(&mut bytes, &review.material.support)?;
    super::anchor_encoding::material(&mut bytes, review.material.anchor.as_ref())?;
    bytes.extend_from_slice(&(review.material.predecessors.len() as u32).to_be_bytes());
    for prior in &review.material.predecessors {
        record_decision_wire::predecessor(&mut bytes, prior)?;
    }
    bytes.extend_from_slice(&(review.results.len() as u32).to_be_bytes());
    for result in &review.results {
        record_decision_wire::result(&mut bytes, result)?;
    }
    Ok(bytes)
}

pub fn measure_capture_v2_bytes(capture: &MeasureCaptureV2) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"MMCR2".to_vec();
    bytes.extend_from_slice(capture.case_id.as_uuid().as_bytes());
    record_decision_wire::result(&mut bytes, &capture.result)?;
    bytes.extend_from_slice(capture.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(capture.decision_id.as_uuid().as_bytes());
    bytes.extend_from_slice(capture.decision_digest.as_bytes());
    actor(&mut bytes, &capture.actor)?;
    timestamp(&mut bytes, capture.recorded_at);
    Ok(bytes)
}

pub fn measure_decision_group_v2_bytes(
    group: &MeasureDecisionGroupCaptureV2,
) -> Result<Vec<u8>, ApplicationError> {
    record_decision_group_shape(group)?;
    let mut bytes = b"MDGR2".to_vec();
    blob(
        &mut bytes,
        &measure_decision_review_v2_bytes(&group.review)?,
    )?;
    bytes.extend_from_slice(group.review.review_digest.as_bytes());
    blob(
        &mut bytes,
        &measure_decision_capture_bytes(&group.decision)?,
    )?;
    bytes.extend_from_slice(group.decision.capture_digest.as_bytes());
    bytes.extend_from_slice(&(group.measures.len() as u32).to_be_bytes());
    for capture in &group.measures {
        blob(&mut bytes, &measure_capture_v2_bytes(capture)?)?;
        bytes.extend_from_slice(capture.capture_digest.as_bytes());
    }
    bytes.extend_from_slice(&(group.substitutions.len() as u32).to_be_bytes());
    for relation in &group.substitutions {
        bytes.extend_from_slice(relation.effect_key.as_uuid().as_bytes());
        bytes.extend_from_slice(&(relation.predecessors.len() as u32).to_be_bytes());
        for pair in &relation.predecessors {
            reference(&mut bytes, pair.previous);
            reference(&mut bytes, pair.result);
        }
        bytes.extend_from_slice(&(relation.successors.len() as u32).to_be_bytes());
        for successor in &relation.successors {
            reference(&mut bytes, *successor);
        }
    }
    timestamp(&mut bytes, group.recorded_at);
    Ok(bytes)
}

pub(crate) fn record_decision_group_shape(
    group: &MeasureDecisionGroupCaptureV2,
) -> Result<(), ApplicationError> {
    bounded(group.measures.len())?;
    bounded(group.review.results.len())?;
    bounded(group.review.material.result_sources.len())?;
    bounded(group.review.material.predecessors.len())?;
    bounded(group.substitutions.len())?;
    super::anchor_validation::shape(&group.review.material.anchor)?;
    super::anchor_validation::shape(&group.decision.anchor)?;
    for relation in &group.substitutions {
        bounded(relation.predecessors.len())?;
        bounded(relation.successors.len())?;
    }
    Ok(())
}
