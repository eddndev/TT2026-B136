use super::*;
use crate::{
    cases::case_administration_digest,
    deadline_reevaluation::{
        encode_observations, DependencyFamily, ObservationEntry, ObservationRole,
    },
    deadline_tracking::{
        DeadlineReviewState, TrackingDependency, TrackingPolicy, TrackingReviewReason,
    },
    ApplicationError,
};
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    deadline_triggers::TriggerSourceRef,
    procedural_facts::FactDeclaration,
};

/// Validate metadata shared by complete deadline states and their stored suffix.
/// The selected legal inputs remain the responsibility of the complete state.
pub(super) fn validate_capture(
    hasher: &dyn DocumentHasher,
    value: &DeadlineTrackingCapture,
) -> Result<Sha256Digest, ApplicationError> {
    let bytes = encode_observations(&value.observations)
        .map_err(|error| inconsistent(&error.to_string()))?;
    if let Some(admin) = value.administration.snapshot() {
        if admin.case_id != value.observations.case_id
            || case_administration_digest(hasher, &admin.values) != admin.values_digest
        {
            return Err(inconsistent("tracked administration differs"));
        }
    }
    let present = |role| {
        value
            .observations
            .entries
            .iter()
            .any(|entry| entry.role == role)
    };
    let notification = value.observations.entries.iter().any(|entry| {
        entry.role == ObservationRole::Source && entry.family == DependencyFamily::Notification
    });
    if notification
        && value.review.state() == DeadlineReviewState::Accepted
        && !present(ObservationRole::NotificationParent)
    {
        return Err(inconsistent(
            "accepted notification requires observed parent head",
        ));
    }
    value
        .review
        .validate_policies(
            &value.policies,
            [
                true,
                present(ObservationRole::Source),
                present(ObservationRole::Calendar),
            ],
        )
        .map_err(|error| inconsistent(&error.to_string()))?;
    Ok(hasher.hash_bytes(&bytes))
}

pub(super) fn validate(
    hasher: &dyn DocumentHasher,
    definition: &DeadlineDefinition,
    calculation: &DeadlineCalculation,
    value: &DeadlineTrackingCapture,
) -> Result<Sha256Digest, ApplicationError> {
    let digest = validate_capture(hasher, value)?;
    let case_id = definition.input.selection.case_id;
    if value.observations.case_id != case_id {
        return Err(inconsistent("tracked observations belong to another case"));
    }
    let find = |role| {
        value
            .observations
            .entries
            .iter()
            .find(|entry| entry.role == role)
    };
    let profile =
        find(ObservationRole::Profile).ok_or_else(|| inconsistent("observed profile absent"))?;
    let expected_scope = match calculation.profile.definition.scope() {
        crate::deadline_profiles::DeadlineProfileScope::Global(_) => None,
        crate::deadline_profiles::DeadlineProfileScope::Case(case) => Some(*case),
    };
    if profile.id != definition.profile.id.as_uuid() || profile.case_id != expected_scope {
        return Err(inconsistent("observed profile identity or scope differs"));
    }
    selected_revision(
        profile,
        definition.profile.revision.get(),
        value.policies.profile,
        value.review.state(),
    )?;
    let source = find(ObservationRole::Source);
    match (&definition.input.selection.source, source) {
        (FactDeclaration::Unknown(_), None) => {}
        (FactDeclaration::Known(selected), Some(observed)) => {
            let (family, id, revision, hearing, parent) = match selected {
                TriggerSourceRef::Resolution(reference) => (
                    DependencyFamily::Resolution,
                    reference.id.as_uuid(),
                    reference.revision.get(),
                    None,
                    None,
                ),
                TriggerSourceRef::Notification {
                    id,
                    revision,
                    resolution,
                } => (
                    DependencyFamily::Notification,
                    id.as_uuid(),
                    revision.get(),
                    None,
                    Some(resolution.id.as_uuid()),
                ),
                TriggerSourceRef::HearingResult(reference) => (
                    DependencyFamily::HearingResult,
                    reference.result_id.as_uuid(),
                    reference.revision.get(),
                    Some(reference.hearing_id.as_uuid()),
                    None,
                ),
            };
            if observed.family != family
                || observed.id != id
                || observed.hearing_id != hearing
                || observed.parent_resolution.map(|p| p.id) != parent
            {
                return Err(inconsistent(
                    "observed source differs from selected identity",
                ));
            }
            selected_revision(
                observed,
                revision,
                value.policies.source,
                value.review.state(),
            )?;
        }
        _ => return Err(inconsistent("observed source presence differs")),
    }
    let calendar = find(ObservationRole::Calendar);
    match (definition.input.calendar, calendar) {
        (None, None) => {}
        (Some(selected), Some(observed)) if observed.id == selected.id.as_uuid() => {
            selected_revision(
                observed,
                selected.revision.get(),
                value.policies.calendar,
                value.review.state(),
            )?;
        }
        _ => return Err(inconsistent("observed calendar differs")),
    }
    Ok(digest)
}
fn selected_revision(
    observed: &ObservationEntry,
    selected: u32,
    policy: TrackingPolicy,
    state: DeadlineReviewState,
) -> Result<(), ApplicationError> {
    if observed.revision < selected
        || (state == DeadlineReviewState::Accepted
            && policy == TrackingPolicy::Follow
            && observed.revision != selected)
    {
        return Err(inconsistent(
            "selected and observed tracking revisions disagree",
        ));
    }
    Ok(())
}

pub(super) fn append(
    bytes: &mut Vec<u8>,
    hasher: &dyn DocumentHasher,
    value: &DeadlineTrackingCapture,
    observations_digest: Sha256Digest,
    capture: bool,
) {
    for policy in [
        value.policies.profile,
        value.policies.source,
        value.policies.calendar,
    ] {
        bytes.push(match policy {
            TrackingPolicy::Undetermined => 0,
            TrackingPolicy::Fixed => 1,
            TrackingPolicy::Follow => 2,
        });
    }
    bytes.push(match value.review.state() {
        DeadlineReviewState::LegacyUndeclared => 0,
        DeadlineReviewState::Accepted => 1,
        DeadlineReviewState::Pending => 2,
    });
    bytes.push(value.review.reasons().len() as u8);
    for reason in value.review.reasons() {
        bytes.push(match reason.dependency {
            TrackingDependency::Profile => 0,
            TrackingDependency::Source => 1,
            TrackingDependency::Calendar => 2,
        });
        bytes.push(match reason.reason {
            TrackingReviewReason::SourceChanged => 0,
            TrackingReviewReason::ProfileChanged => 1,
            TrackingReviewReason::DependencyRetired => 2,
            TrackingReviewReason::PolicyUndetermined => 3,
        });
    }
    bytes.extend_from_slice(observations_digest.as_bytes());
    if capture {
        evidence::optional(bytes, value.administration.snapshot(), |bytes, admin| {
            bytes.extend_from_slice(&admin.revision.get().to_be_bytes())
        });
        bytes.extend_from_slice(
            case_administration_digest(hasher, &value.administration.values()).as_bytes(),
        );
        let mut administration = Vec::new();
        evidence::administration(&mut administration, hasher, &value.administration);
        bytes.extend_from_slice(hasher.hash_bytes(&administration).as_bytes());
    }
}
