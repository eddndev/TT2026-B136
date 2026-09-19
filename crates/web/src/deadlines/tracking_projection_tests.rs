use super::{tracking_projection as projection, tracking_projection_test_support as fixture};
use application::{
    cases::{
        CaseActorSnapshot, CaseAdministrationSnapshot, CaseAdministrationValues,
        CaseAdministrativeStatus, CaseRevision, CurrentCaseAdministration,
    },
    deadline_tracking::{
        DeadlineReviewState, TrackingDependency, TrackingPolicies, TrackingPolicy, TrackingReview,
        TrackingReviewReason, TrackingReviewRequirement,
    },
};
use domain::{
    cases::{CaseId, CaseMetadata},
    clock::OffsetDateTime,
    identity::UserId,
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn tracking_http_capture_has_explicit_policies_review_and_preserved_administration() {
    let value = projection::capture(&fixture::capture(), fixture::case()).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 4);
    assert_eq!(
        value["policies"],
        json!({
            "profile": "follow", "source": "fixed", "calendar": "follow",
        })
    );
    assert_eq!(value["review"], json!({"state": "accepted", "reasons": []}));
    assert_eq!(
        value["administration"],
        json!({
            "kind": "unrevised", "title": "Case", "reference": "REF-1", "status": "active",
        })
    );
    assert_eq!(
        value["observations"]["entries"].as_array().unwrap().len(),
        4
    );
}

#[test]
fn tracking_http_absent_dependencies_remain_explicitly_undetermined() {
    let mut capture = fixture::capture();
    capture.observations.entries.truncate(1);
    capture.policies.source = TrackingPolicy::Undetermined;
    capture.policies.calendar = TrackingPolicy::Undetermined;
    let value = projection::capture(&capture, fixture::case()).unwrap();
    assert_eq!(
        value["policies"],
        json!({
            "profile": "follow", "source": "undetermined", "calendar": "undetermined",
        })
    );
    for declared in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
        capture.policies.source = declared;
        assert!(projection::capture(&capture, fixture::case()).is_err());
    }
    capture.policies.source = TrackingPolicy::Undetermined;
    capture.policies.profile = TrackingPolicy::Undetermined;
    assert!(projection::capture(&capture, fixture::case()).is_err());
}

#[test]
fn tracking_http_pending_reasons_preserve_domain_order_and_all_reason_names() {
    let review = TrackingReview::new(
        DeadlineReviewState::Pending,
        vec![
            TrackingReviewRequirement {
                dependency: TrackingDependency::Profile,
                reason: TrackingReviewReason::ProfileChanged,
            },
            TrackingReviewRequirement {
                dependency: TrackingDependency::Source,
                reason: TrackingReviewReason::SourceChanged,
            },
            TrackingReviewRequirement {
                dependency: TrackingDependency::Calendar,
                reason: TrackingReviewReason::DependencyRetired,
            },
            TrackingReviewRequirement {
                dependency: TrackingDependency::Calendar,
                reason: TrackingReviewReason::PolicyUndetermined,
            },
        ],
    )
    .unwrap();
    let mut capture = fixture::capture();
    capture.review = review;
    capture.policies.source = TrackingPolicy::Follow;
    capture.policies.calendar = TrackingPolicy::Undetermined;
    let value = projection::capture(&capture, fixture::case()).unwrap();
    assert_eq!(
        value["review"],
        json!({"state": "pending", "reasons": [
            {"dependency": "profile", "reason": "profile_changed"},
            {"dependency": "source", "reason": "source_changed"},
            {"dependency": "calendar", "reason": "dependency_retired"},
            {"dependency": "calendar", "reason": "policy_undetermined"},
        ]})
    );
    capture.policies.source = TrackingPolicy::Fixed;
    assert!(projection::capture(&capture, fixture::case()).is_err());
}

#[test]
fn tracking_http_legacy_notification_does_not_invent_observed_parent_or_acceptance() {
    let mut capture = fixture::capture();
    capture.observations.entries.pop();
    assert!(projection::capture(&capture, fixture::case()).is_err());
    capture.review = TrackingReview::new(DeadlineReviewState::LegacyUndeclared, vec![]).unwrap();
    capture.policies = TrackingPolicies {
        profile: TrackingPolicy::Undetermined,
        source: TrackingPolicy::Undetermined,
        calendar: TrackingPolicy::Undetermined,
    };
    let value = projection::capture(&capture, fixture::case()).unwrap();
    assert_eq!(
        value["review"],
        json!({"state": "legacy_undeclared", "reasons": []})
    );
    assert_eq!(
        value["observations"]["entries"].as_array().unwrap().len(),
        3
    );
    capture.policies.profile = TrackingPolicy::Follow;
    assert!(projection::capture(&capture, fixture::case()).is_err());
}

#[test]
fn tracking_http_recorded_closed_administration_keeps_a_human_author() {
    let mut capture = fixture::capture();
    let snapshot = CaseAdministrationSnapshot {
        case_id: fixture::case(),
        revision: CaseRevision::new(2).unwrap(),
        values: CaseAdministrationValues::basic(CaseMetadata::new("Case", "REF-1").unwrap())
            .with_status(CaseAdministrativeStatus::Closed),
        values_digest: fixture::digest(0x33),
        changed_at: OffsetDateTime::UNIX_EPOCH,
        changed_by: CaseActorSnapshot {
            id: UserId::from_uuid(Uuid::nil()),
            email: "owner@example.test".into(),
        },
    };
    capture.administration = CurrentCaseAdministration::Recorded(Box::new(snapshot.clone()));
    let value = projection::capture(&capture, fixture::case()).unwrap();
    assert_eq!(
        value["administration"],
        json!({
            "kind": "recorded", "case_id": Uuid::nil(), "revision": 2,
            "title": "Case", "reference": "REF-1", "status": "closed",
            "values_digest": "33".repeat(32),
            "changed_at": {"unix_seconds": 0, "nanosecond": 0, "offset_seconds": 0},
            "changed_by": {"id": Uuid::nil(), "email": "owner@example.test"},
        })
    );
    let mut wrong_case = snapshot;
    wrong_case.case_id = CaseId::from_uuid(Uuid::from_u128(99));
    capture.administration = CurrentCaseAdministration::Recorded(Box::new(wrong_case));
    assert!(projection::capture(&capture, fixture::case()).is_err());
}
