#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;
use application::{cases::*, deadlines::*};
use deadline_support::{evaluation::inputs, *};
use domain::{
    case_administration::{CaseAdministrationValues, CaseAdministrativeStatus, CaseRevision},
    cases::CaseMetadata,
};

fn administration(status: CaseAdministrativeStatus) -> CurrentCaseAdministration {
    let values =
        CaseAdministrationValues::basic(CaseMetadata::new("Renamed case", "REF-2").unwrap())
            .with_status(status);
    CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
        case_id: inputs::case_id(),
        revision: CaseRevision::FIRST,
        values_digest: case_administration_digest(inputs::hasher().as_ref(), &values),
        values,
        changed_at: case_support::instant(),
        changed_by: CaseActorSnapshot {
            id: owner(),
            email: "owner@example.com".into(),
        },
    }))
}

#[test]
fn active_administrative_update_does_not_change_the_reviewed_submission() {
    let (command, mut preparation) = fixture();
    let first = prepare(command.clone(), preparation.clone()).unwrap();
    let current = administration(CaseAdministrativeStatus::Active);
    preparation.administration = current.clone();
    preparation
        .resolved
        .as_mut()
        .unwrap()
        .material
        .administration = current;
    let second = prepare(command, preparation).unwrap();
    assert_eq!(first.calculation().result, second.calculation().result);
    assert_ne!(
        first.calculation().material.administration,
        second.calculation().material.administration
    );
    assert_eq!(first.submission_digest(), second.submission_digest());
    assert_ne!(first.capture_digest(), second.capture_digest());
    deadline_receipt_matches(inputs::hasher().as_ref(), &detail(&first)).unwrap();
    deadline_receipt_matches(inputs::hasher().as_ref(), &detail(&second)).unwrap();
}

#[test]
fn closure_blocks_recording_but_does_not_invalidate_captured_history() {
    let (command, mut preparation) = fixture();
    let first = detail(&prepare(command.clone(), preparation.clone()).unwrap());
    let current = administration(CaseAdministrativeStatus::Closed);
    preparation.administration = current.clone();
    preparation
        .resolved
        .as_mut()
        .unwrap()
        .material
        .administration = current;
    assert!(matches!(
        prepare(command, preparation),
        Err(application::ApplicationError::CaseClosed)
    ));
    deadline_receipt_matches(inputs::hasher().as_ref(), &first).unwrap();
}
