#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_service_support;
mod deadline_support;
use application::{deadlines::*, ApplicationError};
use deadline_service_support::*;
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use mockall::Sequence;
use uuid::Uuid;

fn id(n: u128) -> UserId {
    UserId::from_uuid(Uuid::from_u128(n))
}
fn query() -> DeadlineResponsibleQuery {
    DeadlineResponsibleQuery::new(2, None).unwrap()
}
fn page() -> DeadlineResponsiblePage {
    DeadlineResponsiblePage {
        case_id: case_id(),
        responsibles: vec![
            DeadlineResponsibleCandidate {
                id: id(1),
                email: "one@example.test".into(),
                role: Role::Owner,
            },
            DeadlineResponsibleCandidate {
                id: id(2),
                email: "two@example.test".into(),
                role: Role::Paralegal,
            },
        ],
        has_more: true,
        next_after_id: Some(id(2)),
    }
}

#[test]
fn responsible_queries_are_bounded_and_preserve_nil_cursor() {
    for limit in [0, 101, u32::MAX] {
        assert!(DeadlineResponsibleQuery::new(limit, None).is_err());
    }
    for limit in [1, 20, 100] {
        let query = DeadlineResponsibleQuery::new(limit, Some(id(0))).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(query.after_id(), Some(id(0)));
    }
}

#[test]
fn responsible_selector_authenticates_staff_before_and_after_port_with_case_scope() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let mut store = MockStore::new();
        let expected = page();
        let mut sequence = Sequence::new();
        let mut identity = MockIdentity::new();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(move |_| Ok(principal(role)));
        store
            .expect_responsibles()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |actor, case, q, _| {
                assert_eq!(actor, owner());
                assert_eq!(case, case_id());
                assert_eq!(q, query());
                Ok(page())
            });
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(move |_| Ok(principal(role)));
        let (workflow, clock) = service(store, identity);
        assert_eq!(
            workflow
                .responsibles("session", case_id(), query())
                .unwrap(),
            expected
        );
        assert_eq!(clock.calls(), 1);
    }
}

#[test]
fn responsible_selector_denies_client_and_invalid_session_before_store() {
    let (workflow, clock) = service(MockStore::new(), identity(Role::Client, 1));
    assert!(matches!(
        workflow.responsibles("session", case_id(), query()),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(clock.calls(), 0);
    let mut session = MockIdentity::new();
    session
        .expect_authenticate()
        .times(1)
        .returning(|_| Err(ApplicationError::InvalidSession));
    let (workflow, clock) = service(MockStore::new(), session);
    assert!(matches!(
        workflow.responsibles("session", case_id(), query()),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(clock.calls(), 0);
}

#[test]
fn responsible_selector_rejects_session_revocation_actor_and_role_changes_after_read() {
    for change in 0..5 {
        let mut session = MockIdentity::new();
        let mut sequence = Sequence::new();
        session
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_| Ok(principal(Role::Owner)));
        let mut store = MockStore::new();
        store
            .expect_responsibles()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_, _, _, _| Ok(page()));
        session
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(move |_| match change {
                0 => Err(ApplicationError::InvalidSession),
                1 => {
                    let mut actor = principal(Role::Owner);
                    actor.id = id(7);
                    Ok(actor)
                }
                2 => Ok(principal(Role::Litigator)),
                3 => Ok(principal(Role::Client)),
                _ => {
                    let mut actor = principal(Role::Owner);
                    actor.email = "changed@example.test".into();
                    Ok(actor)
                }
            });
        let (workflow, _) = service(store, session);
        let result = workflow.responsibles("session", case_id(), query());
        if change == 3 {
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
        } else {
            assert!(matches!(result, Err(ApplicationError::InvalidSession)));
        }
    }
}

#[test]
fn responsible_selector_rejects_malformed_scope_role_order_size_and_cursor_without_returning_rows()
{
    for mutation in 0..10 {
        let mut response = page();
        match mutation {
            0 => response.case_id = CaseId::new(),
            1 => response.responsibles[0].role = Role::Client,
            2 => response.responsibles[0].email = " \t ".into(),
            3 => response.responsibles.swap(0, 1),
            4 => response.responsibles[1].id = id(1),
            5 => response.responsibles.push(response.responsibles[1].clone()),
            6 => {
                response.responsibles.pop();
            }
            7 => response.next_after_id = Some(id(8)),
            8 => response.next_after_id = None,
            _ => response.has_more = false,
        }
        let mut store = MockStore::new();
        store
            .expect_responsibles()
            .times(1)
            .return_once(move |_, _, _, _| Ok(response));
        let (workflow, _) = service(store, identity(Role::Owner, 1));
        invalid(
            workflow
                .responsibles("session", case_id(), query())
                .map(|_| ()),
        );
    }
    let mut store = MockStore::new();
    store
        .expect_responsibles()
        .times(1)
        .returning(|_, _, _, _| Ok(page()));
    let (workflow, _) = service(store, identity(Role::Owner, 1));
    invalid(
        workflow
            .responsibles(
                "session",
                case_id(),
                DeadlineResponsibleQuery::new(2, Some(id(1))).unwrap(),
            )
            .map(|_| ()),
    );
}

#[test]
fn empty_responsible_page_preserves_scope_and_store_failure_propagates() {
    let mut store = MockStore::new();
    store
        .expect_responsibles()
        .times(1)
        .returning(|_, _, _, _| {
            Ok(DeadlineResponsiblePage {
                case_id: case_id(),
                responsibles: vec![],
                has_more: false,
                next_after_id: None,
            })
        });
    let (workflow, _) = service(store, identity(Role::Paralegal, 2));
    assert!(workflow
        .responsibles("session", case_id(), query())
        .unwrap()
        .responsibles
        .is_empty());
    let mut store = MockStore::new();
    store
        .expect_responsibles()
        .times(1)
        .returning(|_, _, _, _| Err(ApplicationError::CaseNotFound));
    let (workflow, _) = service(store, identity(Role::Owner, 1));
    assert!(matches!(
        workflow.responsibles("session", case_id(), query()),
        Err(ApplicationError::CaseNotFound)
    ));
}
