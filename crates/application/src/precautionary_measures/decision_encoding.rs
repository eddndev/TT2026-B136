use super::{decision_wire::*, *};
use crate::{identity::Principal, ApplicationError};
use domain::cases::CaseId;

pub fn measure_decision_submission_bytes(
    principal: &Principal,
    case_id: CaseId,
    command: &MeasureDecisionCommand,
) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"MDTXN1".to_vec();
    actor(&mut bytes, principal)?;
    bytes.extend_from_slice(case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(command.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(command.decision_id.as_uuid().as_bytes());
    bytes.extend_from_slice(&command.context.administration_revision.get().to_be_bytes());
    bytes.extend_from_slice(&command.context.stage_revision.get().to_be_bytes());
    bytes.extend_from_slice(command.context.context_digest.as_bytes());
    blob(&mut bytes, &command.values.canonical_bytes())?;
    no_anchor(&mut bytes, command.anchor.is_some())?;
    blob(&mut bytes, &command.outcome.canonical_bytes())?;
    Ok(bytes)
}

pub fn measure_decision_review_bytes(
    review: &MeasureDecisionReview,
) -> Result<Vec<u8>, ApplicationError> {
    bounded(review.results.len())?;
    bounded(review.material.result_sources.len())?;
    if !review.material.predecessors.is_empty() {
        return Err(invalid("predecessor groups require ancestry validation"));
    }
    let mut bytes = b"MDPR1".to_vec();
    blob(
        &mut bytes,
        &measure_decision_submission_bytes(&review.actor, review.case_id, &review.command)?,
    )?;
    bytes.extend_from_slice(review.submission_digest.as_bytes());
    blob(&mut bytes, &review.material.context.canonical_bytes())?;
    support(&mut bytes, &review.material.support)?;
    no_anchor(&mut bytes, review.material.anchor.is_some())?;
    bytes.extend_from_slice(&0u32.to_be_bytes());
    bytes.extend_from_slice(&(review.results.len() as u32).to_be_bytes());
    for value in &review.results {
        result(&mut bytes, value)?;
    }
    Ok(bytes)
}

pub fn measure_decision_capture_bytes(
    capture: &MeasureDecisionCapture,
) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"MDCR1".to_vec();
    bytes.extend_from_slice(capture.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(capture.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(capture.decision_id.as_uuid().as_bytes());
    actor(&mut bytes, &capture.actor)?;
    blob(&mut bytes, &capture.context.canonical_bytes())?;
    blob(&mut bytes, &capture.values.canonical_bytes())?;
    support(&mut bytes, &capture.support)?;
    no_anchor(&mut bytes, capture.anchor.is_some())?;
    timestamp(&mut bytes, capture.recorded_at);
    Ok(bytes)
}

pub fn measure_capture_bytes(capture: &MeasureCapture) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"MMCR1".to_vec();
    bytes.extend_from_slice(capture.case_id.as_uuid().as_bytes());
    result(&mut bytes, &capture.result)?;
    bytes.extend_from_slice(capture.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(capture.decision_id.as_uuid().as_bytes());
    bytes.extend_from_slice(capture.decision_digest.as_bytes());
    actor(&mut bytes, &capture.actor)?;
    timestamp(&mut bytes, capture.recorded_at);
    Ok(bytes)
}

pub fn measure_decision_group_bytes(
    group: &MeasureDecisionGroupCapture,
) -> Result<Vec<u8>, ApplicationError> {
    bounded(group.measures.len())?;
    if !group.substitutions.is_empty() {
        return Err(invalid("substitution groups require ancestry validation"));
    }
    let mut bytes = b"MDGR1".to_vec();
    blob(&mut bytes, &measure_decision_review_bytes(&group.review)?)?;
    bytes.extend_from_slice(group.review.review_digest.as_bytes());
    blob(
        &mut bytes,
        &measure_decision_capture_bytes(&group.decision)?,
    )?;
    bytes.extend_from_slice(group.decision.capture_digest.as_bytes());
    bytes.extend_from_slice(&(group.measures.len() as u32).to_be_bytes());
    for value in &group.measures {
        blob(&mut bytes, &measure_capture_bytes(value)?)?;
        bytes.extend_from_slice(value.capture_digest.as_bytes());
    }
    bytes.extend_from_slice(&0u32.to_be_bytes());
    timestamp(&mut bytes, group.recorded_at);
    Ok(bytes)
}
