//! Cross-field transport checks for a captured calculation and its tracking.
use super::{sources, tracking_projection};
use crate::error::ApiError;
use application::{
    cases::CaseAdministrativeStatus,
    deadline_reevaluation::TechnicalCause,
    deadline_tracking::{DeadlineReviewState, TrackingPolicy},
    deadlines::*,
};
use domain::cases::CaseId;
use serde_json::Value;

pub(super) fn project(
    receipt: &DeadlineReceipt,
    capture: Option<&DeadlineTrackingCapture>,
    calculation: &DeadlineCalculation,
    case: CaseId,
) -> Result<Value, ApiError> {
    let (metadata, capture) = match (&receipt.version, capture) {
        (DeadlineReceiptVersion::Legacy, None) => return Ok(Value::Null),
        (DeadlineReceiptVersion::Tracked(metadata), Some(capture)) => (metadata, capture),
        _ => return Err(ApiError::internal()),
    };
    let output = tracking_projection::capture(capture, case)?;
    match receipt.action {
        DeadlineAction::Register | DeadlineAction::Correct => {
            if capture.review.state() != DeadlineReviewState::Accepted
                || capture.administration.values().status() != CaseAdministrativeStatus::Active
                || output["administration"]
                    != sources::administration(&calculation.material.administration, case)?
            {
                return Err(ApiError::internal());
            }
        }
        DeadlineAction::Reevaluate => technical(metadata, capture)?,
        DeadlineAction::SetAttention | DeadlineAction::Retire => {}
    }
    Ok(output)
}

fn technical(
    metadata: &DeadlineTrackedReceipt,
    capture: &DeadlineTrackingCapture,
) -> Result<(), ApiError> {
    if capture.review.state() == DeadlineReviewState::LegacyUndeclared {
        return Err(ApiError::internal());
    }
    let valid = match metadata.cause {
        Some(TechnicalCause::LegacyBootstrap { .. }) => {
            let policies = &capture.policies;
            capture.review.state() == DeadlineReviewState::Pending
                && [policies.profile, policies.source, policies.calendar]
                    .iter()
                    .all(|policy| *policy == TrackingPolicy::Undetermined)
        }
        Some(TechnicalCause::SourceEvent { event, .. }) => {
            capture.observations.entries.iter().any(|entry| {
                entry.family == event.family
                    && entry.id == event.source_id
                    && entry.case_id == event.case_id
                    && entry.hearing_id == event.hearing_id
                    && entry.revision >= event.revision
            })
        }
        None => false,
    };
    if !valid {
        return Err(ApiError::internal());
    }
    Ok(())
}
