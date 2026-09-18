use super::{super::*, calendar, preservation};
use crate::{
    deadline_reevaluation::{ObservationEntry, ObservationRole, TechnicalCause},
    deadline_tracking::{
        DeadlineReviewState, TrackingDependency, TrackingPolicy, TrackingReviewReason,
    },
    ApplicationError,
};
use domain::crypto::DocumentHasher;

pub(super) fn undeclared(value: &DeadlineTrackingCapture) -> bool {
    [
        value.policies.profile,
        value.policies.source,
        value.policies.calendar,
    ]
    .into_iter()
    .all(|policy| policy == TrackingPolicy::Undetermined)
}

pub(super) fn validate(
    hasher: &dyn DocumentHasher,
    previous: &DeadlineDetail,
    next: &DeadlineDetail,
) -> Result<(), ApplicationError> {
    let new = next
        .tracking
        .as_ref()
        .ok_or_else(|| inconsistent("technical successor requires tracking evidence"))?;
    let DeadlineReceiptVersion::Tracked(metadata) = &next.receipt.version else {
        return Err(inconsistent(
            "technical successor requires a tracked receipt",
        ));
    };
    let Some(cause) = metadata.cause else {
        return Err(inconsistent("technical successor requires a cause"));
    };
    let Some(old) = previous.tracking.as_ref() else {
        if !undeclared(new) || new.review.state() != DeadlineReviewState::Pending {
            return Err(inconsistent(
                "technical legacy successor cannot qualify tracking",
            ));
        }
        if !preservation::same_body(hasher, previous, next)? {
            return Err(inconsistent(
                "technical legacy successor replaced historical evidence",
            ));
        }
        return cause_matches(None, new, cause);
    };
    if old.policies != new.policies {
        return Err(inconsistent(
            "technical successor changed declared tracking policies",
        ));
    }
    if new.review.state() == DeadlineReviewState::LegacyUndeclared
        || (old.review.state() != DeadlineReviewState::Accepted
            && new.review.state() == DeadlineReviewState::Accepted)
        || old
            .review
            .reasons()
            .iter()
            .any(|reason| !new.review.reasons().contains(reason))
    {
        return Err(inconsistent(
            "technical successor cleared required human review",
        ));
    }
    observations_advance(old, new)?;
    require_advanced_reviews(old, new)?;
    cause_matches(Some(old), new, cause)?;
    if preservation::same_body(hasher, previous, next)? {
        return Ok(());
    }
    calendar::validate(hasher, previous, next, cause)
}

fn observations_advance(
    old: &DeadlineTrackingCapture,
    new: &DeadlineTrackingCapture,
) -> Result<(), ApplicationError> {
    for previous in &old.observations.entries {
        let next = new
            .observations
            .entries
            .iter()
            .find(|entry| entry.role == previous.role)
            .ok_or_else(|| inconsistent("technical successor removed an observation"))?;
        if !same_identity(previous, next)
            || next.revision < previous.revision
            || (next.revision == previous.revision && previous != next)
        {
            return Err(inconsistent(
                "technical observation changed identity or regressed",
            ));
        }
    }
    if new.observations.entries.iter().any(|entry| {
        entry.role != ObservationRole::NotificationParent
            && !old
                .observations
                .entries
                .iter()
                .any(|old| old.role == entry.role)
    }) {
        return Err(inconsistent(
            "technical successor added a selected dependency",
        ));
    }
    Ok(())
}
/// A notification parent is part of its source qualification. Each followed
/// dependency advanced in this capture needs its own review, even when another
/// dependency supplied the event. The repository establishes actual retirement.
fn require_advanced_reviews(
    old: &DeadlineTrackingCapture,
    new: &DeadlineTrackingCapture,
) -> Result<(), ApplicationError> {
    for entry in &new.observations.entries {
        let advanced = old
            .observations
            .entries
            .iter()
            .find(|previous| previous.role == entry.role)
            .is_none_or(|previous| entry.revision > previous.revision);
        if !advanced {
            continue;
        }
        let (policy, dependency, changed) = match entry.role {
            ObservationRole::Profile => (
                old.policies.profile,
                TrackingDependency::Profile,
                TrackingReviewReason::ProfileChanged,
            ),
            ObservationRole::Source | ObservationRole::NotificationParent => (
                old.policies.source,
                TrackingDependency::Source,
                TrackingReviewReason::SourceChanged,
            ),
            ObservationRole::Calendar => continue,
        };
        if policy != TrackingPolicy::Follow {
            continue;
        }
        let explained = new.review.reasons().iter().any(|requirement| {
            requirement.dependency == dependency
                && (requirement.reason == changed
                    || requirement.reason == TrackingReviewReason::DependencyRetired)
        });
        if new.review.state() != DeadlineReviewState::Pending || !explained {
            return Err(inconsistent(
                "followed dependency advanced without its required review",
            ));
        }
    }
    Ok(())
}

fn same_identity(previous: &ObservationEntry, next: &ObservationEntry) -> bool {
    previous.role == next.role
        && previous.family == next.family
        && previous.id == next.id
        && previous.case_id == next.case_id
        && previous.hearing_id == next.hearing_id
        && previous.parent_resolution.map(|p| p.id) == next.parent_resolution.map(|p| p.id)
}

fn cause_matches(
    old: Option<&DeadlineTrackingCapture>,
    new: &DeadlineTrackingCapture,
    cause: TechnicalCause,
) -> Result<(), ApplicationError> {
    match cause {
        TechnicalCause::LegacyBootstrap { .. } => {
            if old
                .is_some_and(|value| value.review.state() != DeadlineReviewState::LegacyUndeclared)
            {
                return Err(inconsistent(
                    "bootstrap requires an undeclared legacy predecessor",
                ));
            }
        }
        TechnicalCause::SourceEvent { event, .. } => {
            let observed = new
                .observations
                .entries
                .iter()
                .find(|entry| {
                    entry.family == event.family
                        && entry.id == event.source_id
                        && entry.revision == event.revision
                        && entry.case_id == event.case_id
                        && entry.hearing_id == event.hearing_id
                })
                .ok_or_else(|| {
                    inconsistent("technical event differs from its observed revision")
                })?;
            if old
                .and_then(|old| {
                    old.observations
                        .entries
                        .iter()
                        .find(|entry| entry.role == observed.role)
                })
                .is_some_and(|entry| entry.revision >= event.revision)
            {
                return Err(inconsistent("technical event has already been observed"));
            }
        }
    }
    Ok(())
}
