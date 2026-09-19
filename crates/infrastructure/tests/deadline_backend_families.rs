mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_input_history_support;
mod deadline_input_support;
mod deadline_profile_database_support;
mod hearing_database_support;
mod hearing_result_database_support;
mod judicial_calendar_database_support;
mod procedural_fact_backend_support;

use application::{
    deadline_evaluations::*, deadline_inputs::*, deadline_profiles::*, deadlines::*,
};
use deadline_backend_support as dl;
use deadline_input_support as inputs;
use deadline_profile_database_support as profiles;
use domain::{
    deadline_arithmetic::*, deadline_profiles::*, deadline_triggers::*,
    hearing_results::HearingResultAgreementId, identity::Role, procedural_facts::*,
    procedural_time::DeclaredProceduralTime,
};
use procedural_fact_backend_support as facts;
use std::num::NonZeroU32;
use time::{Duration, UtcOffset};

fn publish(db: &dl::Fixture, definition: DeadlineProfileDefinitionInput) -> DeadlineProfileDetail {
    profiles::persist(
        &profiles::service(db, db.owner, Role::Owner),
        DeadlineProfileCollection::ForCase(db.case),
        DeadlineProfileCommand {
            operation_id: DeadlineProfileOperationId::new(),
            profile_id: DeadlineProfileId::new(),
            change: DeadlineProfileChange::Publish {
                definition: DeadlineProfileDefinition::new(definition).unwrap(),
            },
        },
    )
}
fn assert_historical(db: &dl::Fixture, expected: &DeadlineDetail) {
    let actual = dl::service(db, db.owner, Role::Owner)
        .get("session", db.case, expected.id, Some(expected.revision))
        .unwrap();
    assert_eq!(actual, *expected);
    assert_eq!(
        deadline_evaluation_record_bytes(&actual.calculation.result),
        deadline_evaluation_record_bytes(&expected.calculation.result),
    );
    deadline_receipt_matches(&infrastructure::RingSha256Hasher, &actual).unwrap();
}

#[test]
fn monthly_missing_homologous_day_remains_blocked_after_the_source_changes() {
    let Some(db) = dl::Fixture::new() else { return };
    let workflow = dl::service(&db, db.owner, Role::Owner);
    let mut definition = profiles::input(Some(db.case));
    definition.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::CivilMonths {
        quantity: NonZeroU32::MIN,
        final_day: FinalDayPolicy::Preserve,
    });
    definition.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::CivilCandidate {
            date: "2026-02-06".parse().unwrap(),
        });
    let profile = publish(&db, definition);
    let source = facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        inputs::dated_resolution("2026-01-31"),
    );
    let record = dl::persist(
        &workflow,
        db.case,
        dl::human(
            dl::command(&db, &profile, &source),
            Some(dl::FOLLOW_RESOLUTION),
        ),
    );
    assert!(record.calculation.result.due_at().is_none());
    assert_eq!(
        record.calculation.result.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::MissingHomologousDay {
            year: 2026,
            month: 2,
            requested_day: 31,
        })
    );
    assert!(matches!(
        record.calculation.result.arithmetic().unwrap().trace(),
        [DeadlineTraceRecord::CivilMonths {
            candidate: None,
            requested_day: 31,
            ..
        }]
    ));
    facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        inputs::correct_resolution(&source, "2026-01-28"),
    );
    assert_historical(&db, &record);
}

#[test]
fn ordered_hours_preserve_missing_duration_then_use_the_declared_quantity_and_offset() {
    let Some(db) = dl::Fixture::new() else { return };
    let workflow = dl::service(&db, db.owner, Role::Owner);
    let at = DeclaredProceduralTime::second(
        "2026-01-06".parse().unwrap(),
        14,
        30,
        7,
        Some(UtcOffset::from_hms(-6, 0, 0).unwrap()),
    )
    .unwrap();
    let mut definition = profiles::input(Some(db.case));
    definition.trigger = TriggerRequirement::Qualified {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        family: TriggerFamily::Resolution,
    };
    definition.template = DeadlineRuleTemplate::Ordered {
        unit: OrderedDeadlineUnit::ElapsedHours,
        maximum: NonZeroU32::new(72),
    };
    definition.completion = DeadlineCompletionPolicy::ArithmeticInstant;
    definition.examples[0].anchor = at;
    definition.examples[0].ordered_quantity = NonZeroU32::new(24);
    definition.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate {
            instant: at.instant_value().unwrap() + Duration::hours(24),
        });
    let profile = publish(&db, definition);
    let source = dl::source(&db);
    let mut command = dl::command(&db, &profile, &source);
    dl::definition_mut(&mut command)
        .input
        .selection
        .qualification = Some(QualifiedTriggerTime {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        at,
        statement: dl::text("Declared ordered start"),
        locator: dl::label("Resolution page 1"),
    });
    let blocked = dl::persist(
        &workflow,
        db.case,
        dl::human(command, Some(dl::FOLLOW_RESOLUTION)),
    );
    assert!(blocked.calculation.result.rule().is_none());
    assert!(blocked.calculation.result.due_at().is_none());
    let mut correction = dl::correct(&blocked);
    dl::definition_mut(&mut correction).input.ordered_quantity = NonZeroU32::new(24);
    let calculated = dl::persist(
        &workflow,
        db.case,
        dl::human(correction, Some(dl::FOLLOW_RESOLUTION)),
    );
    assert_eq!(
        calculated.calculation.result.due_at(),
        Some((at.instant_value().unwrap() + Duration::hours(24)).to_offset(UtcOffset::UTC))
    );
    let trace = calculated.calculation.result.arithmetic().unwrap();
    assert_eq!(trace.anchor().offset(), at.offset());
    assert!(
        matches!(trace.trace(), [DeadlineTraceRecord::ElapsedHours { quantity, .. }]
        if quantity.get() == 24)
    );
    assert_historical(&db, &blocked);
    assert_historical(&db, &calculated);
}

#[test]
fn notification_capture_preserves_distinct_selected_and_head_parent_revisions() {
    let Some(db) = dl::Fixture::new() else { return };
    let fact_workflow = facts::service(&db, db.owner, Role::Owner);
    let parent = facts::persist(
        &fact_workflow,
        db.case,
        inputs::dated_resolution("2026-01-05"),
    );
    let first = deadline_input_history_support::notice(&db, facts::resolution_ref(&parent), None);
    let new_parent = facts::persist(
        &fact_workflow,
        db.case,
        inputs::correct_resolution(&parent, "2026-01-08"),
    );
    let head = deadline_input_history_support::notice(
        &db,
        facts::resolution_ref(&new_parent),
        Some(&first),
    );
    let mut definition = profiles::input(Some(db.case));
    definition.trigger = TriggerRequirement::SourceField(TriggerField::NotificationPracticedAt);
    let profile = publish(&db, definition);
    let mut command = dl::command(&db, &profile, &parent);
    dl::definition_mut(&mut command).input.selection = inputs::fact_request(&first).trigger;
    let policies = dl::TrackingPolicies {
        source: dl::TrackingPolicy::Fixed,
        ..dl::FOLLOW_RESOLUTION
    };
    let record = dl::persist(
        &dl::service(&db, db.owner, Role::Owner),
        db.case,
        dl::human(command, Some(policies)),
    );
    assert_eq!(
        record.calculation.material.source,
        Some(DeadlineSourceDetail::Fact(Box::new(first)))
    );
    assert_eq!(
        record.calculation.material.source_head,
        Some(DeadlineSourceDetail::Fact(Box::new(head.clone())))
    );
    facts::persist(&fact_workflow, db.case, facts::withdraw(&head));
    assert_historical(&db, &record);
}

#[test]
fn hearing_agreement_and_calendar_captures_survive_head_replacement_and_retirement() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let first = inputs::hearing_result(&mut db, true);
    let head = inputs::corrected_result(&db, &first);
    let calendar = inputs::calendar(&db);
    let calendar_workflow = judicial_calendar_database_support::service(&db, db.owner, Role::Owner);
    let calendar_head = judicial_calendar_database_support::persist(
        &calendar_workflow,
        judicial_calendar_database_support::replace(&calendar),
    );
    let mut definition = profiles::input(Some(db.case));
    definition.trigger = TriggerRequirement::SourceField(TriggerField::HearingSessionEventTime);
    let profile = publish(&db, definition);
    let source = dl::source(&db);
    let mut command = dl::command(&db, &profile, &source);
    let input = &mut dl::definition_mut(&mut command).input;
    input.selection = inputs::request(
        db.case,
        TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: first.snapshot.hearing_id,
            result_id: first.snapshot.id,
            revision: first.snapshot.revision,
            agreement_id: Some(HearingResultAgreementId::from_uuid(uuid::Uuid::nil())),
        }),
    )
    .trigger;
    input.calendar = Some(DeadlineCalendarRef {
        id: calendar.id,
        revision: calendar.revision,
    });
    let policies = dl::TrackingPolicies {
        source: dl::TrackingPolicy::Fixed,
        calendar: dl::TrackingPolicy::Fixed,
        ..dl::FOLLOW_RESOLUTION
    };
    let record = dl::persist(
        &dl::service(&db, db.owner, Role::Owner),
        db.case,
        dl::human(command, Some(policies)),
    );
    assert_eq!(
        record.calculation.material.source,
        Some(DeadlineSourceDetail::HearingResult(Box::new(first)))
    );
    assert_eq!(
        record.calculation.material.source_head,
        Some(DeadlineSourceDetail::HearingResult(Box::new(head.clone())))
    );
    assert_eq!(record.calculation.material.calendar, Some(calendar));
    assert_eq!(
        record.calculation.material.calendar_head,
        Some(calendar_head.clone())
    );
    inputs::retired_result(&db, &head);
    judicial_calendar_database_support::persist(
        &calendar_workflow,
        judicial_calendar_database_support::retire(&calendar_head),
    );
    assert_historical(&db, &record);
}
