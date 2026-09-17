#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod procedural_fact_read_support;
#[allow(dead_code)]
mod procedural_fact_service_support;
use application::{procedural_facts::*, ApplicationError};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use procedural_fact_read_support::*;

#[test]
fn denied_roles_and_invalid_sessions_never_read_storage_or_clock() {
    for invalid_session in [false, true] {
        let mut identity = case_support::MockIdentity::new();
        identity.expect_authenticate().times(4).returning(move |_| {
            if invalid_session {
                Err(ApplicationError::InvalidSession)
            } else {
                Ok(application::identity::Principal {
                    id: UserId::new(),
                    email: "client@example.com".into(),
                    role: Role::Client,
                })
            }
        });
        let (service, clock) = service(MockReads::new(), identity);
        let case = CaseId::new();
        let target = FactTarget::Resolution(id(10));
        let results = [
            service
                .list_resolutions(
                    "session",
                    case,
                    ResolutionQuery::new(10, None, FactStatusFilter::All).unwrap(),
                )
                .map(|_| ()),
            service
                .list_notifications(
                    "session",
                    case,
                    id(10),
                    NotificationQuery::new(10, None, FactStatusFilter::All).unwrap(),
                )
                .map(|_| ()),
            service.get("session", case, target, None).map(|_| ()),
            service
                .history(
                    "session",
                    case,
                    target,
                    FactHistoryQuery::new(10, None).unwrap(),
                )
                .map(|_| ()),
        ];
        for result in results {
            assert!(matches!(
                (invalid_session, result),
                (true, Err(ApplicationError::InvalidSession))
                    | (false, Err(ApplicationError::PermissionDenied))
            ));
        }
        assert_eq!(clock.calls(), 0);
    }
}
#[test]
fn all_staff_roles_can_read_empty_pages_without_admission() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let (identity, actor) = case_support::identity(role, 2);
        let case = CaseId::new();
        let mut store = MockReads::new();
        store
            .expect_list_resolutions()
            .times(1)
            .withf(move |user, scope, _, at| {
                *user == actor.id && *scope == case && *at == case_support::instant()
            })
            .returning(|_, _, _, _| {
                Ok(ResolutionPage {
                    resolutions: vec![],
                    has_more: false,
                    next_after_id: None,
                })
            });
        store
            .expect_list_notifications()
            .times(1)
            .withf(move |user, scope, parent, _, _| {
                *user == actor.id && *scope == case && *parent == id(10)
            })
            .returning(|_, _, _, _, _| {
                Ok(NotificationPage {
                    notifications: vec![],
                    has_more: false,
                    next_after_id: None,
                })
            });
        let (service, clock) = service(store, identity);
        assert!(service
            .list_resolutions(
                "session",
                case,
                ResolutionQuery::new(1, None, FactStatusFilter::All).unwrap()
            )
            .unwrap()
            .resolutions
            .is_empty());
        assert!(service
            .list_notifications(
                "session",
                case,
                id(10),
                NotificationQuery::new(1, None, FactStatusFilter::All).unwrap()
            )
            .unwrap()
            .notifications
            .is_empty());
        assert_eq!(clock.calls(), 2);
    }
}
#[test]
fn nil_uuid_and_exact_next_cursors_are_accepted_for_both_lists() {
    let case = CaseId::new();
    let (identity, _) = case_support::identity(Role::Paralegal, 2);
    let mut store = MockReads::new();
    store
        .expect_list_resolutions()
        .returning(move |_, _, _, _| {
            Ok(ResolutionPage {
                resolutions: vec![resolution(case, 0), resolution(case, 1)],
                has_more: true,
                next_after_id: Some(id(1)),
            })
        });
    store
        .expect_list_notifications()
        .returning(move |_, _, _, _, _| {
            Ok(NotificationPage {
                notifications: vec![notification(case, id(10), 0), notification(case, id(10), 1)],
                has_more: true,
                next_after_id: Some(nid(1)),
            })
        });
    let (service, _) = service(store, identity);
    assert_eq!(
        service
            .list_resolutions(
                "session",
                case,
                ResolutionQuery::new(2, None, FactStatusFilter::Recorded).unwrap()
            )
            .unwrap()
            .next_after_id,
        Some(id(1))
    );
    assert_eq!(
        service
            .list_notifications(
                "session",
                case,
                id(10),
                NotificationQuery::new(2, None, FactStatusFilter::Recorded).unwrap()
            )
            .unwrap()
            .next_after_id,
        Some(nid(1))
    );
}
#[test]
fn resolution_pages_reject_scope_status_order_limits_and_cursor_contradictions() {
    let case = CaseId::new();
    for defect in 0..10 {
        let mut page = ResolutionPage {
            resolutions: vec![resolution(case, 2), resolution(case, 3)],
            has_more: true,
            next_after_id: Some(id(3)),
        };
        match defect {
            0 => page.resolutions[0].root = ResolutionRoot::new(id(2), CaseId::new()),
            1 => page.resolutions[0].status = FactStatus::Withdrawn,
            2 => page.resolutions.reverse(),
            3 => page.resolutions[1] = page.resolutions[0].clone(),
            4 => page.resolutions.push(resolution(case, 4)),
            5 => page.resolutions[0] = resolution(case, 1),
            6 => {
                page.resolutions.pop();
            }
            7 => page.next_after_id = None,
            8 => page.next_after_id = Some(id(99)),
            _ => page.has_more = false,
        }
        let mut store = MockReads::new();
        store
            .expect_list_resolutions()
            .returning(move |_, _, _, _| Ok(page.clone()));
        let (identity, _) = case_support::identity(Role::Owner, 1);
        let (service, _) = service(store, identity);
        assert_bad(service.list_resolutions(
            "session",
            case,
            ResolutionQuery::new(2, Some(id(1)), FactStatusFilter::Recorded).unwrap(),
        ));
    }
}
#[test]
fn notification_pages_validate_parent_in_root_and_selected_reference() {
    let case = CaseId::new();
    for defect in 0..12 {
        let mut page = NotificationPage {
            notifications: vec![notification(case, id(10), 2), notification(case, id(10), 3)],
            has_more: true,
            next_after_id: Some(nid(3)),
        };
        match defect {
            0 => page.notifications[0].root = NotificationRoot::new(nid(2), CaseId::new(), id(10)),
            1 => page.notifications[0].root = NotificationRoot::new(nid(2), case, id(99)),
            2 => page.notifications[0].resolution.id = id(99),
            3 => page.notifications[0].status = FactStatus::Withdrawn,
            4 => page.notifications.reverse(),
            5 => page.notifications[1] = page.notifications[0].clone(),
            6 => page.notifications.push(notification(case, id(10), 4)),
            7 => page.notifications[0] = notification(case, id(10), 1),
            8 => {
                page.notifications.pop();
            }
            9 => page.next_after_id = None,
            10 => page.next_after_id = Some(nid(99)),
            _ => page.has_more = false,
        }
        let mut store = MockReads::new();
        store
            .expect_list_notifications()
            .returning(move |_, _, _, _, _| Ok(page.clone()));
        let (identity, _) = case_support::identity(Role::Owner, 1);
        let (service, _) = service(store, identity);
        assert_bad(service.list_notifications(
            "session",
            case,
            id(10),
            NotificationQuery::new(2, Some(nid(1)), FactStatusFilter::Recorded).unwrap(),
        ));
    }
}
#[test]
fn get_keeps_an_exact_revision_in_both_families_and_a_withdrawn_parent() {
    let case = CaseId::new();
    for notice in [false, true] {
        let (identity, actor) = case_support::identity(Role::Paralegal, 1);
        let detail = if notice {
            notification_detail(case, actor.id)
        } else {
            resolution_detail(case, actor.id, 3)
        };
        let expected = detail.clone();
        let target = detail.snapshot.target();
        let revision = detail.snapshot.metadata().revision;
        let mut store = MockReads::new();
        store
            .expect_get()
            .withf(move |user, scope, t, rev, _| {
                *user == actor.id && *scope == case && *t == target && *rev == Some(revision)
            })
            .returning(move |_, _, _, _, _| Ok(detail.clone()));
        let (service, _) = service(store, identity);
        assert_eq!(
            service
                .get("session", case, target, Some(revision))
                .unwrap(),
            expected
        );
    }
}
#[test]
fn get_rejects_foreign_case_target_revision_values_sources_and_receipt() {
    let case = CaseId::new();
    let actor = UserId::new();
    for defect in 0..7 {
        let mut detail = resolution_detail(case, actor, 3);
        let target = detail.snapshot.target();
        match defect {
            0 => detail = resolution_detail(CaseId::new(), actor, 3),
            1 => {
                let s = procedural_fact_service_support::snapshot_mut(&mut detail);
                s.root = ResolutionRoot::new(id(99), case);
            }
            2 => detail = notification_detail(case, actor),
            3 => detail = resolution_detail(case, actor, 2),
            4 => {
                procedural_fact_service_support::snapshot_mut(&mut detail)
                    .metadata
                    .receipt
                    .submission_digest = procedural_fact_service_support::digest(99)
            }
            5 => {
                procedural_fact_service_support::snapshot_mut(&mut detail)
                    .metadata
                    .values_digest = procedural_fact_service_support::digest(99)
            }
            _ => {
                procedural_fact_service_support::snapshot_mut(&mut detail)
                    .metadata
                    .receipt
                    .sources_digest = procedural_fact_service_support::digest(99)
            }
        }
        let mut store = MockReads::new();
        store
            .expect_get()
            .returning(move |_, _, _, _, _| Ok(detail.clone()));
        let (identity, _) = case_support::identity(Role::Owner, 1);
        let (service, _) = service(store, identity);
        assert_bad(service.get("session", case, target, Some(FactRevision::new(3).unwrap())));
    }
}
#[test]
fn get_rejects_another_parent_even_with_same_notification_uuid() {
    let case = CaseId::new();
    let (identity, actor) = case_support::identity(Role::Owner, 1);
    let detail = notification_detail(case, actor.id);
    let target = FactTarget::Notification {
        id: nid(20),
        resolution_id: id(99),
    };
    let mut store = MockReads::new();
    store
        .expect_get()
        .returning(move |_, _, _, _, _| Ok(detail.clone()));
    let (service, _) = service(store, identity);
    assert_bad(service.get("session", case, target, None));
}
#[path = "procedural_fact_read_support/history_tests.rs"]
mod history_tests;
