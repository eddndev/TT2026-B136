#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;
mod deadline_tracked_support;

use application::{
    deadline_evaluations::*, deadline_inputs::*, deadline_profiles::*, deadline_reevaluation::*,
    deadline_tracking::*, deadlines::*,
};
use deadline_support::evaluation::{self, inputs, text};
use deadline_tracked_support::{accepted, capture, resign};
use domain::{
    deadline_arithmetic::*, deadline_profiles::DeadlineRuleTemplate,
    judicial_calendars::JudicialCalendarClassification,
};

fn calendar_base() -> DeadlineDetail {
    let mut value = accepted();
    let calendar = inputs::calendar(1, false, JudicialCalendarClassification::Countable);
    let mut definition = evaluation::definition();
    definition.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::Days {
        quantity: evaluation::n(2),
        inclusion: DayInclusion::OnAnchor,
        basis: DayBasis::CalendarCountable,
        final_day: FinalDayPolicy::Preserve,
    });
    definition.examples[0].calendar = Some(calendar.values.clone());
    let profile = &mut value.calculation.profile;
    profile.definition = DeadlineProfileDefinition::new(definition).unwrap();
    profile.definition_digest =
        deadline_profile_definition_digest(inputs::hasher().as_ref(), &profile.definition);
    profile.receipt.submission_digest = deadline_profile_submission_digest(
        inputs::hasher().as_ref(),
        profile.recorded_by.id,
        &DeadlineProfileCommand {
            operation_id: profile.receipt.operation_id,
            profile_id: profile.id,
            change: DeadlineProfileChange::Publish {
                definition: profile.definition.clone(),
            },
        },
        profile.algorithm,
        profile.definition_digest,
    );
    value.definition.input.calendar = Some(DeadlineCalendarRef {
        id: calendar.id,
        revision: calendar.revision,
    });
    value.calculation.material.calendar = Some(calendar.clone());
    value.calculation.material.calendar_head = Some(calendar.clone());
    calculate(&mut value);
    let mut tracking = capture(&value);
    tracking.policies.calendar = TrackingPolicy::Follow;
    tracking
        .observations
        .entries
        .push(calendar_observation(&calendar));
    value.tracking = Some(tracking);
    resign(&mut value);
    value
}
fn calendar_observation(
    value: &application::judicial_calendars::JudicialCalendarDetail,
) -> ObservationEntry {
    ObservationEntry {
        role: ObservationRole::Calendar,
        family: DependencyFamily::Calendar,
        id: value.id.as_uuid(),
        revision: value.revision.get(),
        case_id: None,
        hearing_id: None,
        parent_resolution: None,
        submission_digest: value.receipt.submission_digest,
        evidence_digest: domain::crypto::Sha256Digest::from_array([8; 32]),
    }
}
fn calculate(value: &mut DeadlineDetail) {
    let result = evaluate_profiled_deadline(
        inputs::hasher().as_ref(),
        &value.calculation.profile.definition,
        &value.definition.input,
        &value.calculation.material,
    )
    .unwrap();
    value.calculation.result = DeadlineEvaluationRecord::capture(&result);
}
fn recalculated(base: &DeadlineDetail) -> DeadlineDetail {
    let mut value = base.clone();
    value.revision = base.revision.next().unwrap();
    value.reason = Some(text("Follow the newly observed calendar revision"));
    value.receipt.action = DeadlineAction::Reevaluate;
    value.receipt.operation_id = DeadlineOperationId::from_uuid(uuid::Uuid::from_u128(102));
    value.receipt.expected_revision = base.revision.get();
    value.recorded_by = DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    };
    let calendar = inputs::calendar(2, false, JudicialCalendarClassification::Unresolved);
    value.definition.input.calendar = Some(DeadlineCalendarRef {
        id: calendar.id,
        revision: calendar.revision,
    });
    value.calculation.material.calendar = Some(calendar.clone());
    value.calculation.material.calendar_head = Some(calendar.clone());
    calculate(&mut value);
    let mut tracking = base.tracking.clone().unwrap();
    tracking.observations.entries[2] = calendar_observation(&calendar);
    value.tracking = Some(tracking);
    let DeadlineReceiptVersion::Tracked(metadata) = &mut value.receipt.version else {
        unreachable!();
    };
    metadata.predecessor = Some(PredecessorReceipt {
        submission_digest: base.receipt.submission_digest,
        capture_digest: base.receipt.capture_digest,
    });
    metadata.cause = Some(TechnicalCause::SourceEvent {
        job_id: uuid::Uuid::from_u128(501),
        event: SourceEventReference {
            sequence: 2,
            family: DependencyFamily::Calendar,
            source_id: calendar.id.as_uuid(),
            revision: calendar.revision.get(),
            case_id: None,
            hearing_id: None,
            operation_id: calendar.receipt.operation_id.as_uuid(),
        },
    });
    resign(&mut value);
    value
}
fn valid_record(value: &DeadlineDetail) {
    deadline_receipt_matches(inputs::hasher().as_ref(), value).unwrap();
}
fn reject(previous: &DeadlineDetail, next: &DeadlineDetail) {
    valid_record(previous);
    valid_record(next);
    assert!(deadline_successor_matches(inputs::hasher().as_ref(), previous, next).is_err());
}

#[test]
fn a_followed_calendar_can_produce_a_new_calculation_without_human_requalification() {
    let previous = calendar_base();
    let next = recalculated(&previous);
    valid_record(&previous);
    valid_record(&next);
    deadline_successor_matches(inputs::hasher().as_ref(), &previous, &next).unwrap();
    assert_eq!(next.review_state(), DeadlineReviewState::Accepted);
    assert!(previous.calculation.result.due_at().is_some());
    assert!(next.calculation.result.due_at().is_none());
    assert_ne!(
        deadline_evaluation_record_bytes(&previous.calculation.result),
        deadline_evaluation_record_bytes(&next.calculation.result)
    );
}

#[test]
fn calendar_recalculation_cannot_change_legal_qualification_or_source_evidence() {
    let previous = calendar_base();
    for mutation in 0..3 {
        let mut next = recalculated(&previous);
        match mutation {
            0 => next.definition.input.qualification.statement = text("Changed qualification"),
            1 => {
                next.calculation.profile.recorded_at = next
                    .calculation
                    .profile
                    .recorded_at
                    .to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap())
            }
            _ => {
                next.calculation.material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(
                    inputs::resolution(2, false, "2026-01-09"),
                )))
            }
        }
        resign(&mut next);
        reject(&previous, &next);
    }
}

#[test]
fn calendar_recalculation_requires_an_exact_new_calendar_event_and_follow_policy() {
    let previous = calendar_base();
    for mutation in 0..5 {
        let mut next = recalculated(&previous);
        match mutation {
            0 => next.tracking.as_mut().unwrap().policies.calendar = TrackingPolicy::Fixed,
            _ => {
                let DeadlineReceiptVersion::Tracked(metadata) = &mut next.receipt.version else {
                    unreachable!();
                };
                let Some(TechnicalCause::SourceEvent { event, .. }) = &mut metadata.cause else {
                    unreachable!();
                };
                match mutation {
                    1 => event.revision = 1,
                    2 => event.source_id = uuid::Uuid::from_u128(999),
                    3 => event.operation_id = uuid::Uuid::from_u128(998),
                    _ => event.family = DependencyFamily::Profile,
                }
            }
        }
        resign(&mut next);
        reject(&previous, &next);
    }
}

#[test]
fn a_fixed_calendar_cannot_be_silently_reselected_even_with_a_valid_result() {
    let mut previous = calendar_base();
    previous.tracking.as_mut().unwrap().policies.calendar = TrackingPolicy::Fixed;
    resign(&mut previous);
    let next = recalculated(&previous);
    reject(&previous, &next);
}

#[test]
fn calendar_follow_does_not_clear_existing_pending_human_review() {
    let mut previous = calendar_base();
    previous.receipt.action = DeadlineAction::SetAttention;
    previous.receipt.expected_revision = 1;
    previous.revision = DeadlineRevision::new(2).unwrap();
    previous.reason = Some(text("Previously pending review"));
    let DeadlineReceiptVersion::Tracked(metadata) = &mut previous.receipt.version else {
        unreachable!();
    };
    metadata.predecessor = Some(PredecessorReceipt {
        submission_digest: previous.receipt.submission_digest,
        capture_digest: previous.receipt.capture_digest,
    });
    let tracking = previous.tracking.as_mut().unwrap();
    tracking.review = TrackingReview::new(
        DeadlineReviewState::Pending,
        vec![TrackingReviewRequirement {
            dependency: TrackingDependency::Source,
            reason: TrackingReviewReason::SourceChanged,
        }],
    )
    .unwrap();
    tracking.observations.entries[1].revision = 2;
    resign(&mut previous);
    let mut next = recalculated(&previous);
    next.tracking.as_mut().unwrap().observations.entries[1] =
        capture(&previous).observations.entries[1].clone();
    next.tracking.as_mut().unwrap().review =
        TrackingReview::new(DeadlineReviewState::Accepted, vec![]).unwrap();
    resign(&mut next);
    reject(&previous, &next);
}
