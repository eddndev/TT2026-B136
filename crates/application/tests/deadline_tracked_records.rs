#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_technical_support;
mod deadline_tracked_support;

use application::{
    cases::CurrentCaseAdministration, deadline_currentness::DeadlineCurrent,
    deadline_reevaluation::*, deadline_tracking::*, deadlines::*,
};
use deadline_support::evaluation::inputs;
use deadline_tracked_support::*;
use domain::{cases::CaseMetadata, crypto::Sha256Digest};
use uuid::Uuid;

fn historical_overview(detail: &DeadlineDetail) -> DeadlineOverview {
    DeadlineOverview::from(&DeadlineCurrent::historical(inputs::hasher().as_ref(), detail).unwrap())
}

#[test]
fn tracked_state_and_history_use_explicit_new_versions() {
    let value = accepted();
    let hasher = inputs::hasher();
    deadline_receipt_matches(hasher.as_ref(), &value).unwrap();
    assert_eq!(
        &deadline_review_bytes(hasher.as_ref(), &value).unwrap()[..5],
        b"DLRV2"
    );
    assert_eq!(
        &deadline_capture_bytes(hasher.as_ref(), &value).unwrap()[..5],
        b"DLST2"
    );
    assert_eq!(
        &deadline_record_submission_bytes(&value).unwrap()[..5],
        b"DLTX2"
    );
    let history = DeadlineHistoryEntry::from_detail(hasher.as_ref(), &value).unwrap();
    deadline_history_receipt_matches(hasher.as_ref(), &history).unwrap();
}

#[test]
fn pending_capture_preserves_historical_due_and_removes_operational_projection() {
    let before = accepted();
    let after = pending(&before);
    deadline_receipt_matches(inputs::hasher().as_ref(), &after).unwrap();
    assert_eq!(after.calculation, before.calculation);
    assert!(after.calculation.result.due_at().is_some());
    assert_eq!(
        before.operational_due_at(),
        before.calculation.result.due_at()
    );
    assert_eq!(after.operational_due_at(), None);
    let summary = historical_overview(&after);
    assert!(!summary.calculation_blocked);
    assert_eq!(
        summary.calculation_due_at,
        after.calculation.result.due_at()
    );
    assert_eq!(summary.operational.due_at(), None);
    assert_eq!(summary.review_state, DeadlineReviewState::Pending);
    assert_eq!(legacy().operational_due_at(), None);
}

#[test]
fn policies_review_and_observations_are_committed_by_the_receipt() {
    let original = accepted();
    for change in 0..5 {
        let mut altered = original.clone();
        let tracking = altered.tracking.as_mut().unwrap();
        match change {
            0 => tracking.policies.source = TrackingPolicy::Fixed,
            1 => {
                tracking.review = TrackingReview::new(
                    DeadlineReviewState::Pending,
                    vec![TrackingReviewRequirement {
                        dependency: TrackingDependency::Source,
                        reason: TrackingReviewReason::DependencyRetired,
                    }],
                )
                .unwrap()
            }
            2 => {
                tracking.observations.entries[1].evidence_digest = Sha256Digest::from_array([9; 32])
            }
            3 => {
                tracking.observations.entries[0].submission_digest =
                    Sha256Digest::from_array([8; 32])
            }
            _ => tracking.observations.entries[1].revision = 2,
        }
        assert!(
            deadline_receipt_matches(inputs::hasher().as_ref(), &altered).is_err(),
            "change {change}"
        );
    }
}

#[test]
fn tracked_receipt_and_capture_presence_must_agree_without_fallback() {
    let mut value = accepted();
    value.tracking = None;
    assert!(deadline_receipt_matches(inputs::hasher().as_ref(), &value).is_err());
    value = accepted();
    value.receipt.version = DeadlineReceiptVersion::Legacy;
    assert!(deadline_receipt_matches(inputs::hasher().as_ref(), &value).is_err());
}

#[test]
fn outer_administration_changes_only_capture_and_preserves_calculation() {
    let before = accepted();
    let mut after = before.clone();
    after.tracking.as_mut().unwrap().administration = CurrentCaseAdministration::Unrevised(
        CaseMetadata::new("New outer observation", "OUTER-2").unwrap(),
    );
    let hasher = inputs::hasher();
    assert_eq!(before.calculation, after.calculation);
    assert_eq!(
        deadline_review_bytes(hasher.as_ref(), &before).unwrap(),
        deadline_review_bytes(hasher.as_ref(), &after).unwrap()
    );
    assert_ne!(
        deadline_capture_bytes(hasher.as_ref(), &before).unwrap(),
        deadline_capture_bytes(hasher.as_ref(), &after).unwrap()
    );
    assert!(deadline_receipt_matches(hasher.as_ref(), &after).is_err());
}

#[test]
fn tracked_shape_rejects_cross_case_wrong_dependency_and_implicit_policy() {
    for change in 0..5 {
        let mut value = accepted();
        let tracking = value.tracking.as_mut().unwrap();
        match change {
            0 => tracking.observations.case_id = domain::cases::CaseId::from_uuid(Uuid::nil()),
            1 => tracking.observations.entries[0].id = Uuid::nil(),
            2 => tracking.observations.entries[1].id = Uuid::nil(),
            3 => tracking.policies.source = TrackingPolicy::Undetermined,
            _ => tracking.observations.entries[1].revision = 2,
        }
        assert!(
            deadline_review_bytes(inputs::hasher().as_ref(), &value).is_err(),
            "change {change}"
        );
    }
}

#[test]
fn technical_author_is_not_a_human_registration_or_correction() {
    let mut value = accepted();
    value.recorded_by = DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    };
    assert!(deadline_record_submission_bytes(&value).is_err());
    let mut value = pending(&accepted());
    value.recorded_by = DeadlineActorSnapshot::User {
        id: inputs::actor(),
        email: "owner@example.com".into(),
    };
    assert!(deadline_record_submission_bytes(&value).is_err());
}

#[test]
fn retired_deadline_never_exposes_an_operational_due() {
    let value = deadline_technical_support::retired(&deadline_technical_support::accepted(
        TrackingPolicy::Follow,
    ));
    assert!(value.calculation.result.due_at().is_some());
    assert_eq!(value.operational_due_at(), None);
    let row = historical_overview(&value);
    assert!(!row.calculation_blocked);
    assert_eq!(row.calculation_due_at, value.calculation.result.due_at());
    assert_eq!(row.operational.due_at(), None);
}
