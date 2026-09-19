//! JSON representations of captured tracking state, without hash verification.
use super::sources;
use crate::error::ApiError;
use application::{
    deadline_reevaluation::{encode_observations, DependencyFamily, ObservationRole, Observations},
    deadline_tracking::{
        DeadlineReviewState, TrackingDependency, TrackingPolicies, TrackingPolicy, TrackingReview,
        TrackingReviewReason,
    },
    deadlines::DeadlineTrackingCapture,
};
use domain::cases::CaseId;
use serde_json::{json, Value};

pub(super) fn capture(v: &DeadlineTrackingCapture, case: CaseId) -> Result<Value, ApiError> {
    let observed = observations(&v.observations, case)?;
    let present = |role| {
        v.observations
            .entries
            .iter()
            .any(|entry| entry.role == role)
    };
    v.review
        .validate_policies(
            &v.policies,
            [
                true,
                present(ObservationRole::Source),
                present(ObservationRole::Calendar),
            ],
        )
        .map_err(|_| ApiError::internal())?;
    let notification = v.observations.entries.iter().any(|entry| {
        entry.role == ObservationRole::Source && entry.family == DependencyFamily::Notification
    });
    if notification
        && v.review.state() == DeadlineReviewState::Accepted
        && !present(ObservationRole::NotificationParent)
    {
        return Err(ApiError::internal());
    }
    Ok(json!({
        "policies": policies(&v.policies),
        "review": review(&v.review),
        "observations": observed,
        "administration": sources::administration(&v.administration, case)?,
    }))
}

pub(super) fn policies(v: &TrackingPolicies) -> Value {
    json!({
        "profile": policy(v.profile),
        "source": policy(v.source),
        "calendar": policy(v.calendar),
    })
}

fn policy(v: TrackingPolicy) -> &'static str {
    match v {
        TrackingPolicy::Fixed => "fixed",
        TrackingPolicy::Follow => "follow",
        TrackingPolicy::Undetermined => "undetermined",
    }
}

pub(super) fn review(v: &TrackingReview) -> Value {
    let state = match v.state() {
        DeadlineReviewState::Accepted => "accepted",
        DeadlineReviewState::Pending => "pending",
        DeadlineReviewState::LegacyUndeclared => "legacy_undeclared",
    };
    let reasons: Vec<_> = v
        .reasons()
        .iter()
        .map(|entry| {
            let dependency = match entry.dependency {
                TrackingDependency::Profile => "profile",
                TrackingDependency::Source => "source",
                TrackingDependency::Calendar => "calendar",
            };
            let reason = match entry.reason {
                TrackingReviewReason::SourceChanged => "source_changed",
                TrackingReviewReason::ProfileChanged => "profile_changed",
                TrackingReviewReason::DependencyRetired => "dependency_retired",
                TrackingReviewReason::PolicyUndetermined => "policy_undetermined",
            };
            json!({"dependency": dependency, "reason": reason})
        })
        .collect();
    json!({"state": state, "reasons": reasons})
}

pub(super) fn observations(v: &Observations, case: CaseId) -> Result<Value, ApiError> {
    if v.case_id != case {
        return Err(ApiError::internal());
    }
    // The canonical encoder validates shape, not the supplied digest contents.
    encode_observations(v).map_err(|_| ApiError::internal())?;
    let entries: Vec<_> = v
        .entries
        .iter()
        .map(|entry| {
            let role = match entry.role {
                ObservationRole::Profile => "profile",
                ObservationRole::Source => "source",
                ObservationRole::Calendar => "calendar",
                ObservationRole::NotificationParent => "notification_parent",
            };
            let family = match entry.family {
                DependencyFamily::Resolution => "resolution",
                DependencyFamily::Notification => "notification",
                DependencyFamily::HearingResult => "hearing_result",
                DependencyFamily::Calendar => "calendar",
                DependencyFamily::Profile => "profile",
            };
            json!({
                "role": role,
                "family": family,
                "id": entry.id.to_string(),
                "revision": entry.revision,
                "case_id": entry.case_id.map(|id| id.to_string()),
                "hearing_id": entry.hearing_id.map(|id| id.to_string()),
                "parent_resolution": entry.parent_resolution.map(|parent| json!({
                    "id": parent.id.to_string(), "revision": parent.revision,
                })),
                "submission_digest": entry.submission_digest.to_hex(),
                "evidence_digest": entry.evidence_digest.to_hex(),
            })
        })
        .collect();
    Ok(json!({"case_id": case.to_string(), "entries": entries}))
}
