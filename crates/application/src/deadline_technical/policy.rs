use super::*;
use crate::{
    deadline_inputs::DeadlineSourceDetail,
    deadline_profiles::DeadlineProfileStatus,
    deadline_reevaluation::{ObservationRole, Observations},
    deadline_tracking::{
        decide_tracking_change, TrackingChange, TrackingDependency, TrackingDisposition,
        TrackingReview, TrackingReviewReason, TrackingReviewRequirement,
    },
    procedural_facts::{FactStatus, ProceduralFactSnapshot},
};
use domain::{
    deadline_triggers::TriggerSourceRef, hearing_results::HearingResultStatus,
    judicial_calendars::JudicialCalendarStatus, procedural_facts::FactDeclaration,
};

pub(super) fn review(
    base: &DeadlineDetail,
    inputs: &DeadlineReevaluationInputs,
    policies: TrackingPolicies,
    old: &Observations,
    new: &Observations,
) -> Result<(TrackingReview, bool)> {
    let mut reasons = base
        .tracking
        .as_ref()
        .map_or_else(Vec::new, |value| value.review.reasons().to_vec());
    let mut recalculate = false;
    for entry in &new.entries {
        let (dependency, policy) = match entry.role {
            ObservationRole::Profile => (TrackingDependency::Profile, policies.profile),
            ObservationRole::Source | ObservationRole::NotificationParent => {
                (TrackingDependency::Source, policies.source)
            }
            ObservationRole::Calendar => (TrackingDependency::Calendar, policies.calendar),
        };
        if policy == TrackingPolicy::Undetermined {
            reasons.push(TrackingReviewRequirement {
                dependency,
                reason: TrackingReviewReason::PolicyUndetermined,
            });
        }
        let selected_revision = selected_revision(base, entry.role)?;
        let observed_revision = old
            .entries
            .iter()
            .find(|value| value.role == entry.role)
            .map_or(selected_revision, |value| value.revision);
        let decision = decide_tracking_change(TrackingChange {
            dependency,
            policy,
            selected_revision,
            observed_revision,
            new_revision: entry.revision,
            retired: retired(inputs, entry.role),
        })
        .map_err(inconsistent)?;
        match decision.disposition {
            TrackingDisposition::ReviewRequired(reason) => {
                reasons.push(TrackingReviewRequirement { dependency, reason })
            }
            TrackingDisposition::Recalculate => recalculate = true,
            TrackingDisposition::Unchanged | TrackingDisposition::Preserve => {}
        }
    }
    reasons.sort_by_key(|value| key(*value));
    reasons.dedup();
    let state = if reasons.is_empty() {
        DeadlineReviewState::Accepted
    } else {
        DeadlineReviewState::Pending
    };
    let review = TrackingReview::new(state, reasons).map_err(inconsistent)?;
    recalculate &= state == DeadlineReviewState::Accepted
        && base.review_state() == DeadlineReviewState::Accepted;
    Ok((review, recalculate))
}

fn selected_revision(base: &DeadlineDetail, role: ObservationRole) -> Result<u32> {
    let revision = match role {
        ObservationRole::Profile => Some(base.definition.profile.revision.get()),
        ObservationRole::Calendar => base
            .definition
            .input
            .calendar
            .map(|value| value.revision.get()),
        ObservationRole::Source => match base.definition.input.selection.source {
            FactDeclaration::Known(TriggerSourceRef::Resolution(value)) => {
                Some(value.revision.get())
            }
            FactDeclaration::Known(TriggerSourceRef::Notification { revision, .. }) => {
                Some(revision.get())
            }
            FactDeclaration::Known(TriggerSourceRef::HearingResult(value)) => {
                Some(value.revision.get())
            }
            FactDeclaration::Unknown(_) => None,
        },
        ObservationRole::NotificationParent => match &base.calculation.material.source {
            Some(DeadlineSourceDetail::Fact(value)) => match &value.snapshot {
                ProceduralFactSnapshot::Notification(value) => {
                    Some(value.values.resolution().revision.get())
                }
                _ => None,
            },
            _ => None,
        },
    };
    revision.ok_or_else(|| inconsistent("observed dependency has no historical selection"))
}

fn retired(inputs: &DeadlineReevaluationInputs, role: ObservationRole) -> bool {
    match role {
        ObservationRole::Profile => inputs.profile_head.status == DeadlineProfileStatus::Retired,
        ObservationRole::Calendar => inputs
            .material
            .calendar_head
            .as_ref()
            .is_some_and(|value| value.status != JudicialCalendarStatus::Published),
        ObservationRole::NotificationParent => inputs
            .notification_parent_head
            .as_ref()
            .is_some_and(|value| value.snapshot.metadata().status != FactStatus::Recorded),
        ObservationRole::Source => {
            inputs
                .material
                .source_head
                .as_ref()
                .is_some_and(|value| match value {
                    DeadlineSourceDetail::Fact(value) => {
                        value.snapshot.metadata().status != FactStatus::Recorded
                    }
                    DeadlineSourceDetail::HearingResult(value) => {
                        value.snapshot.status != HearingResultStatus::Recorded
                    }
                })
        }
    }
}
fn key(value: TrackingReviewRequirement) -> (u8, u8) {
    let dependency = match value.dependency {
        TrackingDependency::Profile => 0,
        TrackingDependency::Source => 1,
        TrackingDependency::Calendar => 2,
    };
    let reason = match value.reason {
        TrackingReviewReason::SourceChanged => 0,
        TrackingReviewReason::ProfileChanged => 1,
        TrackingReviewReason::DependencyRetired => 2,
        TrackingReviewReason::PolicyUndetermined => 3,
    };
    (dependency, reason)
}
