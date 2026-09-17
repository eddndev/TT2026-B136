#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;
use application::{deadline_evaluations::*, deadline_profiles::*, deadlines::*};
use deadline_support::{evaluation::text, *};
use domain::{
    deadline_triggers::TriggerBlock,
    identity::{Role, UserId},
    procedural_facts::FactDeclaration,
};
use uuid::Uuid;

#[test]
fn registration_captures_a_due_instant_exact_profile_and_all_inputs() {
    let (command, preparation) = fixture();
    let original = preparation.resolved.clone().unwrap();
    let prepared = prepare(command, preparation).unwrap();
    assert_eq!(prepared.calculation().profile, original.profile);
    assert_eq!(prepared.calculation().material, original.material);
    assert!(prepared.calculation().result.due_at().is_some());
    assert!(prepared.calculation().result.blocks().is_empty());
    assert_eq!(prepared.status(), DeadlineStatus::Active);
    assert_eq!(prepared.attention(), &DeadlineAttention::Pending);
    assert_eq!(prepared.responsible().id, owner());
}

#[test]
fn blocked_unknown_source_can_be_registered_without_fabricating_a_due_date() {
    let (mut command, mut preparation) = fixture();
    let DeadlineChange::Register { definition } = &mut command.change else {
        unreachable!()
    };
    definition.input.selection.source = FactDeclaration::Unknown(text("Awaiting the exact source"));
    let material = &mut preparation.resolved.as_mut().unwrap().material;
    material.source = None;
    material.source_head = None;
    let prepared = prepare(command, preparation).unwrap();
    assert!(prepared.calculation().result.due_at().is_none());
    assert_eq!(
        prepared.calculation().result.blocks(),
        &[DeadlineEvaluationBlock::Trigger(
            TriggerBlock::UnknownSource
        )]
    );
}

#[test]
fn responsible_must_be_the_selected_staff_account() {
    for (id, role) in [
        (owner(), Role::Client),
        (UserId::from_uuid(Uuid::nil()), Role::Owner),
    ] {
        let (command, mut preparation) = fixture();
        let responsible = preparation.responsible.as_mut().unwrap();
        responsible.id = id;
        responsible.role = role;
        assert!(prepare(command, preparation).is_err());
    }
}

#[test]
fn missing_responsible_or_unrequested_resolved_material_is_rejected() {
    let (command, mut preparation) = fixture();
    preparation.responsible = None;
    assert!(prepare(command, preparation).is_err());
    let (command, mut preparation) = fixture();
    preparation.resolved = None;
    assert!(prepare(command, preparation).is_err());
}

#[test]
fn exact_profile_selection_cannot_cross_cases_or_silently_follow_a_new_head() {
    let (mut command, preparation) = fixture();
    let DeadlineChange::Register { definition } = &mut command.change else {
        unreachable!()
    };
    definition.input.selection.case_id = domain::cases::CaseId::from_uuid(Uuid::nil());
    assert!(prepare(command, preparation).is_err());
    let (mut command, preparation) = fixture();
    let DeadlineChange::Register { definition } = &mut command.change else {
        unreachable!()
    };
    definition.profile.revision = DeadlineProfileRevision::new(2).unwrap();
    assert!(prepare(command, preparation).is_err());
}

#[test]
fn attention_retains_the_calculation_without_resolving_new_sources() {
    let (command, preparation) = fixture();
    let first = detail(&prepare(command, preparation).unwrap());
    let (command, preparation) = followup(
        &first,
        DeadlineChange::SetAttention {
            expected_revision: first.revision,
            attention: attention(),
            reason: text("Record declared filing"),
        },
    );
    let prepared = prepare(command, preparation).unwrap();
    assert_eq!(prepared.calculation(), &first.calculation);
    assert_eq!(prepared.definition(), &first.definition);
    assert_eq!(prepared.attention(), &attention());
    assert_eq!(prepared.command().result_revision().unwrap().get(), 2);
}

#[test]
fn correcting_the_calculation_keeps_recorded_attention() {
    let (command, preparation) = fixture();
    let fresh = preparation.clone();
    let first = detail(&prepare(command, preparation).unwrap());
    let (command, preparation) = followup(
        &first,
        DeadlineChange::SetAttention {
            expected_revision: first.revision,
            attention: attention(),
            reason: text("Record filing"),
        },
    );
    let second = detail(&prepare(command, preparation).unwrap());
    let (command, mut preparation) = followup(
        &second,
        DeadlineChange::Correct {
            expected_revision: second.revision,
            definition: second.definition.clone(),
            reason: text("Explicitly recheck the inputs"),
        },
    );
    preparation.resolved = fresh.resolved;
    preparation.responsible = fresh.responsible;
    let prepared = prepare(command, preparation).unwrap();
    assert_eq!(prepared.attention(), &attention());
}

#[test]
fn retirement_preserves_attention_and_is_terminal() {
    let (command, preparation) = fixture();
    let first = detail(&prepare(command, preparation).unwrap());
    let (command, preparation) = followup(
        &first,
        DeadlineChange::Retire {
            expected_revision: first.revision,
            reason: text("No longer tracked"),
        },
    );
    let retired = detail(&prepare(command, preparation).unwrap());
    assert_eq!(retired.status, DeadlineStatus::Retired);
    assert_eq!(retired.calculation, first.calculation);
    let (command, preparation) = followup(
        &retired,
        DeadlineChange::SetAttention {
            expected_revision: retired.revision,
            attention: attention(),
            reason: text("Late edit"),
        },
    );
    assert!(matches!(
        prepare(command, preparation),
        Err(application::ApplicationError::Deadline(
            DeadlineError::Retired
        ))
    ));
}

#[test]
fn stale_revision_and_existing_registration_fail_before_another_revision() {
    let (command, preparation) = fixture();
    let first = detail(&prepare(command.clone(), preparation.clone()).unwrap());
    let mut existing = preparation;
    existing.base = Some(first.clone());
    assert!(prepare(command, existing).is_err());
    let (command, preparation) = followup(
        &first,
        DeadlineChange::Retire {
            expected_revision: DeadlineRevision::new(2).unwrap(),
            reason: text("Stale command"),
        },
    );
    assert!(matches!(
        prepare(command, preparation),
        Err(application::ApplicationError::Deadline(
            DeadlineError::RevisionConflict
        ))
    ));
}
