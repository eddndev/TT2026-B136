#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_service_support;
mod deadline_support;

use application::{deadline_currentness::DeadlineCurrent, deadlines::*, ApplicationError};
use deadline_service_support::*;
use domain::{cases::CaseId, identity::Role};

fn current() -> DeadlineCurrent {
    DeadlineCurrent::historical(hasher().as_ref(), &captured()).unwrap()
}

#[test]
fn current_query_authenticates_before_loading_and_reauthenticates_before_disclosure() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let expected = current();
        let id = expected.detail().id;
        let response = expected.clone();
        let mut store = MockStore::new();
        store
            .expect_current()
            .times(1)
            .return_once(move |actor, case, requested| {
                assert_eq!(actor, owner());
                assert_eq!(case, case_id());
                assert_eq!(requested, id);
                Ok(response)
            });
        let (workflow, _) = service(store, identity(role, 2));
        assert_eq!(
            workflow.current("session", case_id(), id).unwrap(),
            expected
        );
    }
}

#[test]
fn client_cannot_query_current_deadlines() {
    let (workflow, _) = service(MockStore::new(), identity(Role::Client, 1));
    assert!(matches!(
        workflow.current("session", case_id(), captured().id),
        Err(ApplicationError::PermissionDenied)
    ));
}

#[test]
fn a_current_projection_must_belong_to_the_requested_case_and_deadline() {
    for wrong_case in [false, true] {
        let result = current();
        let id = if wrong_case {
            result.detail().id
        } else {
            DeadlineId::new()
        };
        let case = if wrong_case { CaseId::new() } else { case_id() };
        let mut store = MockStore::new();
        store
            .expect_current()
            .times(1)
            .return_once(move |_, _, _| Ok(result));
        let (workflow, _) = service(store, identity(Role::Owner, 1));
        invalid(workflow.current("session", case, id).map(|_| ()));
    }
}

#[test]
fn current_query_rejects_revocation_or_identity_changes_after_reading() {
    for mutation in 0..4 {
        let result = current();
        let id = result.detail().id;
        let mut store = MockStore::new();
        store
            .expect_current()
            .times(1)
            .return_once(move |_, _, _| Ok(result));
        let mut authentication = MockIdentity::new();
        authentication
            .expect_authenticate()
            .times(1)
            .return_once(|_| Ok(principal(Role::Owner)));
        authentication
            .expect_authenticate()
            .times(1)
            .return_once(move |_| {
                let mut actor = principal(Role::Owner);
                match mutation {
                    0 => return Err(ApplicationError::InvalidSession),
                    1 => actor.id = domain::identity::UserId::new(),
                    2 => actor.role = Role::Litigator,
                    _ => actor.email = "changed@example.test".into(),
                }
                Ok(actor)
            });
        let (workflow, _) = service(store, authentication);
        assert!(workflow.current("session", case_id(), id).is_err());
    }
}
