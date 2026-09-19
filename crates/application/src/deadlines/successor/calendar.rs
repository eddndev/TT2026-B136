use super::{super::*, preservation};
use crate::{
    deadline_reevaluation::{DependencyFamily, ObservationRole, TechnicalCause},
    deadline_tracking::{DeadlineReviewState, TrackingPolicy},
    ApplicationError,
};
use domain::{crypto::DocumentHasher, judicial_calendars::JudicialCalendarStatus};

/// Only the new calendar selection, its captured head and the arithmetic result
/// may differ. A technical preparer computes that result before committing it;
/// verifying stored history must never replay the arithmetic algorithm.
pub(super) fn validate(
    hasher: &dyn DocumentHasher,
    previous: &DeadlineDetail,
    next: &DeadlineDetail,
    cause: TechnicalCause,
) -> Result<(), ApplicationError> {
    let old = previous
        .tracking
        .as_ref()
        .ok_or_else(|| inconsistent("calendar tracking absent"))?;
    let new = next
        .tracking
        .as_ref()
        .ok_or_else(|| inconsistent("calendar tracking absent"))?;
    if old.review.state() != DeadlineReviewState::Accepted
        || new.review.state() != DeadlineReviewState::Accepted
        || old.policies.calendar != TrackingPolicy::Follow
    {
        return Err(inconsistent(
            "calendar recalculation requires accepted followed tracking",
        ));
    }
    let Some(earlier) = previous.definition.input.calendar else {
        return Err(inconsistent(
            "calendar recalculation has no previous selection",
        ));
    };
    let Some(selected) = next.definition.input.calendar else {
        return Err(inconsistent("calendar recalculation removed its selection"));
    };
    let old_observed = old
        .observations
        .entries
        .iter()
        .find(|entry| entry.role == ObservationRole::Calendar)
        .ok_or_else(|| inconsistent("previous calendar observation absent"))?;
    if earlier.id != selected.id || selected.revision.get() <= old_observed.revision {
        return Err(inconsistent(
            "calendar recalculation did not select a new revision",
        ));
    }
    let calendar = next
        .calculation
        .material
        .calendar
        .as_ref()
        .ok_or_else(|| inconsistent("new calendar evidence absent"))?;
    let TechnicalCause::SourceEvent { event, .. } = cause else {
        return Err(inconsistent(
            "calendar recalculation requires a source event",
        ));
    };
    // A job can observe a newer calendar while handling another dependency.
    // For this exact calendar revision, its captured receipt also proves the
    // event operation. An older event requires its own durable row verification.
    let calendar_event_differs = event.family == DependencyFamily::Calendar
        && (event.source_id != calendar.id.as_uuid()
            || event.revision > calendar.revision.get()
            || (event.revision == calendar.revision.get()
                && event.operation_id != calendar.receipt.operation_id.as_uuid())
            || event.case_id.is_some()
            || event.hearing_id.is_some());
    if calendar_event_differs || calendar.status != JudicialCalendarStatus::Published {
        return Err(inconsistent(
            "calendar recalculation event or status differs",
        ));
    }
    for entry in old
        .observations
        .entries
        .iter()
        .filter(|entry| entry.role != ObservationRole::Calendar)
    {
        let policy = match entry.role {
            ObservationRole::Profile => old.policies.profile,
            ObservationRole::Source | ObservationRole::NotificationParent => old.policies.source,
            ObservationRole::Calendar => unreachable!(),
        };
        // Monotonic identity-preserving observations are checked before this
        // branch. Fixed selections may observe new heads without replacing the
        // retained profile, source, or human qualification below.
        if policy != TrackingPolicy::Fixed && !new.observations.entries.contains(entry) {
            return Err(inconsistent(
                "calendar recalculation changed another followed observation",
            ));
        }
    }
    if old.observations.entries.len() != new.observations.entries.len() {
        return Err(inconsistent(
            "calendar recalculation changed observation presence",
        ));
    }
    let mut same_head = next.clone();
    same_head.calculation.material.calendar = next.calculation.material.calendar_head.clone();
    if !preservation::same_body(hasher, next, &same_head)? {
        return Err(inconsistent(
            "calendar recalculation did not capture the selected head",
        ));
    }
    let mut compared = next.clone();
    compared.definition.input.calendar = previous.definition.input.calendar;
    compared.calculation.material.calendar = previous.calculation.material.calendar.clone();
    compared.calculation.material.calendar_head =
        previous.calculation.material.calendar_head.clone();
    compared.calculation.result = previous.calculation.result.clone();
    if !preservation::same_body(hasher, previous, &compared)? {
        return Err(inconsistent(
            "calendar recalculation changed retained legal evidence",
        ));
    }
    Ok(())
}
