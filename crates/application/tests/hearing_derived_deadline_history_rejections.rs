use crate::hearing_derived_deadline_history::captured_creation;
use crate::hearing_result_support::hasher;
use application::{
    deadline_reevaluation::DependencyFamily,
    deadlines::{DeadlineChange, DeadlineHumanCommand, DeadlineId, DeadlineOperationId},
    hearing_derived_deadlines::*,
    hearing_results::HearingResultOperationId,
};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
use time::{Duration, UtcOffset};
use uuid::Uuid;

#[test]
fn history_rejects_substituted_material_authorship_commands_and_ordinary_records() {
    let original = captured_creation(false).evidence();
    let foreign = captured_creation(false).evidence();
    for field in [
        "foreign_material",
        "actor_id",
        "actor_email",
        "actor_role",
        "result_command",
        "deadline_operation",
        "deadline_id",
        "material_responsible",
        "profile_head",
        "calendar_capture",
        "recorded_result",
        "result_receipt",
        "result_capture_time",
        "result_capture_offset",
        "deadline_record",
        "deadline_receipt",
        "deadline_source",
        "deadline_time",
        "deadline_offset",
        "review_digest",
        "capture_digest",
    ] {
        let mut evidence = original.clone();
        let corrupt = Sha256Digest::from_array([11; 32]);
        match field {
            "foreign_material" => evidence.material = foreign.material.clone(),
            "actor_id" => evidence.actor.id = UserId::new(),
            "actor_email" => evidence.actor.email = "different@example.com".into(),
            "actor_role" => evidence.actor.role = Role::Litigator,
            "result_command" => {
                evidence.command.result.operation_id = HearingResultOperationId::new()
            }
            "deadline_operation" | "deadline_id" => {
                let (mut command, policies) = evidence.command.deadline.clone().into_parts();
                if field == "deadline_operation" {
                    command.operation_id = DeadlineOperationId::new();
                } else {
                    command.deadline_id = DeadlineId::new();
                }
                evidence.command.deadline = DeadlineHumanCommand::new(command, policies).unwrap();
            }
            "material_responsible" => {
                evidence.material.responsible.email = "different@example.com".into()
            }
            "profile_head" => {
                evidence.material.profile_head.recorded_by.email = "different@example.com".into()
            }
            "calendar_capture" => {
                evidence.material.calendar.as_mut().unwrap().recorded_at += Duration::seconds(1);
            }
            "recorded_result" => evidence.result = foreign.result.clone(),
            "result_receipt" => evidence.result.snapshot.receipt.submission_digest = corrupt,
            "result_capture_time" => evidence.result.snapshot.recorded_at += Duration::seconds(1),
            "result_capture_offset" => {
                evidence.result.snapshot.recorded_at = evidence
                    .result
                    .snapshot
                    .recorded_at
                    .to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap());
                assert_eq!(
                    evidence.result.snapshot.recorded_at,
                    original.result.snapshot.recorded_at
                );
            }
            "deadline_record" => evidence.deadline = foreign.deadline.clone(),
            "deadline_receipt" => evidence.deadline.receipt.capture_digest = corrupt,
            "deadline_source" => {
                evidence.deadline.calculation.material.source =
                    foreign.deadline.calculation.material.source.clone();
            }
            "deadline_time" => evidence.deadline.recorded_at += Duration::seconds(1),
            "deadline_offset" => {
                evidence.deadline.recorded_at = evidence
                    .deadline
                    .recorded_at
                    .to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap());
                assert_eq!(evidence.deadline.recorded_at, original.deadline.recorded_at);
            }
            "review_digest" => evidence.review_digest = corrupt,
            _ => evidence.capture_digest = corrupt,
        }
        assert!(
            restore_hearing_derived_deadline(hasher().as_ref(), evidence).is_err(),
            "{field}"
        );
    }
}

#[test]
fn history_verifies_event_scope_even_where_the_capture_encoding_uses_result_scope() {
    let original = captured_creation(false).evidence();
    for field in [
        "family",
        "root",
        "revision",
        "case",
        "missing_case",
        "hearing",
        "missing_hearing",
        "operation",
        "sequence",
        "zero_sequence",
        "overflow_sequence",
    ] {
        let mut evidence = original.clone();
        let event = &mut evidence.source_event;
        match field {
            "family" => event.family = DependencyFamily::Notification,
            "root" => event.source_id = Uuid::new_v4(),
            "revision" => event.revision = 2,
            "case" => event.case_id = Some(CaseId::new()),
            "missing_case" => event.case_id = None,
            "hearing" => event.hearing_id = Some(Uuid::new_v4()),
            "missing_hearing" => event.hearing_id = None,
            "operation" => event.operation_id = Uuid::new_v4(),
            "sequence" => event.sequence += 1,
            "zero_sequence" => event.sequence = 0,
            _ => event.sequence = (i64::MAX as u64) + 1,
        }
        assert!(
            restore_hearing_derived_deadline(hasher().as_ref(), evidence).is_err(),
            "{field}"
        );
    }
}

#[test]
fn a_replacement_deadline_instruction_cannot_be_adopted_as_the_original_history() {
    let mut evidence = captured_creation(false).evidence();
    let (mut command, policies) = evidence.command.deadline.clone().into_parts();
    let DeadlineChange::Register { definition } = &command.change else {
        panic!("registration expected")
    };
    command.change = DeadlineChange::Correct {
        expected_revision: evidence.deadline.revision,
        definition: definition.clone(),
        reason: domain::procedural_facts::FactText::new("Separate later correction").unwrap(),
    };
    evidence.command.deadline = DeadlineHumanCommand::new(command, policies).unwrap();
    assert!(restore_hearing_derived_deadline(hasher().as_ref(), evidence).is_err());
}
