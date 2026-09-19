use super::{
    response,
    tracking_context_test_support::notification_parent_event,
    tracking_response_test_support::{case, digest, records},
};
use crate::error::ApiError;
use application::{
    cases::{CaseActorSnapshot, CaseAdministrationSnapshot, CurrentCaseAdministration},
    deadline_reevaluation::*,
    deadline_tracking::*,
    deadlines::*,
};
use domain::{
    case_administration::{CaseAdministrationValues, CaseAdministrativeStatus, CaseRevision},
    cases::CaseMetadata,
    procedural_facts::FactText,
};
use serde_json::Value;
use uuid::Uuid;
// Fixed digests exercise transport consistency, not cryptographic verification.
fn project(value: DeadlineDetail) -> Result<Value, ApiError> {
    let (case_id, id, revision) = (value.case_id, value.id, value.revision);
    response::detail(value, case_id, id, Some(revision))
}
pub(super) fn metadata(value: &mut DeadlineDetail) -> &mut DeadlineTrackedReceipt {
    let DeadlineReceiptVersion::Tracked(metadata) = &mut value.receipt.version else {
        unreachable!()
    };
    metadata
}
pub(super) fn pending(
    dependency: TrackingDependency,
    reason: TrackingReviewReason,
) -> TrackingReview {
    TrackingReview::new(
        DeadlineReviewState::Pending,
        vec![TrackingReviewRequirement { dependency, reason }],
    )
    .unwrap()
}
pub(super) fn technical() -> DeadlineDetail {
    let mut value = records::tracked_fixture();
    value.revision = DeadlineRevision::new(2).unwrap();
    value.reason = Some(FactText::new("Observed profile change").unwrap());
    value.receipt.action = DeadlineAction::Reevaluate;
    value.receipt.expected_revision = 1;
    value.receipt.operation_id = DeadlineOperationId::new();
    value.recorded_by = DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    };
    let tracking = value.tracking.as_mut().unwrap();
    tracking.observations.entries[0].revision = 3;
    tracking.review = pending(
        TrackingDependency::Profile,
        TrackingReviewReason::ProfileChanged,
    );
    let profile = tracking.observations.entries[0].clone();
    let receipt = metadata(&mut value);
    receipt.predecessor = Some(PredecessorReceipt {
        submission_digest: digest(),
        capture_digest: digest(),
    });
    receipt.cause = Some(TechnicalCause::SourceEvent {
        job_id: Uuid::nil(),
        event: SourceEventReference {
            sequence: 9_007_199_254_740_993,
            family: profile.family,
            source_id: profile.id,
            revision: 2,
            case_id: profile.case_id,
            hearing_id: None,
            operation_id: Uuid::from_u128(8),
        },
    });
    value
}
fn bootstrap() -> DeadlineDetail {
    let mut value = technical();
    metadata(&mut value).cause = Some(TechnicalCause::LegacyBootstrap {
        job_id: Uuid::nil(),
        policy_version: 1,
    });
    let tracking = value.tracking.as_mut().unwrap();
    tracking.policies = TrackingPolicies {
        profile: TrackingPolicy::Undetermined,
        source: TrackingPolicy::Undetermined,
        calendar: TrackingPolicy::Undetermined,
    };
    tracking.review = TrackingReview::new(
        DeadlineReviewState::Pending,
        [TrackingDependency::Profile, TrackingDependency::Source]
            .map(|dependency| TrackingReviewRequirement {
                dependency,
                reason: TrackingReviewReason::PolicyUndetermined,
            })
            .to_vec(),
    )
    .unwrap();
    value
}
fn recorded(status: CaseAdministrativeStatus) -> CurrentCaseAdministration {
    CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
        case_id: case(),
        revision: CaseRevision::FIRST,
        values: CaseAdministrationValues::basic(
            CaseMetadata::new("Updated case", "REF-1").unwrap(),
        )
        .with_status(status),
        values_digest: digest(),
        changed_at: records::instant(),
        changed_by: CaseActorSnapshot {
            id: records::actor(),
            email: "owner@example.com".into(),
        },
    }))
}
fn human(base: &DeadlineDetail, correction: bool) -> DeadlineHumanCommand {
    let change = if correction {
        DeadlineChange::Correct {
            expected_revision: base.revision,
            definition: base.definition.clone(),
            reason: FactText::new("Declared correction").unwrap(),
        }
    } else {
        DeadlineChange::Register {
            definition: base.definition.clone(),
        }
    };
    DeadlineHumanCommand::new(
        DeadlineCommand {
            operation_id: DeadlineOperationId::new(),
            deadline_id: base.id,
            change,
        },
        Some(records::tracking_policies()),
    )
    .unwrap()
}

#[test]
fn technical_events_accept_exact_or_older_observed_heads() {
    for revision in [2, 3] {
        let mut value = technical();
        let Some(TechnicalCause::SourceEvent { event, .. }) = &mut metadata(&mut value).cause
        else {
            unreachable!()
        };
        event.revision = revision;
        let result = project(value).unwrap();
        assert_eq!(
            result["receipt"]["version"]["cause"]["event"]["revision"],
            revision
        );
        assert_eq!(
            result["tracking"]["observations"]["entries"][0]["revision"],
            3
        );
        assert_eq!(result["calculation"]["profile"]["revision"], 1);
    }
}

#[test]
fn technical_events_reject_unobserved_identity_scope_family_or_revision() {
    for mutation in 0..5 {
        let mut value = technical();
        let Some(TechnicalCause::SourceEvent { event, .. }) = &mut metadata(&mut value).cause
        else {
            unreachable!()
        };
        match mutation {
            0 => event.source_id = Uuid::from_u128(99),
            1 => event.revision = 4,
            2 => event.case_id = None,
            3 => {
                event.family = DependencyFamily::Calendar;
                event.case_id = None;
            }
            _ => {
                event.family = DependencyFamily::HearingResult;
                event.hearing_id = Some(Uuid::nil());
            }
        }
        assert!(project(value).is_err(), "mutation {mutation}");
    }
}

#[test]
fn technical_resolution_event_can_belong_to_the_independent_notification_parent() {
    let result = project(notification_parent_event()).unwrap();
    assert_eq!(
        result["receipt"]["version"]["cause"]["event"]["revision"],
        3
    );
    assert_eq!(
        result["tracking"]["observations"]["entries"][2]["role"],
        "notification_parent"
    );
    assert_eq!(
        result["tracking"]["observations"]["entries"][2]["revision"],
        4
    );
    assert_eq!(
        result["tracking"]["observations"]["entries"][1]["parent_resolution"]["revision"],
        1
    );
    assert_eq!(
        result["definition"]["input"]["selection"]["source"]["value"]["resolution"]["revision"],
        1
    );
}

#[test]
fn technical_reevaluation_cannot_produce_legacy_undeclared_tracking() {
    let mut value = bootstrap();
    value.tracking.as_mut().unwrap().review =
        TrackingReview::new(DeadlineReviewState::LegacyUndeclared, vec![]).unwrap();
    assert!(project(value).is_err());
}

#[test]
fn legacy_bootstrap_requires_pending_review_and_undeclared_policies() {
    assert!(project(bootstrap()).is_ok());
    for accepted in [false, true] {
        let mut value = bootstrap();
        let tracking = value.tracking.as_mut().unwrap();
        tracking.policies = records::tracking_policies();
        tracking.review = if accepted {
            tracking.observations.entries[0].revision = 1;
            TrackingReview::new(DeadlineReviewState::Accepted, vec![]).unwrap()
        } else {
            pending(
                TrackingDependency::Profile,
                TrackingReviewReason::ProfileChanged,
            )
        };
        assert!(project(value).is_err(), "accepted {accepted}");
    }
}

#[test]
fn qualification_rejects_divergent_or_closed_administration_in_draft_and_detail() {
    for correction in [false, true] {
        let base = records::fixture();
        let expected = human(&base, correction);
        for mutation in 0..3 {
            let mut draft = records::draft(expected.clone(), &base);
            let mut detail = records::tracked_change(expected.clone(), &base);
            let captured = recorded(if mutation == 2 {
                CaseAdministrativeStatus::Closed
            } else {
                CaseAdministrativeStatus::Active
            });
            if mutation != 1 {
                draft.tracking.administration = captured.clone();
                detail.tracking.as_mut().unwrap().administration = captured.clone();
            }
            if mutation != 0 {
                draft.calculation.material.administration = captured.clone();
                detail.calculation.material.administration = captured;
            }
            assert!(
                response::draft(draft, case(), &expected).is_err(),
                "draft {correction}/{mutation}"
            );
            assert!(project(detail).is_err(), "detail {correction}/{mutation}");
        }
    }
}

#[test]
fn qualification_accepts_coherent_active_administration_in_both_captures() {
    for correction in [false, true] {
        let base = records::fixture();
        let expected = human(&base, correction);
        let mut draft = records::draft(expected.clone(), &base);
        let mut detail = records::tracked_change(expected.clone(), &base);
        let captured = recorded(CaseAdministrativeStatus::Active);
        draft.tracking.administration = captured.clone();
        draft.calculation.material.administration = captured.clone();
        detail.tracking.as_mut().unwrap().administration = captured.clone();
        detail.calculation.material.administration = captured;
        assert!(response::draft(draft, case(), &expected).is_ok());
        assert!(project(detail).is_ok());
    }
}

#[test]
fn attention_and_retirement_preserve_inherited_closed_tracking_administration() {
    for retire in [false, true] {
        let mut base = technical();
        base.tracking.as_mut().unwrap().administration = recorded(CaseAdministrativeStatus::Closed);
        let reason = FactText::new("Declared action").unwrap();
        let change = if retire {
            DeadlineChange::Retire {
                expected_revision: base.revision,
                reason,
            }
        } else {
            DeadlineChange::SetAttention {
                expected_revision: base.revision,
                attention: DeadlineAttention::Pending,
                reason,
            }
        };
        let expected = DeadlineHumanCommand::new(
            DeadlineCommand {
                operation_id: DeadlineOperationId::new(),
                deadline_id: base.id,
                change,
            },
            None,
        )
        .unwrap();
        let draft = records::draft(expected.clone(), &base);
        assert!(response::draft(draft, case(), &expected).is_ok());
        assert!(project(records::tracked_change(expected, &base)).is_ok());
    }
}
