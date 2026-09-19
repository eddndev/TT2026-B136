#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_technical_support;

use application::{
    deadline_evaluations::deadline_evaluation_record_bytes, deadline_reevaluation::*,
    deadline_tracking::*,
};
use deadline_technical_support::*;

#[test]
fn calendar_follow_recalculates_and_keeps_the_exact_historical_source_and_attention() {
    let base = attention(&calendar_base(
        TrackingPolicy::Follow,
        TrackingPolicy::Follow,
    ));
    let resolved = calendar_heads(&base, 2, false);
    let calendar = resolved.material.calendar_head.clone().unwrap();
    let next = revision(&base, event_command(calendar_event(&calendar, 2)), resolved);
    assert_eq!(next.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(next.definition.input.calendar.unwrap().revision.get(), 2);
    assert_eq!(next.calculation.material.calendar.as_ref(), Some(&calendar));
    assert_eq!(
        next.calculation.material.calendar_head.as_ref(),
        Some(&calendar)
    );
    assert!(base.calculation.result.due_at().is_some());
    assert_eq!(next.calculation.result.due_at(), None);
    assert_eq!(next.operational_due_at(), None);
    let expected = application::deadline_evaluations::evaluate_profiled_deadline(
        inputs::hasher().as_ref(),
        &base.calculation.profile.definition,
        &next.definition.input,
        &next.calculation.material,
    )
    .unwrap();
    assert_eq!(
        next.calculation.result,
        application::deadline_evaluations::DeadlineEvaluationRecord::capture(&expected)
    );
    assert_ne!(
        deadline_evaluation_record_bytes(&next.calculation.result),
        deadline_evaluation_record_bytes(&base.calculation.result)
    );
    assert_eq!(next.calculation.profile, base.calculation.profile);
    assert_eq!(
        next.calculation.material.source,
        base.calculation.material.source
    );
    assert_eq!(
        next.calculation.material.source_head,
        base.calculation.material.source_head
    );
    assert_eq!(
        next.calculation.material.administration,
        base.calculation.material.administration
    );
    assert_eq!(
        next.definition.input.qualification,
        base.definition.input.qualification
    );
    assert_eq!(next.responsible, base.responsible);
    assert_eq!(next.attention, base.attention);
}

#[test]
fn calendar_fixed_keeps_the_selected_revision_and_captured_arithmetic() {
    let base = calendar_base(TrackingPolicy::Follow, TrackingPolicy::Fixed);
    let resolved = calendar_heads(&base, 2, false);
    let event = calendar_event(resolved.material.calendar_head.as_ref().unwrap(), 2);
    let next = revision(&base, event_command(event), resolved);
    assert_eq!(next.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(next.definition, base.definition);
    assert_eq!(next.operational_due_at(), base.operational_due_at());
    assert_eq!(observation(&next, ObservationRole::Calendar).revision, 2);
}

#[test]
fn a_retired_followed_calendar_preserves_arithmetic_and_requires_review() {
    let base = calendar_base(TrackingPolicy::Follow, TrackingPolicy::Follow);
    let resolved = calendar_heads(&base, 2, true);
    let event = calendar_event(resolved.material.calendar_head.as_ref().unwrap(), 2);
    let next = revision(&base, event_command(event), resolved);
    assert!(has_reason(
        &next,
        TrackingDependency::Calendar,
        TrackingReviewReason::DependencyRetired
    ));
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(next.definition, base.definition);
    assert_eq!(next.operational_due_at(), None);
}

#[test]
fn calendar_follow_cannot_clear_pending_source_review_or_replace_its_calculation() {
    let initial = calendar_base(TrackingPolicy::Follow, TrackingPolicy::Follow);
    let resolved = source_heads(&initial, 2, false);
    let base = revision(
        &initial,
        event_command(source_event(&resolved, 2)),
        resolved,
    );
    let mut resolved = calendar_heads(&base, 2, false);
    resolved.material.source_head = source_heads(&base, 2, false).material.source_head;
    let event = calendar_event(resolved.material.calendar_head.as_ref().unwrap(), 3);
    let next = revision(&base, event_command(event), resolved);
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
    assert!(has_reason(
        &next,
        TrackingDependency::Source,
        TrackingReviewReason::SourceChanged
    ));
    assert_eq!(next.calculation, initial.calculation);
    assert_eq!(next.definition, initial.definition);
    assert_eq!(observation(&next, ObservationRole::Calendar).revision, 2);
}

#[test]
fn a_calendar_event_can_coalesce_to_a_later_exact_calendar_head() {
    let base = calendar_base(TrackingPolicy::Follow, TrackingPolicy::Follow);
    let event = calendar_event(
        calendar_heads(&base, 2, false)
            .material
            .calendar_head
            .as_ref()
            .unwrap(),
        2,
    );
    let next = revision(&base, event_command(event), calendar_heads(&base, 3, false));
    assert_eq!(next.definition.input.calendar.unwrap().revision.get(), 3);
    assert_eq!(observation(&next, ObservationRole::Calendar).revision, 3);
    let application::deadlines::DeadlineReceiptVersion::Tracked(metadata) = next.receipt.version
    else {
        panic!()
    };
    assert_eq!(metadata.cause, Some(event_command(event).cause));
}

#[test]
fn an_advanced_fixed_source_can_trigger_a_simultaneously_changed_followed_calendar() {
    let base = calendar_base(TrackingPolicy::Fixed, TrackingPolicy::Follow);
    let mut resolved = source_heads(&base, 2, false);
    resolved.material.calendar_head = calendar_heads(&base, 2, false).material.calendar_head;
    let next = revision(&base, event_command(source_event(&resolved, 2)), resolved);
    assert_eq!(next.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(next.definition.input.calendar.unwrap().revision.get(), 2);
    assert_eq!(observation(&next, ObservationRole::Source).revision, 2);
    assert_eq!(
        next.calculation.material.source,
        base.calculation.material.source
    );
    assert_eq!(
        next.calculation.material.source_head,
        base.calculation.material.source_head
    );
}

#[test]
fn simultaneous_source_follow_and_calendar_follow_preserve_the_old_calculation() {
    let base = calendar_base(TrackingPolicy::Follow, TrackingPolicy::Follow);
    let mut resolved = source_heads(&base, 2, false);
    resolved.material.calendar_head = calendar_heads(&base, 2, false).material.calendar_head;
    let event = calendar_event(resolved.material.calendar_head.as_ref().unwrap(), 3);
    let next = revision(&base, event_command(event), resolved);
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
    assert!(has_reason(
        &next,
        TrackingDependency::Source,
        TrackingReviewReason::SourceChanged
    ));
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(next.definition, base.definition);
    assert_eq!(observation(&next, ObservationRole::Source).revision, 2);
    assert_eq!(observation(&next, ObservationRole::Calendar).revision, 2);
    assert_eq!(next.operational_due_at(), None);
}
