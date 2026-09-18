#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_technical_support;

use application::{
    deadline_reevaluation::*, deadline_technical::*, deadline_tracking::*, deadlines::*,
};
use deadline_technical_support::*;

#[test]
fn legacy_bootstrap_preserves_history_and_requests_each_undeclared_policy() {
    let base = legacy();
    let before = deadline_capture_bytes(inputs::hasher().as_ref(), &base).unwrap();
    let next = revision(&base, bootstrap(), heads(&base));
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
    assert_eq!(next.operational_due_at(), None);
    assert_eq!(next.definition, base.definition);
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(next.responsible, base.responsible);
    assert_eq!(next.attention, base.attention);
    assert_eq!(
        next.tracking.as_ref().unwrap().policies,
        TrackingPolicies {
            profile: TrackingPolicy::Undetermined,
            source: TrackingPolicy::Undetermined,
            calendar: TrackingPolicy::Undetermined,
        }
    );
    for dependency in [TrackingDependency::Profile, TrackingDependency::Source] {
        assert!(has_reason(
            &next,
            dependency,
            TrackingReviewReason::PolicyUndetermined
        ));
    }
    assert_eq!(next.tracking.as_ref().unwrap().review.reasons().len(), 2);
    assert_eq!(
        deadline_capture_bytes(inputs::hasher().as_ref(), &base).unwrap(),
        before
    );
    assert!(matches!(
        next.recorded_by,
        DeadlineActorSnapshot::Technical {
            service: TechnicalService::DeadlineReevaluator,
            policy_version: 1,
        }
    ));
    let DeadlineReceiptVersion::Tracked(metadata) = &next.receipt.version else {
        panic!()
    };
    assert_eq!(
        metadata.predecessor,
        Some(PredecessorReceipt {
            submission_digest: base.receipt.submission_digest,
            capture_digest: base.receipt.capture_digest,
        })
    );
    assert_eq!(metadata.cause, Some(bootstrap().cause));
    assert!(matches!(
        prepare(&next, bootstrap(), heads(&next)).unwrap(),
        DeadlineReevaluationOutcome::NoChange(DeadlineReevaluationNoChange::AlreadyInitialized)
    ));
}

#[test]
fn bootstrap_does_not_redeclare_an_accepted_human_record() {
    let base = accepted(TrackingPolicy::Follow);
    assert!(matches!(
        prepare(&base, bootstrap(), heads(&base)).unwrap(),
        DeadlineReevaluationOutcome::NoChange(DeadlineReevaluationNoChange::AlreadyInitialized)
    ));
}

#[test]
fn an_advanced_followed_source_requires_review_without_requalifying_it() {
    let base = attention(&accepted(TrackingPolicy::Follow));
    let resolved = source_heads(&base, 2, false);
    let cause = event_command(source_event(&resolved, 2));
    let expected_cause = cause.cause;
    let next = revision(&base, cause, resolved);
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
    assert!(has_reason(
        &next,
        TrackingDependency::Source,
        TrackingReviewReason::SourceChanged
    ));
    assert_eq!(next.operational_due_at(), None);
    assert_eq!(next.definition, base.definition);
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(next.responsible, base.responsible);
    assert_eq!(next.attention, base.attention);
    assert_eq!(observation(&next, ObservationRole::Source).revision, 2);
    let DeadlineReceiptVersion::Tracked(metadata) = next.receipt.version else {
        panic!()
    };
    assert_eq!(metadata.cause, Some(expected_cause));
}

#[test]
fn an_explicitly_fixed_source_advances_observation_and_keeps_acceptance() {
    let base = accepted(TrackingPolicy::Fixed);
    let resolved = source_heads(&base, 2, false);
    let next = revision(&base, event_command(source_event(&resolved, 2)), resolved);
    assert_eq!(next.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(next.operational_due_at(), base.operational_due_at());
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(observation(&next, ObservationRole::Source).revision, 2);
}

#[test]
fn retirement_requires_review_even_when_the_source_is_fixed() {
    let base = accepted(TrackingPolicy::Fixed);
    let resolved = source_heads(&base, 2, true);
    let next = revision(&base, event_command(source_event(&resolved, 2)), resolved);
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
    assert!(has_reason(
        &next,
        TrackingDependency::Source,
        TrackingReviewReason::DependencyRetired
    ));
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(next.operational_due_at(), None);
}

#[test]
fn repeated_source_changes_preserve_every_existing_review_reason() {
    let initial = accepted(TrackingPolicy::Follow);
    let resolved = source_heads(&initial, 2, false);
    let base = revision(
        &initial,
        event_command(source_event(&resolved, 2)),
        resolved,
    );
    let resolved = source_heads(&base, 3, true);
    let next = revision(&base, event_command(source_event(&resolved, 3)), resolved);
    for reason in [
        TrackingReviewReason::SourceChanged,
        TrackingReviewReason::DependencyRetired,
    ] {
        assert!(has_reason(&next, TrackingDependency::Source, reason));
    }
    assert_eq!(next.calculation, initial.calculation);
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
}

#[test]
fn an_old_event_can_observe_a_later_head_without_rewriting_its_cause() {
    let base = accepted(TrackingPolicy::Follow);
    let event = fact_event(&inputs::resolution(2, false, "2026-01-10"), 2);
    let next = revision(&base, event_command(event), source_heads(&base, 3, false));
    assert_eq!(observation(&next, ObservationRole::Source).revision, 3);
    let DeadlineReceiptVersion::Tracked(metadata) = &next.receipt.version else {
        panic!()
    };
    assert_eq!(metadata.cause, Some(event_command(event).cause));
    for event in [
        event,
        fact_event(&inputs::resolution(3, false, "2026-01-10"), 3),
    ] {
        assert!(matches!(
            prepare(&next, event_command(event), source_heads(&next, 4, false)).unwrap(),
            DeadlineReevaluationOutcome::NoChange(DeadlineReevaluationNoChange::AlreadyObserved)
        ));
    }
}

#[test]
fn a_retired_deadline_has_no_technical_successor() {
    let base = retired(&accepted(TrackingPolicy::Follow));
    let resolved = source_heads(&base, 2, false);
    assert!(matches!(
        prepare(&base, event_command(source_event(&resolved, 2)), resolved).unwrap(),
        DeadlineReevaluationOutcome::NoChange(DeadlineReevaluationNoChange::Retired)
    ));
}

#[test]
fn an_unrelated_event_cannot_trigger_other_new_heads() {
    let base = accepted(TrackingPolicy::Follow);
    let resolved = source_heads(&base, 2, false);
    let mut event = source_event(&resolved, 2);
    event.source_id = uuid::Uuid::from_u128(999);
    assert!(matches!(
        prepare(&base, event_command(event), resolved).unwrap(),
        DeadlineReevaluationOutcome::NoChange(DeadlineReevaluationNoChange::DependencyNotSelected)
    ));
}

#[test]
fn profile_follow_requests_qualification_and_keeps_the_selected_profile() {
    let base = accepted(TrackingPolicy::Follow);
    let mut resolved = heads(&base);
    deadline_observation_support::replace_profile(&mut resolved.profile_head, false);
    let next = revision(&base, event_command(profile_event(&resolved, 2)), resolved);
    assert!(has_reason(
        &next,
        TrackingDependency::Profile,
        TrackingReviewReason::ProfileChanged
    ));
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(next.definition.profile, base.definition.profile);
    assert_eq!(observation(&next, ObservationRole::Profile).revision, 2);
}

#[test]
fn a_notification_parent_change_uses_source_policy_without_rechaining_the_parent() {
    let base = notification_base(TrackingPolicy::Follow);
    let mut resolved = heads(&base);
    let parent = inputs::resolution(2, false, "2026-01-01");
    let event = fact_event(&parent, 2);
    resolved.notification_parent_head = Some(parent);
    let next = revision(&base, event_command(event), resolved);
    assert!(has_reason(
        &next,
        TrackingDependency::Source,
        TrackingReviewReason::SourceChanged
    ));
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(
        next.definition.input.selection,
        base.definition.input.selection
    );
    assert_eq!(
        observation(&next, ObservationRole::Source)
            .parent_resolution
            .unwrap()
            .revision,
        1
    );
    assert_eq!(
        observation(&next, ObservationRole::NotificationParent).revision,
        2
    );
}

#[test]
fn a_closed_administration_does_not_stop_technical_source_review() {
    let base = accepted(TrackingPolicy::Follow);
    let mut resolved = source_heads(&base, 2, false);
    resolved.material.administration =
        deadline_observation_support::administration(base.case_id, true);
    let expected = resolved.material.administration.clone();
    let next = revision(&base, event_command(source_event(&resolved, 2)), resolved);
    assert_eq!(next.tracking.as_ref().unwrap().administration, expected);
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(next.operational_due_at(), None);
}
