#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;
use application::{deadline_inputs::DeadlineSourceDetail, deadline_profiles::*, deadlines::*};
use deadline_support::{
    evaluation::{inputs, text},
    *,
};
use domain::{crypto::Sha256Digest, identity::UserId};
use uuid::Uuid;

#[test]
fn receipt_binds_applicability_even_when_the_due_instant_does_not_change() {
    let (command, preparation) = fixture();
    let first = detail(&prepare(command, preparation).unwrap());
    deadline_receipt_matches(inputs::hasher().as_ref(), &first).unwrap();
    let mut altered = first.clone();
    altered.definition.input.qualification.statement =
        text("A different declaration with the same outcome");
    assert!(deadline_receipt_matches(inputs::hasher().as_ref(), &altered).is_err());
    altered = first.clone();
    altered.responsible.email = "changed@example.com".into();
    assert!(deadline_receipt_matches(inputs::hasher().as_ref(), &altered).is_err());
    altered = first.clone();
    altered.recorded_by.id = UserId::from_uuid(Uuid::nil());
    assert!(deadline_receipt_matches(inputs::hasher().as_ref(), &altered).is_err());
    altered = first;
    altered.receipt.submission_digest = Sha256Digest::from_array([0; 32]);
    assert!(deadline_receipt_matches(inputs::hasher().as_ref(), &altered).is_err());
}

#[test]
fn observed_source_head_changes_submission_even_when_selected_history_and_due_stay_equal() {
    let (command, mut preparation) = fixture();
    let first = prepare(command.clone(), preparation.clone()).unwrap();
    preparation.resolved.as_mut().unwrap().material.source_head = Some(DeadlineSourceDetail::Fact(
        Box::new(inputs::resolution(2, false, "2026-01-10")),
    ));
    let second = prepare(command, preparation).unwrap();
    assert_eq!(first.calculation().result, second.calculation().result);
    assert_ne!(first.submission_digest(), second.submission_digest());
}

#[test]
fn unknown_source_does_not_hide_a_corrupt_profile_receipt() {
    let (mut command, mut preparation) = fixture();
    let DeadlineChange::Register { definition } = &mut command.change else {
        unreachable!()
    };
    definition.input.selection.source =
        domain::procedural_facts::FactDeclaration::Unknown(text("No source yet"));
    let resolved = preparation.resolved.as_mut().unwrap();
    resolved.material.source = None;
    resolved.material.source_head = None;
    resolved.profile.receipt.submission_digest = Sha256Digest::from_array([0; 32]);
    resolved.profile_head = resolved.profile.clone();
    assert!(prepare(command, preparation).is_err());
}

#[test]
fn replacement_and_retirement_of_profile_head_prevent_using_the_old_revision() {
    for retire in [false, true] {
        let (command, mut preparation) = fixture();
        let resolved = preparation.resolved.as_mut().unwrap();
        let head = &mut resolved.profile_head;
        let change = if retire {
            DeadlineProfileChange::Retire {
                expected_revision: head.revision,
                reason: text("Retired profile"),
            }
        } else {
            DeadlineProfileChange::Replace {
                expected_revision: head.revision,
                definition: head.definition.clone(),
                reason: text("Replaced profile"),
            }
        };
        let profile_command = DeadlineProfileCommand {
            operation_id: DeadlineProfileOperationId::new(),
            profile_id: head.id,
            change,
        };
        head.revision = profile_command.result_revision().unwrap();
        head.status = profile_command.result_status();
        head.reason = profile_command.reason().cloned();
        head.receipt = DeadlineProfileReceipt {
            operation_id: profile_command.operation_id,
            action: profile_command.action(),
            expected_revision: profile_command.expected_revision(),
            submission_digest: deadline_profile_submission_digest(
                inputs::hasher().as_ref(),
                head.recorded_by.id,
                &profile_command,
                head.algorithm,
                head.definition_digest,
            ),
        };
        deadline_profile_receipt_matches(inputs::hasher().as_ref(), head).unwrap();
        assert!(matches!(
            prepare(command, preparation),
            Err(application::ApplicationError::Deadline(
                DeadlineError::ProfileUnavailable
            ))
        ));
    }
}

#[test]
fn attention_rejects_supplied_current_inputs_and_retains_an_explicit_reopening() {
    let (command, preparation) = fixture();
    let resolved = preparation.resolved.clone();
    let first = detail(&prepare(command, preparation).unwrap());
    let (command, mut preparation) = followup(
        &first,
        DeadlineChange::SetAttention {
            expected_revision: first.revision,
            attention: attention(),
            reason: text("Declared action"),
        },
    );
    preparation.resolved = resolved;
    assert!(prepare(command.clone(), preparation.clone()).is_err());
    preparation.resolved = None;
    let attended = detail(&prepare(command, preparation).unwrap());
    let (command, preparation) = followup(
        &attended,
        DeadlineChange::SetAttention {
            expected_revision: attended.revision,
            attention: DeadlineAttention::Pending,
            reason: text("Prior action requires followup"),
        },
    );
    let reopened = prepare(command, preparation).unwrap();
    assert_eq!(reopened.attention(), &DeadlineAttention::Pending);
    assert_eq!(reopened.calculation(), &attended.calculation);
}
