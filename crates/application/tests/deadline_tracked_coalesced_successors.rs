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
    deadline_inputs::{DeadlineCalendarRef, DeadlineInputMaterial, DeadlineSourceDetail},
    deadline_profiles::{DeadlineProfileDefinition, DeadlineProfileDetail},
    deadline_reevaluation::*,
    deadline_tracking::*,
    deadlines::*,
};
use deadline_observation_support::{build, replace_profile, resign_profile};
use deadline_support::evaluation::{self, inputs, text};
use deadline_tracked_support::{accepted, resign};
use domain::{
    deadline_arithmetic::*, deadline_profiles::DeadlineRuleTemplate,
    judicial_calendars::JudicialCalendarClassification as Classification,
};
use uuid::Uuid;

fn base(calendar: bool, source: TrackingPolicy, profile: TrackingPolicy) -> DeadlineDetail {
    let mut value = accepted();
    if calendar {
        let calendar = inputs::calendar(1, false, Classification::Countable);
        let mut definition = evaluation::definition();
        definition.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::Days {
            quantity: evaluation::n(2),
            inclusion: DayInclusion::OnAnchor,
            basis: DayBasis::CalendarCountable,
            final_day: FinalDayPolicy::Preserve,
        });
        definition.examples[0].calendar = Some(calendar.values.clone());
        value.calculation.profile.definition = DeadlineProfileDefinition::new(definition).unwrap();
        resign_profile(&mut value.calculation.profile);
        select_calendar(&mut value, calendar);
    }
    let tracking = value.tracking.as_mut().unwrap();
    tracking.policies.source = source;
    tracking.policies.profile = profile;
    if calendar {
        tracking.policies.calendar = TrackingPolicy::Follow;
    }
    tracking.observations = build(
        &value.calculation.profile,
        &value.calculation.material,
        None,
    )
    .unwrap();
    resign(&mut value);
    value
}
fn select_calendar(
    value: &mut DeadlineDetail,
    calendar: application::judicial_calendars::JudicialCalendarDetail,
) {
    value.definition.input.calendar = Some(DeadlineCalendarRef {
        id: calendar.id,
        revision: calendar.revision,
    });
    value.calculation.material.calendar = Some(calendar.clone());
    value.calculation.material.calendar_head = Some(calendar);
    let result = evaluate_profiled_deadline(
        inputs::hasher().as_ref(),
        &value.calculation.profile.definition,
        &value.definition.input,
        &value.calculation.material,
    )
    .unwrap();
    value.calculation.result = DeadlineEvaluationRecord::capture(&result);
}
fn event(revision: u32, calendar: bool) -> SourceEventReference {
    let (family, source_id, operation_id, case_id) = if calendar {
        let value = inputs::calendar(revision, false, Classification::Unresolved);
        (
            DependencyFamily::Calendar,
            value.id.as_uuid(),
            value.receipt.operation_id.as_uuid(),
            None,
        )
    } else {
        let value = inputs::resolution(revision, false, "2026-01-10");
        (
            DependencyFamily::Resolution,
            Uuid::from_u128(10),
            value.snapshot.metadata().receipt.operation_id.as_uuid(),
            Some(value.snapshot.case_id()),
        )
    };
    SourceEventReference {
        sequence: u64::from(revision),
        family,
        source_id,
        revision,
        case_id,
        hearing_id: None,
        operation_id,
    }
}
fn review(source_changed: bool) -> TrackingReview {
    TrackingReview::new(
        if source_changed {
            DeadlineReviewState::Pending
        } else {
            DeadlineReviewState::Accepted
        },
        if source_changed {
            vec![TrackingReviewRequirement {
                dependency: TrackingDependency::Source,
                reason: TrackingReviewReason::SourceChanged,
            }]
        } else {
            vec![]
        },
    )
    .unwrap()
}
fn successor(
    previous: &DeadlineDetail,
    profile: &DeadlineProfileDetail,
    material: &DeadlineInputMaterial,
    event: SourceEventReference,
    pending: bool,
    recalculate: bool,
) -> DeadlineDetail {
    let mut value = previous.clone();
    value.revision = previous.revision.next().unwrap();
    value.reason = Some(text("Observe all dependency heads for a durable event"));
    value.recorded_by = DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    };
    value.receipt.action = DeadlineAction::Reevaluate;
    value.receipt.operation_id = DeadlineOperationId::from_uuid(Uuid::from_u128(902));
    value.receipt.expected_revision = previous.revision.get();
    let DeadlineReceiptVersion::Tracked(metadata) = &mut value.receipt.version else {
        unreachable!();
    };
    metadata.predecessor = Some(PredecessorReceipt {
        submission_digest: previous.receipt.submission_digest,
        capture_digest: previous.receipt.capture_digest,
    });
    metadata.cause = Some(TechnicalCause::SourceEvent {
        job_id: Uuid::from_u128(901),
        event,
    });
    let tracking = value.tracking.as_mut().unwrap();
    tracking.review = review(pending);
    tracking.observations = build(profile, material, None).unwrap();
    if recalculate {
        select_calendar(&mut value, material.calendar_head.clone().unwrap());
    }
    resign(&mut value);
    deadline_receipt_matches(inputs::hasher().as_ref(), &value).unwrap();
    value
}
fn observed(previous: &DeadlineDetail, source: bool, calendar: bool) -> DeadlineInputMaterial {
    let mut value = previous.calculation.material.clone();
    if source {
        value.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
            4,
            false,
            "2026-01-10",
        ))));
    }
    if calendar {
        value.calendar_head = Some(inputs::calendar(4, false, Classification::Unresolved));
    }
    value
}
fn assert_link(previous: &DeadlineDetail, next: &DeadlineDetail, allowed: bool) {
    deadline_receipt_matches(inputs::hasher().as_ref(), previous).unwrap();
    deadline_receipt_matches(inputs::hasher().as_ref(), next).unwrap();
    let result = deadline_successor_matches(inputs::hasher().as_ref(), previous, next);
    assert_eq!(
        result.is_ok(),
        allowed,
        "unexpected successor decision: {result:?}"
    );
}
fn captured_event(value: &DeadlineDetail) -> SourceEventReference {
    let DeadlineReceiptVersion::Tracked(metadata) = &value.receipt.version else {
        unreachable!();
    };
    let Some(TechnicalCause::SourceEvent { event, .. }) = metadata.cause else {
        unreachable!();
    };
    event
}

#[test]
fn an_unobserved_source_event_can_capture_a_newer_head_without_rewriting_its_cause() {
    let previous = base(false, TrackingPolicy::Follow, TrackingPolicy::Follow);
    let cause = event(2, false);
    let next = successor(
        &previous,
        &previous.calculation.profile,
        &observed(&previous, true, false),
        cause,
        true,
        false,
    );
    assert_link(&previous, &next, true);
    assert_eq!(captured_event(&next), cause);
    assert_eq!(
        next.tracking.as_ref().unwrap().observations.entries[1].revision,
        4
    );
    assert_eq!(next.calculation, previous.calculation);
    assert!(next.operational_due_at().is_none());
}

#[test]
fn an_event_at_or_before_the_previously_observed_revision_is_rejected() {
    let mut previous = base(false, TrackingPolicy::Fixed, TrackingPolicy::Follow);
    let mut already_observed = previous.calculation.material.clone();
    already_observed.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
        2,
        false,
        "2026-01-10",
    ))));
    previous.tracking.as_mut().unwrap().observations =
        build(&previous.calculation.profile, &already_observed, None).unwrap();
    resign(&mut previous);
    for revision in [1, 2] {
        let next = successor(
            &previous,
            &previous.calculation.profile,
            &observed(&previous, true, false),
            event(revision, false),
            false,
            false,
        );
        assert_link(&previous, &next, false);
    }
}

#[test]
fn an_event_newer_than_its_observed_head_is_rejected() {
    let previous = base(false, TrackingPolicy::Follow, TrackingPolicy::Follow);
    let next = successor(
        &previous,
        &previous.calculation.profile,
        &observed(&previous, true, false),
        event(5, false),
        true,
        false,
    );
    assert_link(&previous, &next, false);
}

#[test]
fn a_calendar_event_can_recalculate_to_a_newer_head_with_a_different_operation() {
    let previous = base(true, TrackingPolicy::Follow, TrackingPolicy::Follow);
    let cause = event(2, true);
    let next = successor(
        &previous,
        &previous.calculation.profile,
        &observed(&previous, false, true),
        cause,
        false,
        true,
    );
    assert_link(&previous, &next, true);
    assert_eq!(captured_event(&next), cause);
    let calendar = next.calculation.material.calendar.as_ref().unwrap();
    assert_eq!(calendar.revision.get(), 4);
    assert_ne!(cause.operation_id, calendar.receipt.operation_id.as_uuid());
    assert!(previous.calculation.result.due_at().is_some());
    assert!(next.calculation.result.due_at().is_none());
}

#[test]
fn a_fixed_source_event_can_coalesce_fixed_observations_and_followed_calendar_recalculation() {
    let previous = base(true, TrackingPolicy::Fixed, TrackingPolicy::Fixed);
    let mut profile = previous.calculation.profile.clone();
    replace_profile(&mut profile, false);
    let cause = event(2, false);
    let next = successor(
        &previous,
        &profile,
        &observed(&previous, true, true),
        cause,
        false,
        true,
    );
    assert_link(&previous, &next, true);
    assert_eq!(captured_event(&next), cause);
    assert_eq!(next.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(next.calculation.profile, previous.calculation.profile);
    assert_eq!(
        next.calculation.material.source,
        previous.calculation.material.source
    );
    assert_eq!(
        next.calculation.material.source_head,
        previous.calculation.material.source_head
    );
    let revisions: Vec<_> = next
        .tracking
        .as_ref()
        .unwrap()
        .observations
        .entries
        .iter()
        .map(|entry| entry.revision)
        .collect();
    assert_eq!(revisions, [2, 4, 4]);
}

#[test]
fn a_simultaneously_changed_followed_source_requires_pending_and_preserved_calculation() {
    let previous = base(true, TrackingPolicy::Follow, TrackingPolicy::Fixed);
    let material = observed(&previous, true, true);
    let next = successor(
        &previous,
        &previous.calculation.profile,
        &material,
        event(2, false),
        true,
        false,
    );
    assert_link(&previous, &next, true);
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
    assert_eq!(next.calculation, previous.calculation);
    assert!(next.operational_due_at().is_none());
    let invalid = successor(
        &previous,
        &previous.calculation.profile,
        &material,
        event(2, false),
        true,
        true,
    );
    assert_link(&previous, &invalid, false);
}
