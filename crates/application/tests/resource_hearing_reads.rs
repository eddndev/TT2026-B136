#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;
#[allow(dead_code)]
mod hearing_support;
use deadline_support::evaluation::inputs::facts as procedural_fact_service_support;
mod procedural_resource_support;
mod resource_activity_support;
mod resource_hearing_read_support;
mod resource_hearing_support;

use application::{resource_activities::*, resource_hearings::*, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    identity::{Role, UserId},
    resource_hearings::*,
};
use resource_hearing_read_support::*;

#[test]
fn read_query_is_bounded_and_preserves_the_exclusive_cursor() {
    let default = ResourceHearingReadQuery::default();
    assert_eq!((default.limit(), default.after_id()), (10, None));
    for limit in [1, 20] {
        let query = ResourceHearingReadQuery::new(limit, Some(id(7))).unwrap();
        assert_eq!((query.limit(), query.after_id()), (limit, Some(id(7))));
    }
    for limit in [0, 21, u16::MAX] {
        assert!(matches!(
            ResourceHearingReadQuery::new(limit, None),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
}

#[test]
fn all_staff_roles_read_exact_creation_without_becoming_its_historical_author() {
    let f = Fixture::new(Role::Owner);
    let saved = creation(&f, f.command.hearing_id, case_support::instant());
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let mut reader = f.actor.clone();
        reader.id = UserId::new();
        reader.email = "reader@example.com".into();
        reader.role = role;
        for list in [false, true] {
            let store = successful_store(&f, &reader, saved.clone(), list);
            let actual = read(&service(store, identity(&reader, 2), clock()), &f, list).unwrap();
            assert_eq!(actual, vec![saved.clone()]);
        }
    }
}

#[test]
fn client_is_denied_before_either_read_port_is_called() {
    let f = Fixture::new(Role::Client);
    for list in [false, true] {
        let service = service(MockReads::new(), identity(&f.actor, 1), clock());
        assert!(matches!(
            read(&service, &f, list),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn list_preserves_order_cursor_and_initial_association_without_head_filtering() {
    let f = Fixture::new(Role::Owner);
    let rows = vec![
        creation(&f, id(20), case_support::instant()),
        creation(&f, id(30), case_support::instant()),
    ];
    let mut expected = page(&f, rows);
    expected.has_more = true;
    expected.next_after_id = Some(id(30));
    let returned = expected.clone();
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _, query| {
            assert_eq!((query.limit(), query.after_id()), (2, Some(id(10))));
            Ok(returned)
        });
    let service = service(store, identity(&f.actor, 2), clock());
    assert_eq!(
        service
            .list(
                "session",
                f.case(),
                f.resource(),
                ResourceHearingReadQuery::new(2, Some(id(10))).unwrap()
            )
            .unwrap(),
        expected
    );
}

#[test]
fn empty_authorized_page_preserves_scope_and_has_no_continuation() {
    let f = Fixture::new(Role::Paralegal);
    let expected = page(&f, vec![]);
    let returned = expected.clone();
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _, _| Ok(returned));
    assert_eq!(
        service(store, identity(&f.actor, 2), clock())
            .list(
                "session",
                f.case(),
                f.resource(),
                ResourceHearingReadQuery::default()
            )
            .unwrap(),
        expected
    );
}

#[test]
fn malformed_list_order_scope_and_continuation_never_escape() {
    let f = Fixture::new(Role::Owner);
    let rows = vec![
        creation(&f, id(20), case_support::instant()),
        creation(&f, id(30), case_support::instant()),
    ];
    for fault in 0..11 {
        let mut returned = page(&f, rows.clone());
        match fault {
            0 => returned.items.reverse(),
            1 => returned.items[1] = returned.items[0].clone(),
            2 => returned
                .items
                .push(creation(&f, id(40), case_support::instant())),
            3 => returned.next_after_id = Some(id(30)),
            4 => returned.has_more = true,
            5 => {
                returned.has_more = true;
                returned.next_after_id = Some(id(20));
            }
            6 => {
                returned.items.pop();
                returned.has_more = true;
                returned.next_after_id = Some(id(20));
            }
            7 => {
                returned.items.clear();
                returned.has_more = true;
                returned.next_after_id = Some(id(30));
            }
            8 => {
                returned.items.clear();
                returned.case_id = CaseId::new();
            }
            9 => {
                returned.items.clear();
                returned.resource_id = ResourceId::new();
            }
            _ => {}
        }
        let mut store = MockReads::new();
        store
            .expect_list()
            .times(1)
            .return_once(move |_, _, _, _| Ok(returned));
        let after = if fault == 10 { id(20) } else { id(10) };
        let service = service(store, identity(&f.actor, 1), clock());
        stored_error(service.list(
            "session",
            f.case(),
            f.resource(),
            ResourceHearingReadQuery::new(2, Some(after)).unwrap(),
        ));
    }
}

#[test]
fn exact_get_rejects_valid_capture_from_another_requested_scope_or_revision() {
    let f = Fixture::new(Role::Owner);
    let saved = creation(&f, f.command.hearing_id, case_support::instant());
    for fault in 0..4 {
        let mut store = MockReads::new();
        let returned = saved.clone();
        store
            .expect_get()
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(returned));
        let (mut case, mut resource, mut hearing, mut revision) = (
            f.case(),
            f.resource(),
            f.command.hearing_id,
            ResourceHearingRevision::initial(),
        );
        match fault {
            0 => case = CaseId::new(),
            1 => resource = ResourceId::new(),
            2 => hearing = ResourceHearingId::new(),
            _ => revision = ResourceHearingRevision::new(2).unwrap(),
        }
        stored_error(service(store, identity(&f.actor, 1), clock()).get(
            "session",
            case,
            resource,
            hearing,
            Some(revision),
        ));
    }
}

#[test]
fn get_without_revision_preserves_the_port_head_selection() {
    let f = Fixture::new(Role::Owner);
    let expected = creation(&f, f.command.hearing_id, case_support::instant());
    let returned = expected.clone();
    let mut store = MockReads::new();
    store
        .expect_get()
        .times(1)
        .return_once(move |_, _, _, _, revision| {
            assert_eq!(revision, None);
            Ok(returned)
        });
    assert_eq!(
        service(store, identity(&f.actor, 2), clock())
            .get(
                "session",
                f.case(),
                f.resource(),
                f.command.hearing_id,
                None
            )
            .unwrap(),
        expected
    );
}

#[test]
fn both_reads_verify_the_capture_initial_link_and_durable_origin() {
    let f = Fixture::new(Role::Owner);
    let saved = creation(&f, f.command.hearing_id, case_support::instant());
    for list in [false, true] {
        for fault in 0..4 {
            let mut returned = saved.clone();
            match fault {
                0 => returned.hearing.capture_digest = Sha256Digest::from_array([7; 32]),
                1 => {
                    returned.association.receipt.capture_digest = Sha256Digest::from_array([8; 32])
                }
                2 => returned.origin.operation_id = ResourceHearingOperationId::new(),
                _ => returned.hearing.review.support.digest = Sha256Digest::from_array([9; 32]),
            }
            let store = successful_store(&f, &f.actor, returned, list);
            stored_error(read(
                &service(store, identity(&f.actor, 1), clock()),
                &f,
                list,
            ));
        }
    }
}

#[test]
fn authorization_absence_and_audit_failures_are_not_replaced_by_empty_success() {
    let f = Fixture::new(Role::Litigator);
    for list in [false, true] {
        for fault in 0..3 {
            let error = match fault {
                0 => ApplicationError::CaseNotFound,
                1 => ResourceActivityError::NotFound.into(),
                _ => ApplicationError::Port("read audit commit failed".into()),
            };
            let mut store = MockReads::new();
            if list {
                store
                    .expect_list()
                    .times(1)
                    .return_once(move |_, _, _, _| Err(error));
            } else {
                store
                    .expect_get()
                    .times(1)
                    .return_once(move |_, _, _, _, _| Err(error));
            }
            let result = read(&service(store, identity(&f.actor, 1), clock()), &f, list);
            match fault {
                0 => assert!(matches!(result, Err(ApplicationError::CaseNotFound))),
                1 => assert!(matches!(
                    result,
                    Err(ApplicationError::ResourceActivity(
                        ResourceActivityError::NotFound
                    ))
                )),
                _ => assert!(
                    matches!(result, Err(ApplicationError::Port(message)) if message == "read audit commit failed")
                ),
            }
        }
    }
}

#[test]
fn write_workflow_exposes_existing_explicit_prepare_and_exact_submit_replay() {
    let f = Fixture::new(Role::Owner);
    let saved = creation(&f, f.command.hearing_id, case_support::instant());
    let expected = saved.clone();
    let mut store = resource_hearing_support::MockStore::new();
    store
        .expect_prepare()
        .times(2)
        .returning(move |_, _, _, _| {
            Ok(ResourceHearingPreparation::Replay(Box::new(saved.clone())))
        });
    let service = resource_hearing_support::service(store, identity(&f.actor, 4));
    let workflow: &dyn ResourceHearingWorkflow = &service;
    let command = expected.hearing.review.command.clone();
    assert_eq!(
        workflow
            .prepare("session", f.case(), f.resource(), command.clone())
            .unwrap(),
        expected.hearing.review
    );
    assert_eq!(
        workflow
            .submit(
                "session",
                f.case(),
                f.resource(),
                command,
                expected.origin.submission_digest
            )
            .unwrap(),
        expected
    );
}

#[path = "resource_hearing_read_support/boundaries.rs"]
mod boundaries;
