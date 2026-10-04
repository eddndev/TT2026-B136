use crate::deadline_support::evaluation::{self, inputs};
use crate::hearing_derived_deadline_support::*;
use application::{deadline_evaluations::DeadlineEvaluationBlock, deadline_profiles::*};
use domain::{
    deadline_profiles::DeadlineRuleBlock,
    deadline_triggers::{QualifiedTriggerPurpose, TriggerBlock, TriggerFamily, TriggerRequirement},
    hearing_results::{HearingResultExtent, HearingResultRevision},
    procedural_facts::FactDeclaration,
};
use time::{Duration, Time, UtcOffset};

#[test]
fn prospective_result_calculates_without_a_stored_result_or_capture_timestamp() {
    let fixture = fixture();
    assert!(fixture.result_preparation.base.is_none());
    let draft = fixture.prepare().unwrap();
    assert_eq!(draft.command(), &fixture.command);
    assert_eq!(draft.result(), &fixture.material.result);
    assert_eq!(
        draft.result().result_revision,
        HearingResultRevision::initial()
    );
    assert_eq!(
        draft.evaluation().due_at(),
        Some(
            evaluation::date("2026-01-07")
                .date()
                .with_time(Time::from_hms(23, 30, 0).unwrap())
                .assume_utc()
        )
    );
    assert!(draft.evaluation().blocks().is_empty());
    let selected = draft.evaluation().trigger().source().unwrap();
    assert_eq!(selected.case_id, case_id());
    assert_eq!(selected.agreement.as_ref().unwrap().id(), agreement(501));
    assert!(draft.require_review(draft.review_digest()).is_ok());
}

#[test]
fn reviewed_instruction_survives_a_later_capture_clock() {
    let fixture = fixture();
    let first = fixture.prepare().unwrap();
    let later = fixture
        .prepare_at(crate::case_support::instant() + Duration::days(2))
        .unwrap();
    assert_eq!(first.result(), later.result());
    assert_eq!(first.evaluation(), later.evaluation());
    assert_eq!(first.review_digest(), later.review_digest());
    assert!(later.require_review(first.review_digest()).is_ok());
}

#[test]
fn observation_clock_rejects_an_event_that_has_not_happened_yet() {
    let fixture = fixture();
    let before = evaluation::date("2026-01-05")
        .date()
        .with_time(Time::MIDNIGHT)
        .assume_offset(UtcOffset::UTC);
    assert!(fixture.prepare_at(before).is_err());
}

#[derive(Debug, Clone, Copy)]
enum MissingDecision {
    OrderedQuantity,
    ScopeApplicability,
    IncidentStatus,
    Condition,
    CivilCutoff,
}

#[test]
fn incomplete_explicit_decisions_block_without_guessing_a_due_instant() {
    for missing in [
        MissingDecision::OrderedQuantity,
        MissingDecision::ScopeApplicability,
        MissingDecision::IncidentStatus,
        MissingDecision::Condition,
        MissingDecision::CivilCutoff,
    ] {
        let mut fixture = fixture();
        let expected = match missing {
            MissingDecision::OrderedQuantity => {
                fixture.edit_definition(|value| value.input.ordered_quantity = None);
                DeadlineEvaluationBlock::Rule(DeadlineRuleBlock::MissingOrderedQuantity)
            }
            MissingDecision::ScopeApplicability => {
                fixture.edit_definition(|value| {
                    value.input.qualification.scope_applies =
                        FactDeclaration::Unknown(evaluation::text("Applicability not reviewed"));
                });
                DeadlineEvaluationBlock::ScopeUnknown
            }
            MissingDecision::IncidentStatus => {
                fixture.edit_definition(|value| {
                    value.input.qualification.unresolved_incident =
                        FactDeclaration::Unknown(evaluation::text("Incident status not reviewed"));
                });
                DeadlineEvaluationBlock::IncidentUnknown
            }
            MissingDecision::Condition => {
                fixture.edit_definition(|value| value.input.qualification.conditions.clear());
                DeadlineEvaluationBlock::ConditionMissing(evaluation::id(1))
            }
            MissingDecision::CivilCutoff => {
                fixture.edit_profile(|value| {
                    value.completion = DeadlineCompletionPolicy::CivilCandidateOnly;
                });
                DeadlineEvaluationBlock::CivilCutoffMissing
            }
        };
        let draft = fixture.prepare().unwrap();
        assert_eq!(draft.evaluation().due_at(), None, "{missing:?}");
        assert_eq!(draft.evaluation().blocks(), &[expected], "{missing:?}");
    }
}

#[test]
fn concluded_session_does_not_invent_a_qualified_hearing_end() {
    let mut fixture = fixture();
    fixture.edit_values(|value| value.extent = HearingResultExtent::Concluded);
    fixture.edit_profile(|value| {
        value.trigger = TriggerRequirement::Qualified {
            purpose: QualifiedTriggerPurpose::HearingEnd,
            family: TriggerFamily::HearingResult,
        };
    });
    let draft = fixture.prepare().unwrap();
    assert_eq!(draft.evaluation().due_at(), None);
    assert_eq!(
        draft.evaluation().blocks(),
        &[DeadlineEvaluationBlock::Trigger(
            TriggerBlock::MissingQualification {
                purpose: QualifiedTriggerPurpose::HearingEnd,
            }
        )]
    );
    fixture.edit_definition(|value| {
        value.input.selection.qualification =
            Some(domain::deadline_triggers::QualifiedTriggerTime {
                purpose: QualifiedTriggerPurpose::HearingEnd,
                at: inputs::date("2026-01-06"),
                statement: evaluation::text("Operator explicitly declares hearing end"),
                locator: evaluation::label("Declared agreement"),
            });
    });
    let qualified = fixture.prepare().unwrap();
    assert!(qualified.evaluation().due_at().is_some());
    assert!(qualified.require_review(draft.review_digest()).is_err());
}

#[test]
fn prospective_calculation_matches_the_strict_tracked_path_with_actual_capture_metadata() {
    use application::{deadline_inputs::*, deadlines::*};
    let fixture = fixture();
    let reviewed = fixture.prepare().unwrap();
    let (command, policies) = fixture.command.deadline.clone().into_parts();
    let mut captures = Vec::new();
    for delay in [1, 2] {
        // A separate recorded-source fixture enters the strict existing path only
        // after the prospective review. This is not a persistence acceptance test.
        let mut source = crate::hearing_result_support::detail(
            fixture.actor.id,
            &fixture.command.result,
            &fixture.result_preparation,
        );
        source.snapshot.recorded_at = crate::case_support::instant() + Duration::hours(delay);
        source.snapshot.recorded_by.email = fixture.actor.email.clone();
        let source = Some(DeadlineSourceDetail::HearingResult(Box::new(source)));
        let preparation = DeadlinePreparation {
            case_id: case_id(),
            deadline_id: command.deadline_id,
            administration: fixture.material.result.observed_administration.clone(),
            base: None,
            resolved: Some(DeadlineResolvedInputs {
                profile: fixture.material.profile.clone(),
                profile_head: fixture.material.profile_head.clone(),
                material: DeadlineInputMaterial {
                    case_id: case_id(),
                    administration: fixture.material.result.observed_administration.clone(),
                    source: source.clone(),
                    source_head: source,
                    calendar: fixture.material.calendar.clone(),
                    calendar_head: fixture.material.calendar_head.clone(),
                },
                notification_parent_head: None,
            }),
            responsible: Some(fixture.material.responsible.clone()),
        };
        let stored = prepare_tracked_deadline_change(
            crate::hearing_result_support::hasher().as_ref(),
            DeadlineActorSnapshot::User {
                id: fixture.actor.id,
                email: fixture.actor.email.clone(),
            },
            case_id(),
            command.clone(),
            preparation,
            policies,
            None,
        )
        .unwrap();
        assert_eq!(
            stored.calculation().result,
            application::deadline_evaluations::DeadlineEvaluationRecord::capture(
                reviewed.evaluation()
            ),
        );
        captures.push(stored.capture_digest());
        let later = fixture
            .prepare_at(crate::case_support::instant() + Duration::hours(delay))
            .unwrap();
        assert!(later.require_review(reviewed.review_digest()).is_ok());
    }
    assert_ne!(captures[0], captures[1]);
}
