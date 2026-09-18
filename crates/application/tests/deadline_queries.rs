#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_service_support;
mod deadline_support;
use application::{deadlines::*, ApplicationError};
use deadline_service_support::*;
use domain::{cases::CaseId, crypto::Sha256Digest, identity::Role};
use uuid::Uuid;

#[test]
fn query_limits_cursors_and_status_are_explicit() {
    for limit in [0, 101, u32::MAX] {
        assert!(DeadlineQuery::new(limit, None, DeadlineStatusFilter::All).is_err());
    }
    for limit in [0, 21, u32::MAX] {
        assert!(DeadlineHistoryQuery::new(limit, None).is_err());
    }
    assert!(DeadlineHistoryQuery::new(1, Some(0)).is_err());
    let zero = DeadlineId::from_uuid(Uuid::nil());
    let query = DeadlineQuery::new(100, Some(zero), DeadlineStatusFilter::Retired).unwrap();
    assert_eq!(query.limit(), 100);
    assert_eq!(query.after_id(), Some(zero));
    assert_eq!(query.status().status(), Some(DeadlineStatus::Retired));
    let history = DeadlineHistoryQuery::new(20, Some(u32::MAX)).unwrap();
    assert_eq!(history.before_revision().unwrap().get(), u32::MAX);
}

#[test]
fn all_staff_read_valid_exact_detail_and_history_without_recalculation() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        for operation in 0..3 {
            let mut store = MockStore::new();
            expect_operation(&mut store, operation);
            let (workflow, clock) = service(store, identity(role, 2));
            run(&workflow, operation).unwrap();
            assert_eq!(clock.calls(), 1);
        }
    }
    let first = captured();
    let second = attention_revision(&first);
    let retired = retirement_revision(&second);
    let expected = retired.clone();
    let mut store = MockStore::new();
    store
        .expect_get()
        .times(1)
        .return_once(move |_, _, _, _, _| Ok(retired));
    let (workflow, _) = service(store, identity(Role::Paralegal, 2));
    assert_eq!(
        workflow
            .get("session", case_id(), expected.id, Some(expected.revision))
            .unwrap(),
        expected
    );
}

#[test]
fn exact_get_rejects_wrong_case_root_revision_or_damaged_receipt() {
    for mutation in 0..4 {
        let mut row = captured();
        let id = row.id;
        let revision = row.revision;
        match mutation {
            0 => row.case_id = CaseId::new(),
            1 => row.id = DeadlineId::new(),
            2 => row = attention_revision(&row),
            _ => row.receipt.submission_digest = Sha256Digest::from_array([0; 32]),
        }
        let mut store = MockStore::new();
        store
            .expect_get()
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(row));
        let (workflow, _) = service(store, identity(Role::Owner, 1));
        assert!(workflow
            .get("session", case_id(), id, Some(revision))
            .is_err());
    }
}

#[test]
fn summary_matches_detail_and_keeps_nil_id_on_an_initial_page() {
    let detail = captured();
    let mut row = DeadlineOverview::from(&detail);
    assert_eq!(row.title, detail.definition.title);
    assert_eq!(row.responsible, detail.responsible);
    assert!(detail.calculation.result.due_at().is_some());
    assert_eq!(row.due_at, None);
    assert!(row.blocked);
    assert_eq!(
        row.review_state,
        application::deadline_tracking::DeadlineReviewState::LegacyUndeclared
    );
    assert!(!row.attention_recorded);
    row.id = DeadlineId::from_uuid(Uuid::nil());
    let expected = row.clone();
    let mut store = MockStore::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, query, _| {
            assert_eq!(query.after_id(), None);
            Ok(DeadlinePage {
                deadlines: vec![row],
                has_more: true,
                next_after_id: Some(DeadlineId::from_uuid(Uuid::nil())),
            })
        });
    let (workflow, _) = service(store, identity(Role::Paralegal, 2));
    assert_eq!(
        workflow
            .list("session", case_id(), list_query(1))
            .unwrap()
            .deadlines,
        vec![expected]
    );
}

#[test]
fn lists_reject_cross_case_wrong_filter_order_cursor_and_contradictory_outcome() {
    for mutation in 0..10 {
        let mut row = DeadlineOverview::from(&captured());
        let id = row.id;
        let mut rows = vec![row.clone()];
        let mut has_more = false;
        let mut next = None;
        let mut query = DeadlineQuery::new(2, None, DeadlineStatusFilter::Active).unwrap();
        match mutation {
            0 => rows[0].case_id = CaseId::new(),
            1 => rows[0].status = DeadlineStatus::Retired,
            2 => rows.push(row.clone()),
            3 => {
                row.id = DeadlineId::from_uuid(Uuid::nil());
                rows.push(row);
            }
            4 => {
                has_more = true;
                next = Some(id);
            }
            5 => next = Some(id),
            6 => {
                has_more = true;
                next = Some(DeadlineId::new());
                query = list_query(1);
            }
            7 => query = DeadlineQuery::new(2, Some(id), DeadlineStatusFilter::All).unwrap(),
            8 => rows[0].blocked = false,
            _ => rows[0].responsible.role = Role::Client,
        }
        let mut store = MockStore::new();
        store.expect_list().times(1).return_once(move |_, _, _, _| {
            Ok(DeadlinePage {
                deadlines: rows,
                has_more,
                next_after_id: next,
            })
        });
        let (workflow, _) = service(store, identity(Role::Owner, 1));
        invalid(workflow.list("session", case_id(), query).map(|_| ()));
    }
}

#[test]
fn history_is_lightweight_descending_consecutive_and_ends_at_r1() {
    let first = captured();
    let second = attention_revision(&first);
    let third = retirement_revision(&second);
    let all: Vec<_> = [&third, &second, &first]
        .into_iter()
        .map(history_entry)
        .collect();
    let expected = all.clone();
    let id = first.id;
    let mut store = MockStore::new();
    store
        .expect_history()
        .times(1)
        .return_once(move |_, _, _, _, _| {
            Ok(DeadlineHistoryPage {
                revisions: all,
                has_more: false,
                next_before_revision: None,
            })
        });
    let (workflow, _) = service(store, identity(Role::Litigator, 2));
    assert_eq!(
        workflow
            .history("session", case_id(), id, history_query(20))
            .unwrap()
            .revisions,
        expected
    );
}

#[test]
fn history_rejects_forged_state_digest_scope_gaps_bad_cursor_and_terminal_successors() {
    let first = captured();
    let second = attention_revision(&first);
    let third = retirement_revision(&second);
    for mutation in 0..10 {
        let mut rows: Vec<_> = [&third, &second, &first]
            .into_iter()
            .map(history_entry)
            .collect();
        let mut has_more = false;
        let mut next = None;
        let mut query = history_query(20);
        match mutation {
            0 => rows[0].state_digest = Sha256Digest::from_array([0; 32]),
            1 => rows[0].case_id = CaseId::new(),
            2 => rows[0].id = DeadlineId::new(),
            3 => {
                rows.remove(1);
            }
            4 => rows.reverse(),
            5 => next = Some(first.revision),
            6 => {
                has_more = true;
                next = Some(first.revision);
                query = history_query(3);
            }
            7 => {
                rows.pop();
            }
            8 => query = DeadlineHistoryQuery::new(20, Some(third.revision.get())).unwrap(),
            _ => {
                rows[1].status = DeadlineStatus::Retired;
            }
        }
        let mut store = MockStore::new();
        store
            .expect_history()
            .times(1)
            .return_once(move |_, _, _, _, _| {
                Ok(DeadlineHistoryPage {
                    revisions: rows,
                    has_more,
                    next_before_revision: next,
                })
            });
        let (workflow, _) = service(store, identity(Role::Owner, 1));
        assert!(workflow
            .history("session", case_id(), first.id, query)
            .is_err());
    }
}

#[test]
fn empty_case_queries_still_reach_the_authorized_store() {
    for operation in [0, 2] {
        let mut store = MockStore::new();
        if operation == 0 {
            store.expect_list().times(1).returning(|_, case, _, _| {
                assert_eq!(case, case_id());
                Err(ApplicationError::CaseNotFound)
            });
        } else {
            store
                .expect_history()
                .times(1)
                .returning(|_, case, _, _, _| {
                    assert_eq!(case, case_id());
                    Err(ApplicationError::CaseNotFound)
                });
        }
        let (workflow, _) = service(store, identity(Role::Litigator, 1));
        assert!(matches!(
            run(&workflow, operation),
            Err(ApplicationError::CaseNotFound)
        ));
    }
}

#[test]
fn summary_without_due_instant_must_report_a_block() {
    let mut row = DeadlineOverview::from(&captured());
    row.due_at = None;
    row.blocked = false;
    let mut store = MockStore::new();
    store.expect_list().times(1).return_once(move |_, _, _, _| {
        Ok(DeadlinePage {
            deadlines: vec![row],
            has_more: false,
            next_after_id: None,
        })
    });
    let mut session = MockIdentity::new();
    session
        .expect_authenticate()
        .returning(|_| Ok(principal(Role::Owner)));
    let (workflow, _) = service(store, session);
    invalid(
        workflow
            .list("session", case_id(), list_query(20))
            .map(|_| ()),
    );
}

#[test]
fn empty_history_requires_the_exclusive_first_revision_bound() {
    for before in [None, Some(2), Some(u32::MAX), Some(1)] {
        let mut store = MockStore::new();
        store.expect_history().times(1).returning(|_, _, _, _, _| {
            Ok(DeadlineHistoryPage {
                revisions: vec![],
                has_more: false,
                next_before_revision: None,
            })
        });
        let mut session = MockIdentity::new();
        session
            .expect_authenticate()
            .returning(|_| Ok(principal(Role::Owner)));
        let (workflow, _) = service(store, session);
        let result = workflow.history(
            "session",
            case_id(),
            captured().id,
            DeadlineHistoryQuery::new(20, before).unwrap(),
        );
        if before == Some(1) {
            assert!(result.unwrap().revisions.is_empty());
        } else {
            invalid(result.map(|_| ()));
        }
    }
}
