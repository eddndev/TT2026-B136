#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_service_support;
mod deadline_support;
use application::{deadlines::*, ApplicationError};
use deadline_service_support::*;
use deadline_service_support::{prepare_human as prepare, tracked_detail as detail};
use domain::{
    crypto::Sha256Digest,
    identity::{Role, UserId},
    procedural_facts::{FactDeclaration, FactLabel},
};
use mockall::Sequence;

#[test]
fn role_and_invalid_session_fail_before_every_store_operation() {
    for operation in 0..5 {
        let (workflow, clock) = service(MockStore::new(), identity(Role::Client, 1));
        assert!(matches!(
            run(&workflow, operation),
            Err(ApplicationError::PermissionDenied)
        ));
        assert_eq!(clock.calls(), 0);
        let mut session = MockIdentity::new();
        session
            .expect_authenticate()
            .times(1)
            .returning(|_| Err(ApplicationError::InvalidSession));
        let (workflow, _) = service(MockStore::new(), session);
        assert!(matches!(
            run(&workflow, operation),
            Err(ApplicationError::InvalidSession)
        ));
    }
    for operation in [3, 4] {
        let (workflow, _) = service(MockStore::new(), identity(Role::Paralegal, 1));
        assert!(matches!(
            run(&workflow, operation),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn owner_and_litigator_prepare_exact_full_state_then_reauthenticate() {
    for role in [Role::Owner, Role::Litigator] {
        let (command, preparation) = fixture();
        let expected = prepare(command.clone(), preparation.clone()).unwrap();
        let mut sequence = Sequence::new();
        let mut session = MockIdentity::new();
        session
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(move |_| Ok(principal(role)));
        let mut store = MockStore::new();
        let wanted = command.clone();
        store
            .expect_prepare()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |actor, case, command| {
                assert_eq!(actor, owner());
                assert_eq!(case, case_id());
                assert_eq!(command, &wanted);
                Ok(preparation)
            });
        session
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(move |_| Ok(principal(role)));
        let (workflow, clock) = service(store, session);
        let draft = workflow
            .prepare("session", case_id(), human_command(command.clone()))
            .unwrap();
        assert_eq!(draft.case_id, case_id());
        assert_eq!(draft.actor, owner());
        assert_eq!(draft.command, command);
        assert_eq!(draft.result_revision, DeadlineRevision::initial());
        assert_eq!(&draft.definition, expected.definition());
        assert_eq!(&draft.calculation, expected.calculation());
        assert_eq!(&draft.responsible, expected.responsible());
        assert_eq!(&draft.attention, expected.attention());
        assert_eq!(draft.status, expected.status());
        assert_eq!(draft.submission_digest, expected.submission_digest());
        assert_eq!(clock.calls(), 0);
    }
}

#[test]
fn blocked_calculation_survives_service_preparation_without_an_invented_due_date() {
    let (mut command, preparation) = fixture();
    if let DeadlineChange::Register { definition } = &mut command.change {
        definition.input.qualification.scope_applies =
            FactDeclaration::Unknown(evaluation::text("Applicability unconfirmed"));
    }
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _| Ok(preparation));
    let (workflow, _) = service(store, identity(Role::Owner, 2));
    let draft = workflow
        .prepare("session", case_id(), human_command(command))
        .unwrap();
    assert!(draft.calculation.result.due_at().is_none());
    assert!(!draft.calculation.result.blocks().is_empty());
    assert!(draft.calculation.result.arithmetic().is_some());
}

#[test]
fn second_authentication_rejects_revocation_changed_actor_or_changed_role() {
    for operation in 0..5 {
        for change in 0..5 {
            let mut session = MockIdentity::new();
            let mut sequence = Sequence::new();
            session
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .returning(|_| Ok(principal(Role::Owner)));
            session
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .returning(move |_| match change {
                    0 => Err(ApplicationError::InvalidSession),
                    1 => {
                        let mut value = principal(Role::Owner);
                        value.id = UserId::new();
                        Ok(value)
                    }
                    2 => Ok(principal(Role::Litigator)),
                    3 => Ok(principal(Role::Client)),
                    _ => {
                        let mut value = principal(Role::Owner);
                        value.email = "changed@example.test".into();
                        Ok(value)
                    }
                });
            let mut store = MockStore::new();
            expect_operation(&mut store, operation);
            let (workflow, _) = service(store, session);
            let result = run(&workflow, operation);
            if change == 3 {
                assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
            } else {
                assert!(matches!(result, Err(ApplicationError::InvalidSession)));
            }
        }
    }
}

#[test]
fn store_errors_propagate_without_fabricating_a_result_or_reauthenticating() {
    for operation in 0..5 {
        let mut store = MockStore::new();
        match operation {
            0 => {
                store
                    .expect_list()
                    .times(1)
                    .returning(|_, _, _, _| Err(ApplicationError::CaseNotFound));
            }
            1 => {
                store
                    .expect_get()
                    .times(1)
                    .returning(|_, _, _, _, _| Err(ApplicationError::CaseNotFound));
            }
            2 => {
                store
                    .expect_history()
                    .times(1)
                    .returning(|_, _, _, _, _| Err(ApplicationError::CaseNotFound));
            }
            _ => {
                store
                    .expect_prepare()
                    .times(1)
                    .returning(|_, _, _| Err(ApplicationError::CaseNotFound));
            }
        }
        let (workflow, _) = service(store, identity(Role::Owner, 1));
        assert!(matches!(
            run(&workflow, operation),
            Err(ApplicationError::CaseNotFound)
        ));
    }
    let mut store = MockStore::new();
    expect_operation(&mut store, 4);
    store
        .expect_commit()
        .times(1)
        .returning(|_, _| Err(ApplicationError::Deadline(DeadlineError::RevisionConflict)));
    let (workflow, _) = service(store, identity(Role::Owner, 2));
    assert!(matches!(
        run(&workflow, 4),
        Err(ApplicationError::Deadline(DeadlineError::RevisionConflict))
    ));
}

#[test]
fn submission_digest_is_checked_before_reauthentication_or_commit() {
    let mut store = MockStore::new();
    expect_operation(&mut store, 4);
    let (workflow, _) = service(store, identity(Role::Owner, 1));
    assert!(matches!(
        workflow.submit(
            "session",
            case_id(),
            human_command(fixture().0),
            Sha256Digest::from_array([0; 32])
        ),
        Err(ApplicationError::Deadline(
            DeadlineError::SubmissionMismatch
        ))
    ));
}

#[test]
fn successful_commit_preserves_every_prepared_field_and_receipt() {
    let (command, preparation) = fixture();
    let prepared = prepare(command.clone(), preparation.clone()).unwrap();
    let wanted = detail(&prepared);
    let returned = wanted.clone();
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _| Ok(preparation));
    store
        .expect_commit()
        .times(1)
        .return_once(move |actor, value| {
            assert_eq!(actor, owner());
            assert_eq!(value.actor(), owner());
            assert_eq!(value.case_id(), case_id());
            assert_eq!(value.command(), prepared.command());
            assert_eq!(value.preparation(), prepared.preparation());
            assert_eq!(value.definition(), prepared.definition());
            assert_eq!(value.calculation(), prepared.calculation());
            assert_eq!(value.responsible(), prepared.responsible());
            assert_eq!(value.attention(), prepared.attention());
            assert_eq!(value.submission_digest(), prepared.submission_digest());
            Ok(returned)
        });
    let (workflow, _) = service(store, identity(Role::Owner, 2));
    assert_eq!(
        workflow
            .submit(
                "session",
                case_id(),
                human_command(command),
                wanted.receipt.submission_digest
            )
            .unwrap(),
        wanted
    );
}

#[test]
fn commit_reply_must_bind_full_state_including_title_attention_responsible_and_sources() {
    for mutation in 0..7 {
        let (command, preparation) = fixture();
        let mut row = detail(&prepare(command, preparation).unwrap());
        match mutation {
            0 => row.definition.title = FactLabel::new("Unrequested title").unwrap(),
            1 => row.attention = attention(),
            2 => row.responsible.email = "other@example.com".into(),
            3 => row.calculation.material.source_head = None,
            4 => row.status = DeadlineStatus::Retired,
            5 => {
                row.recorded_by = DeadlineActorSnapshot::User {
                    id: UserId::new(),
                    email: "owner@example.com".into(),
                };
            }
            _ => row.receipt.operation_id = DeadlineOperationId::new(),
        }
        let mut store = MockStore::new();
        expect_operation(&mut store, 4);
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _| Ok(row));
        let (workflow, _) = service(store, identity(Role::Owner, 2));
        assert!(run(&workflow, 4).is_err());
    }
}

#[test]
fn exhausted_revision_is_rejected_before_preparation_port() {
    let base = captured();
    let (command, _) = followup(
        &base,
        DeadlineChange::Retire {
            expected_revision: DeadlineRevision::new(u32::MAX).unwrap(),
            reason: evaluation::text("Retire"),
        },
    );
    let (workflow, _) = service(MockStore::new(), identity(Role::Owner, 1));
    assert!(matches!(
        workflow.prepare("session", case_id(), human_command(command)),
        Err(ApplicationError::Deadline(DeadlineError::RevisionExhausted))
    ));
}
