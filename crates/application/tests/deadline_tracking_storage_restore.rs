#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_tracked_support;
mod deadline_tracking_storage_support;

use application::{
    deadline_reevaluation::{DependencyFamily, ObservationRole, ResolutionReference},
    deadline_tracking::*,
    deadlines::{
        deadline_tracking_capture_bytes, decode_deadline_tracking_capture, DeadlineTrackingCapture,
    },
};
use deadline_tracking_storage_support::*;
use domain::{cases::CaseId, crypto::Sha256Digest};
use uuid::Uuid;

fn reject_capture(value: &DeadlineTrackingCapture, header: &[u8]) {
    assert!(deadline_tracking_capture_bytes(inputs::hasher().as_ref(), value).is_err());
    rejects_restore(&frame(header, value), value);
}

#[test]
fn restore_verifies_every_byte_of_the_observations_commitment() {
    let value = capture();
    let bytes = frame(&[2, 2, 0, 1, 0], &value);
    for position in 5..37 {
        let mut altered = bytes.clone();
        altered[position] ^= 1;
        assert!(
            decode_deadline_tracking_capture(&altered)
                .unwrap()
                .restore(
                    inputs::hasher().as_ref(),
                    value.observations.clone(),
                    value.administration.clone()
                )
                .is_err(),
            "digest byte {position}"
        );
    }
}

#[test]
fn altered_observation_identity_revision_receipts_and_evidence_do_not_restore() {
    let original = capture();
    let bytes = frame(&[2, 2, 0, 1, 0], &original);
    for change in 0..6 {
        let mut value = original.clone();
        let entry = &mut value.observations.entries[0];
        match change {
            0 => entry.id = Uuid::from_u128(999),
            1 => entry.revision += 1,
            2 => entry.submission_digest = Sha256Digest::from_array([19; 32]),
            3 => entry.evidence_digest = Sha256Digest::from_array([29; 32]),
            4 => entry.case_id = None,
            _ => {
                let case_id = CaseId::from_uuid(Uuid::from_u128(999));
                value.observations.case_id = case_id;
                for entry in &mut value.observations.entries {
                    entry.case_id = Some(case_id);
                }
            }
        }
        rejects_restore(&bytes, &value);
    }
}

#[test]
fn encoder_and_restore_reject_noncanonical_observations_without_normalizing_them() {
    let original = capture();
    let bytes = frame(&[2, 2, 0, 1, 0], &original);
    for change in 0..7 {
        let mut value = original.clone();
        match change {
            0 => value.observations.entries.swap(0, 1),
            1 => value
                .observations
                .entries
                .push(value.observations.entries[1].clone()),
            2 => value.observations.entries.clear(),
            3 => {
                value.observations.entries.remove(0);
            }
            4 => value.observations.entries[0].family = DependencyFamily::Resolution,
            5 => value.observations.entries[0].revision = 0,
            _ => {
                value.observations.entries[1].case_id =
                    Some(CaseId::from_uuid(Uuid::from_u128(999)))
            }
        }
        assert!(
            deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).is_err(),
            "change {change}"
        );
        rejects_restore(&bytes, &value);
    }
}

#[test]
fn accepted_present_dependencies_require_explicit_policies() {
    for dependency in 0..3 {
        let mut value = with_calendar();
        let mut header = [2, 2, 2, 1, 0];
        header[dependency] = 0;
        match dependency {
            0 => value.policies.profile = TrackingPolicy::Undetermined,
            1 => value.policies.source = TrackingPolicy::Undetermined,
            _ => value.policies.calendar = TrackingPolicy::Undetermined,
        }
        reject_capture(&value, &header);
    }
}

#[test]
fn absent_dependencies_cannot_gain_declared_policies_during_restore() {
    for policy in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
        let tag = if policy == TrackingPolicy::Fixed {
            1
        } else {
            2
        };
        let mut value = capture();
        value.policies.calendar = policy;
        reject_capture(&value, &[2, 2, tag, 1, 0]);
        let mut value = capture();
        value.observations.entries.truncate(1);
        value.policies.source = policy;
        reject_capture(&value, &[2, tag, 0, 1, 0]);
    }
}

#[test]
fn pending_review_requires_each_undeclared_present_policy_reason() {
    let mut value = capture();
    value.policies.source = TrackingPolicy::Undetermined;
    value.review = TrackingReview::new(
        DeadlineReviewState::Pending,
        vec![requirement(
            TrackingDependency::Source,
            TrackingReviewReason::DependencyRetired,
        )],
    )
    .unwrap();
    reject_capture(&value, &[2, 0, 0, 2, 1, 1, 2]);
    value.review = TrackingReview::new(
        DeadlineReviewState::Pending,
        vec![
            requirement(
                TrackingDependency::Source,
                TrackingReviewReason::DependencyRetired,
            ),
            requirement(
                TrackingDependency::Source,
                TrackingReviewReason::PolicyUndetermined,
            ),
        ],
    )
    .unwrap();
    let bytes = frame(&[2, 0, 0, 2, 2, 1, 2, 1, 3], &value);
    roundtrip(&bytes, &value);
}

#[test]
fn policy_reason_conflicts_and_reasons_for_absent_dependencies_are_rejected() {
    for (policy, reason, tag, reason_tag) in [
        (
            TrackingPolicy::Fixed,
            TrackingReviewReason::SourceChanged,
            1,
            0,
        ),
        (
            TrackingPolicy::Follow,
            TrackingReviewReason::PolicyUndetermined,
            2,
            3,
        ),
    ] {
        let mut value = capture();
        value.policies.source = policy;
        value.review = TrackingReview::new(
            DeadlineReviewState::Pending,
            vec![requirement(TrackingDependency::Source, reason)],
        )
        .unwrap();
        reject_capture(&value, &[2, tag, 0, 2, 1, 1, reason_tag]);
    }
    let mut value = capture();
    value.review = TrackingReview::new(
        DeadlineReviewState::Pending,
        vec![requirement(
            TrackingDependency::Calendar,
            TrackingReviewReason::PolicyUndetermined,
        )],
    )
    .unwrap();
    reject_capture(&value, &[2, 2, 0, 2, 1, 2, 3]);
}

#[test]
fn legacy_review_does_not_infer_or_accept_a_declared_policy() {
    for dependency in 0..3 {
        let mut value = legacy();
        let mut header = [0, 0, 0, 0, 0];
        header[dependency] = 1;
        match dependency {
            0 => value.policies.profile = TrackingPolicy::Fixed,
            1 => value.policies.source = TrackingPolicy::Fixed,
            _ => value.policies.calendar = TrackingPolicy::Fixed,
        }
        reject_capture(&value, &header);
    }
}

#[test]
fn accepted_notification_requires_a_parent_observation_under_the_source_policy() {
    let mut value = capture();
    let mut parent = value.observations.entries[1].clone();
    parent.role = ObservationRole::NotificationParent;
    let source = &mut value.observations.entries[1];
    source.family = DependencyFamily::Notification;
    source.id = Uuid::from_u128(20);
    source.parent_resolution = Some(ResolutionReference {
        id: parent.id,
        revision: parent.revision,
    });
    reject_capture(&value, &[2, 2, 0, 1, 0]);
    value.observations.entries.push(parent);
    let bytes = frame(&[2, 2, 0, 1, 0], &value);
    roundtrip(&bytes, &value);
}
