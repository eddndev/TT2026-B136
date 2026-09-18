use super::*;
use crate::{
    deadline_inputs::DeadlineSourceDetail,
    deadline_observations::{build_deadline_observations, build_legacy_deadline_observations},
    deadline_reevaluation::{encode_observations, PredecessorReceipt},
    deadline_tracking::{DeadlineReviewState, TrackingPolicies, TrackingPolicy, TrackingReview},
    procedural_facts::{FactDetail, FactStatus},
    ApplicationError,
};
use domain::{
    cases::CaseId, crypto::DocumentHasher, hearing_results::HearingResultStatus,
    judicial_calendars::JudicialCalendarStatus,
};

/// Prepare an explicitly reviewed human operation. Authorization and current-head
/// resolution belong to the application service and are repeated by the store.
/// Policies apply to registration/correction only; attention and retirement keep
/// the predecessor's tracking declaration, including any pending human review.
pub fn prepare_tracked_deadline_change(
    hasher: &dyn DocumentHasher,
    author: DeadlineActorSnapshot,
    case_id: CaseId,
    command: DeadlineCommand,
    preparation: DeadlinePreparation,
    policies: Option<TrackingPolicies>,
    notification_parent_head: Option<&FactDetail>,
) -> Result<PreparedDeadlineChange, ApplicationError> {
    let actor = author
        .user_id()
        .ok_or_else(|| inconsistent("human preparation requires a user author"))?;
    let qualification = matches!(
        command.action(),
        DeadlineAction::Register | DeadlineAction::Correct
    );
    if qualification != policies.is_some() || (!qualification && notification_parent_head.is_some())
    {
        return Err(DeadlineError::Invalid("tracking qualification inputs").into());
    }
    let mut prepared = preparation::prepare_with_profile_policy(
        hasher,
        actor,
        case_id,
        command,
        preparation,
        policies.map(|value| value.profile),
    )?;
    let tracking = if let Some(policies) = policies {
        let resolved = prepared
            .preparation
            .resolved
            .as_ref()
            .ok_or_else(|| inconsistent("qualification requires resolved inputs"))?;
        require_available_heads(resolved, notification_parent_head)?;
        DeadlineTrackingCapture {
            policies,
            review: TrackingReview::new(DeadlineReviewState::Accepted, vec![])
                .map_err(|error| inconsistent(&error.to_string()))?,
            observations: build_deadline_observations(
                hasher,
                case_id,
                &resolved.profile_head,
                &resolved.material,
                notification_parent_head,
            )?,
            administration: prepared.preparation.administration.clone(),
        }
    } else {
        let base = prepared
            .preparation
            .base
            .as_ref()
            .ok_or(DeadlineError::NotFound)?;
        match &base.tracking {
            Some(tracking) => tracking.clone(),
            None => DeadlineTrackingCapture {
                policies: TrackingPolicies {
                    profile: TrackingPolicy::Undetermined,
                    source: TrackingPolicy::Undetermined,
                    calendar: TrackingPolicy::Undetermined,
                },
                review: TrackingReview::new(DeadlineReviewState::LegacyUndeclared, vec![])
                    .map_err(|error| inconsistent(&error.to_string()))?,
                observations: build_legacy_deadline_observations(hasher, base)?,
                administration: base.calculation.material.administration.clone(),
            },
        }
    };
    let observations_digest = hasher.hash_bytes(
        &encode_observations(&tracking.observations)
            .map_err(|error| inconsistent(&error.to_string()))?,
    );
    prepared.receipt_version = DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
        observations_digest,
        predecessor: prepared
            .preparation
            .base
            .as_ref()
            .map(|base| PredecessorReceipt {
                submission_digest: base.receipt.submission_digest,
                capture_digest: base.receipt.capture_digest,
            }),
        cause: None,
    });
    prepared.tracking = Some(tracking);
    prepared.tracked_author = Some(author.clone());
    prepared.review_digest =
        canonical::state_digest(hasher, canonical::Content::from(&prepared), false)?;
    prepared.capture_digest =
        canonical::state_digest(hasher, canonical::Content::from(&prepared), true)?;
    prepared.submission_digest = hasher.hash_bytes(&tracked::submission_bytes(
        &author,
        case_id,
        prepared.command.deadline_id,
        &prepared.receipt(),
        prepared.command.reason(),
    )?);
    Ok(prepared)
}

fn require_available_heads(
    resolved: &DeadlineResolvedInputs,
    parent: Option<&FactDetail>,
) -> Result<(), ApplicationError> {
    let source_available = match &resolved.material.source_head {
        None => true,
        Some(DeadlineSourceDetail::Fact(value)) => {
            value.snapshot.metadata().status == FactStatus::Recorded
        }
        Some(DeadlineSourceDetail::HearingResult(value)) => {
            value.snapshot.status == HearingResultStatus::Recorded
        }
    };
    if !source_available
        || resolved
            .material
            .calendar_head
            .as_ref()
            .is_some_and(|head| head.status != JudicialCalendarStatus::Published)
        || parent.is_some_and(|head| head.snapshot.metadata().status != FactStatus::Recorded)
    {
        return Err(DeadlineError::Invalid("withdrawn dependency requires explicit review").into());
    }
    Ok(())
}
