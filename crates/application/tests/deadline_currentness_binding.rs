#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_currentness_support;
mod deadline_observation_support;
mod deadline_support;
mod deadline_technical_support;

use application::{
    deadline_currentness::{evaluate_deadline_currentness, DeadlineCurrent, DeadlineFreshness},
    deadline_tracking::{
        DeadlineReviewState, TrackingDependency, TrackingPolicy, TrackingReview,
        TrackingReviewReason, TrackingReviewRequirement,
    },
    deadlines::{deadline_capture_bytes, DeadlineId, DeadlineRevision, DeadlineStatus},
};
use deadline_currentness_support::{checked_at, unknown_source};
use deadline_technical_support::*;
use domain::{cases::CaseId, crypto::Sha256Digest};
use uuid::Uuid;

#[test]
fn historical_projection_preserves_verified_evidence_without_claiming_currentness() {
    let base = accepted(TrackingPolicy::Follow);
    let before = deadline_capture_bytes(inputs::hasher().as_ref(), &base).unwrap();
    assert!(base.calculation.result.due_at().is_some());
    let result = DeadlineCurrent::historical(inputs::hasher().as_ref(), &base).unwrap();
    assert_eq!(result.detail(), &base);
    assert_eq!(
        deadline_capture_bytes(inputs::hasher().as_ref(), result.detail()).unwrap(),
        before
    );
    let operational = result.operational();
    assert_eq!(operational.freshness(), DeadlineFreshness::NotChecked);
    assert_eq!(operational.checked_at(), None);
    assert_eq!(operational.changed_dependencies(), &[]);
    assert_eq!(operational.due_at(), None);
    assert!(operational.matches_capture(&base));
}

#[test]
fn historical_projection_rejects_corruption_before_suppressing_the_due() {
    for mut base in [legacy(), accepted(TrackingPolicy::Fixed)] {
        base.receipt.capture_digest = Sha256Digest::from_array([0; 32]);
        assert!(DeadlineCurrent::historical(inputs::hasher().as_ref(), &base).is_err());
    }
}

#[test]
fn a_cloned_operational_view_cannot_be_attached_to_another_capture_or_summary() {
    let base = accepted(TrackingPolicy::Follow);
    let current = evaluate_deadline_currentness(
        inputs::hasher().as_ref(),
        &base,
        Some(&heads(&base)),
        checked_at(),
    )
    .unwrap();
    let copied = current.operational().clone();
    let (detail, operational) = current.into_parts();
    assert_eq!(copied, operational);
    assert!(copied.matches_capture(&detail));
    for kind in 0..7 {
        let mut incompatible = detail.clone();
        match kind {
            0 => incompatible.id = DeadlineId::from_uuid(Uuid::from_u128(999)),
            1 => incompatible.case_id = CaseId::from_uuid(Uuid::from_u128(999)),
            2 => incompatible.revision = DeadlineRevision::new(2).unwrap(),
            3 => incompatible.receipt.capture_digest = Sha256Digest::from_array([0; 32]),
            4 => incompatible.status = DeadlineStatus::Retired,
            5 => {
                incompatible.tracking.as_mut().unwrap().review = TrackingReview::new(
                    DeadlineReviewState::Pending,
                    vec![TrackingReviewRequirement {
                        dependency: TrackingDependency::Source,
                        reason: TrackingReviewReason::SourceChanged,
                    }],
                )
                .unwrap();
            }
            _ => incompatible.calculation.result = unknown_source().calculation.result,
        }
        assert!(!copied.matches_capture(&incompatible), "{kind}");
    }
}
