use super::history;
use crate::error::ApiError;
use application::precautionary_measures::*;
use domain::{
    cases::CaseId,
    precautionary_measures::{MeasureDecisionId, MeasureDecisionOperationId},
};

fn counts(values: &[usize]) -> Result<(), ApiError> {
    if values.iter().any(|count| *count > 32) {
        return Err(ApiError::internal());
    }
    Ok(())
}
fn anchor(value: &Option<MeasureDecisionAnchorMaterial>) -> Result<(), ApiError> {
    match value {
        Some(MeasureDecisionAnchorMaterial::Initial(value)) => counts(&[value.participants.len()]),
        Some(MeasureDecisionAnchorMaterial::Precautionary(value)) => counts(&[
            value.review.sources.participants.len(),
            value.review.participants.len(),
        ]),
        None => Ok(()),
    }
}
pub(super) fn review_v1(value: &MeasureDecisionReview) -> Result<(), ApiError> {
    counts(&[
        value.results.len(),
        value.material.predecessors.len(),
        value.material.result_sources.len(),
    ])?;
    anchor(&value.material.anchor)
}
pub(super) fn review_v2(value: &MeasureDecisionReviewV2) -> Result<(), ApiError> {
    counts(&[
        value.results.len(),
        value.material.predecessors.len(),
        value.material.result_sources.len(),
    ])?;
    anchor(&value.material.anchor)
}
fn substitutions(values: &[MeasureSubstitutionCapture]) -> Result<(), ApiError> {
    counts(&[values.len()])?;
    for value in values {
        counts(&[value.predecessors.len(), value.successors.len()])?;
    }
    Ok(())
}
fn group_v1(value: &MeasureDecisionGroupCapture) -> Result<(), ApiError> {
    counts(&[value.measures.len()])?;
    review_v1(&value.review)?;
    anchor(&value.decision.anchor)?;
    substitutions(&value.substitutions)
}
fn group_v2(value: &MeasureDecisionGroupCaptureV2) -> Result<(), ApiError> {
    counts(&[value.measures.len()])?;
    review_v2(&value.review)?;
    anchor(&value.decision.anchor)?;
    substitutions(&value.substitutions)
}

// Both durable families bind the same origin fields and their own typed result rows.
macro_rules! bind_group {
    ($group:expr, $origin:expr, $case:expr) => {{
        let group = $group;
        let origin = $origin;
        let review = &group.review;
        let decision = &group.decision;
        let expected = MeasureGroupOrigin {
            case_id: review.case_id,
            operation_id: review.command.operation_id,
            decision_id: review.command.decision_id,
            submission_digest: review.submission_digest,
            review_digest: review.review_digest,
            decision_digest: decision.capture_digest,
            group_digest: group.capture_digest,
        };
        if *origin != expected
            || review.case_id != $case
            || decision.case_id != $case
            || decision.operation_id != origin.operation_id
            || decision.decision_id != origin.decision_id
            || decision.actor != review.actor
            || decision.context != review.material.context
            || decision.values != review.command.values
            || decision.support != review.material.support
            || decision.anchor != review.material.anchor
            || decision.recorded_at != group.recorded_at
            || group.measures.len() != review.results.len()
        {
            return Err(ApiError::internal());
        }
        for (row, result) in group.measures.iter().zip(&review.results) {
            if row.case_id != $case
                || row.operation_id != origin.operation_id
                || row.decision_id != origin.decision_id
                || row.decision_digest != origin.decision_digest
                || row.actor != review.actor
                || row.recorded_at != group.recorded_at
                || &row.result != result
            {
                return Err(ApiError::internal());
            }
        }
    }};
}

pub(crate) fn bound_decision(
    value: &MeasureDecisionRecordReceipt,
    case: CaseId,
    decision: Option<MeasureDecisionId>,
    operation: Option<MeasureDecisionOperationId>,
) -> Result<(), ApiError> {
    let origin = value.origin();
    if origin.case_id != case
        || decision.is_some_and(|id| id != origin.decision_id)
        || operation.is_some_and(|id| id != origin.operation_id)
    {
        return Err(ApiError::internal());
    }
    match value {
        MeasureDecisionRecordReceipt::V1(value) => {
            if value.measure_history.groups.len() >= 256 {
                return Err(ApiError::internal());
            }
            group_v1(&value.group)?;
            let mut rows = value.group.measures.len();
            for ancestor in &value.measure_history.groups {
                group_v1(&ancestor.capture)?;
                rows += ancestor.capture.measures.len();
            }
            if rows > 8192 {
                return Err(ApiError::internal());
            }
            bind_group!(&value.group, &value.origin, case);
        }
        MeasureDecisionRecordReceipt::V2(value) => {
            history::check(&value.record_history)?;
            group_v2(&value.group)?;
            let history = &value.record_history;
            let owners = history.records.judicial.groups.len()
                + history.records.administrative.len()
                + history.decisions.len();
            let rows = value.group.measures.len()
                + history
                    .records
                    .judicial
                    .groups
                    .iter()
                    .map(|v| v.capture.measures.len())
                    .sum::<usize>()
                + history
                    .records
                    .administrative
                    .iter()
                    .map(|v| v.capture.records.len())
                    .sum::<usize>()
                + history
                    .decisions
                    .iter()
                    .map(|v| v.capture.measures.len())
                    .sum::<usize>();
            if owners >= 256 || rows > 8192 {
                return Err(ApiError::internal());
            }
            bind_group!(&value.group, &value.origin, case);
        }
    }
    Ok(())
}
