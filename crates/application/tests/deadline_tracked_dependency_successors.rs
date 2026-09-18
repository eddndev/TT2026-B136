#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_tracked_support;

use application::{
    deadline_evaluations::{evaluate_profiled_deadline, DeadlineEvaluationRecord},
    deadline_inputs::{DeadlineInputMaterial, DeadlineSourceDetail},
    deadline_profiles::{DeadlineProfileDefinition, DeadlineProfileDetail},
    deadline_reevaluation::*,
    deadline_tracking::*,
    deadlines::*,
    procedural_facts::FactDetail,
};
use deadline_observation_support::{build, replace_profile, resign_profile};
use deadline_support::evaluation::{inputs, text};
use deadline_tracked_support::{accepted, resign};
use domain::deadline_triggers::{TriggerField, TriggerRequirement};
use uuid::Uuid;

fn base(notification: bool) -> DeadlineDetail {
    let mut value = accepted();
    let parent = notification.then(|| inputs::resolution(1, false, "2026-01-01"));
    if notification {
        let mut definition =
            deadline_observation_support::profile_input(&value.calculation.profile);
        definition.trigger = TriggerRequirement::SourceField(TriggerField::NotificationPracticedAt);
        value.calculation.profile.definition = DeadlineProfileDefinition::new(definition).unwrap();
        resign_profile(&mut value.calculation.profile);
        let source =
            DeadlineSourceDetail::Fact(Box::new(inputs::notification(1, false, 1, "2026-01-06")));
        value.definition.input.selection = inputs::request(&source).trigger;
        value.calculation.material = inputs::material(source);
        let result = evaluate_profiled_deadline(
            inputs::hasher().as_ref(),
            &value.calculation.profile.definition,
            &value.definition.input,
            &value.calculation.material,
        )
        .unwrap();
        value.calculation.result = DeadlineEvaluationRecord::capture(&result);
    }
    value.tracking.as_mut().unwrap().observations = build(
        &value.calculation.profile,
        &value.calculation.material,
        parent.as_ref(),
    )
    .unwrap();
    resign(&mut value);
    deadline_receipt_matches(inputs::hasher().as_ref(), &value).unwrap();
    value
}
fn review(reasons: &[(TrackingDependency, TrackingReviewReason)]) -> TrackingReview {
    TrackingReview::new(
        if reasons.is_empty() {
            DeadlineReviewState::Accepted
        } else {
            DeadlineReviewState::Pending
        },
        reasons
            .iter()
            .map(|(dependency, reason)| TrackingReviewRequirement {
                dependency: *dependency,
                reason: *reason,
            })
            .collect(),
    )
    .unwrap()
}
fn successor(
    base: &DeadlineDetail,
    profile: &DeadlineProfileDetail,
    material: &DeadlineInputMaterial,
    parent: Option<&FactDetail>,
    cause_role: ObservationRole,
    reasons: &[(TrackingDependency, TrackingReviewReason)],
) -> DeadlineDetail {
    let observations = build(profile, material, parent).unwrap();
    let observed = observations
        .entries
        .iter()
        .find(|value| value.role == cause_role)
        .unwrap();
    let operation_id = match cause_role {
        ObservationRole::Profile => profile.receipt.operation_id.as_uuid(),
        ObservationRole::NotificationParent => parent
            .unwrap()
            .snapshot
            .metadata()
            .receipt
            .operation_id
            .as_uuid(),
        ObservationRole::Source => deadline_observation_support::fact(&material.source_head)
            .snapshot
            .metadata()
            .receipt
            .operation_id
            .as_uuid(),
        ObservationRole::Calendar => unreachable!(),
    };
    let cause = TechnicalCause::SourceEvent {
        job_id: Uuid::from_u128(901),
        event: SourceEventReference {
            sequence: 10,
            family: observed.family,
            source_id: observed.id,
            revision: observed.revision,
            case_id: observed.case_id,
            hearing_id: observed.hearing_id,
            operation_id,
        },
    };
    let mut value = base.clone();
    value.revision = base.revision.next().unwrap();
    value.reason = Some(text("Observe exact dependency revisions"));
    value.recorded_by = DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    };
    value.receipt.action = DeadlineAction::Reevaluate;
    value.receipt.expected_revision = base.revision.get();
    value.receipt.operation_id =
        DeadlineOperationId::from_uuid(Uuid::from_u128(900 + u128::from(value.revision.get())));
    let DeadlineReceiptVersion::Tracked(metadata) = &mut value.receipt.version else {
        unreachable!();
    };
    metadata.predecessor = Some(PredecessorReceipt {
        submission_digest: base.receipt.submission_digest,
        capture_digest: base.receipt.capture_digest,
    });
    metadata.cause = Some(cause);
    let tracking = value.tracking.as_mut().unwrap();
    tracking.review = review(reasons);
    tracking.observations = observations;
    resign(&mut value);
    deadline_receipt_matches(inputs::hasher().as_ref(), &value).unwrap();
    value
}
fn parent_successor(
    base: &DeadlineDetail,
    reasons: &[(TrackingDependency, TrackingReviewReason)],
) -> DeadlineDetail {
    successor(
        base,
        &base.calculation.profile,
        &base.calculation.material,
        Some(&inputs::resolution(2, false, "2026-01-01")),
        ObservationRole::NotificationParent,
        reasons,
    )
}
fn assert_link(previous: &DeadlineDetail, next: &DeadlineDetail, allowed: bool) {
    let result = deadline_successor_matches(inputs::hasher().as_ref(), previous, next);
    assert_eq!(
        result.is_ok(),
        allowed,
        "unexpected successor decision: {result:?}"
    );
}
fn both_heads(base: &DeadlineDetail) -> (DeadlineProfileDetail, DeadlineInputMaterial) {
    let mut profile = base.calculation.profile.clone();
    replace_profile(&mut profile, false);
    let mut material = base.calculation.material.clone();
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
        2,
        false,
        "2026-01-10",
    ))));
    (profile, material)
}

#[test]
fn a_followed_notification_parent_cannot_advance_while_the_deadline_stays_accepted() {
    let previous = base(true);
    assert!(previous.operational_due_at().is_some());
    let next = parent_successor(&previous, &[]);
    assert_link(&previous, &next, false);
}

#[test]
fn a_followed_notification_parent_requires_a_source_review_reason() {
    let previous = base(true);
    let wrong = parent_successor(
        &previous,
        &[(
            TrackingDependency::Profile,
            TrackingReviewReason::ProfileChanged,
        )],
    );
    assert_link(&previous, &wrong, false);
    let next = parent_successor(
        &previous,
        &[(
            TrackingDependency::Source,
            TrackingReviewReason::SourceChanged,
        )],
    );
    assert_link(&previous, &next, true);
    assert!(next.operational_due_at().is_none());
    assert_eq!(next.calculation, previous.calculation);
    assert_eq!(next.attention, previous.attention);
    assert_eq!(next.responsible, previous.responsible);
}

#[test]
fn a_fixed_notification_parent_keeps_the_accepted_historical_selection() {
    let mut previous = base(true);
    previous.tracking.as_mut().unwrap().policies.source = TrackingPolicy::Fixed;
    resign(&mut previous);
    let next = parent_successor(&previous, &[]);
    assert_link(&previous, &next, true);
    assert_eq!(next.operational_due_at(), previous.operational_due_at());
}

#[test]
fn each_simultaneously_advanced_follow_dependency_requires_its_own_review_reason() {
    let previous = base(false);
    let (profile, material) = both_heads(&previous);
    for only_reason in [
        (
            TrackingDependency::Profile,
            TrackingReviewReason::ProfileChanged,
        ),
        (
            TrackingDependency::Source,
            TrackingReviewReason::SourceChanged,
        ),
    ] {
        for cause_role in [ObservationRole::Profile, ObservationRole::Source] {
            let next = successor(
                &previous,
                &profile,
                &material,
                None,
                cause_role,
                &[only_reason],
            );
            assert_link(&previous, &next, false);
        }
    }
}

#[test]
fn simultaneous_follow_changes_keep_all_reasons_and_the_historical_calculation() {
    let previous = base(false);
    let (profile, material) = both_heads(&previous);
    for cause_role in [ObservationRole::Profile, ObservationRole::Source] {
        let next = successor(
            &previous,
            &profile,
            &material,
            None,
            cause_role,
            &[
                (
                    TrackingDependency::Profile,
                    TrackingReviewReason::ProfileChanged,
                ),
                (
                    TrackingDependency::Source,
                    TrackingReviewReason::SourceChanged,
                ),
            ],
        );
        assert_link(&previous, &next, true);
        assert!(next.operational_due_at().is_none());
        assert_eq!(next.calculation, previous.calculation);
    }
}

#[test]
fn a_fixed_profile_does_not_require_a_changed_reason_during_source_review() {
    let mut previous = base(false);
    previous.tracking.as_mut().unwrap().policies.profile = TrackingPolicy::Fixed;
    resign(&mut previous);
    let (profile, material) = both_heads(&previous);
    let next = successor(
        &previous,
        &profile,
        &material,
        None,
        ObservationRole::Source,
        &[(
            TrackingDependency::Source,
            TrackingReviewReason::SourceChanged,
        )],
    );
    assert_link(&previous, &next, true);
}

#[test]
fn a_retirement_reason_is_sufficient_for_the_same_advanced_follow_dependency() {
    let previous = base(false);
    let (mut profile, mut material) = both_heads(&previous);
    replace_profile(&mut profile, true);
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
        2,
        true,
        "2026-01-10",
    ))));
    let next = successor(
        &previous,
        &profile,
        &material,
        None,
        ObservationRole::Source,
        &[
            (
                TrackingDependency::Profile,
                TrackingReviewReason::DependencyRetired,
            ),
            (
                TrackingDependency::Source,
                TrackingReviewReason::DependencyRetired,
            ),
        ],
    );
    assert_link(&previous, &next, true);
    let previous = base(true);
    let parent = inputs::resolution(2, true, "2026-01-01");
    let next = successor(
        &previous,
        &previous.calculation.profile,
        &previous.calculation.material,
        Some(&parent),
        ObservationRole::NotificationParent,
        &[(
            TrackingDependency::Source,
            TrackingReviewReason::DependencyRetired,
        )],
    );
    assert_link(&previous, &next, true);
}
