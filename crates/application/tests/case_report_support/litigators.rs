use super::case_report_support::*;
use application::{case_reports::*, ApplicationError};
use domain::{
    clock::Clock,
    identity::{Role, UserId},
};
use mockall::Sequence;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn id(value: u128) -> UserId {
    UserId::from_uuid(uuid::Uuid::from_u128(value))
}
fn query() -> CaseReportLitigatorQuery {
    CaseReportLitigatorQuery {
        kind: CaseReportKind::CaseState,
        limit: 2,
        after_id: Some(id(1)),
    }
}
fn page(who: &application::identity::Principal) -> CaseReportLitigatorPage {
    CaseReportLitigatorPage {
        scope: scope(who),
        checked_at: now(),
        litigators: vec![
            CaseReportLitigator {
                user_id: id(2),
                email: "closed-colleague@example.test".into(),
            },
            CaseReportLitigator {
                user_id: id(3),
                email: "other-colleague@example.test".into(),
            },
        ],
        has_more: true,
        next_after_id: Some(id(3)),
    }
}

#[test]
fn litigator_picker_preserves_authorized_scope_cursor_and_reauthenticates() {
    for role in [Role::Owner, Role::Litigator] {
        let who = actor(role);
        let expected_actor = who.clone();
        let expected = page(&who);
        let returned = expected.clone();
        let mut store = MockStore::new();
        store
            .expect_litigators()
            .times(1)
            .withf(move |actor, input, at| {
                actor == &expected_actor && *input == query() && *at == now()
            })
            .returning(move |_, _, _| Ok(returned.clone()));
        assert_eq!(
            service(store, identity(&who, 2))
                .litigators("session", query())
                .unwrap(),
            expected
        );
    }
}

#[test]
fn litigator_picker_denies_other_roles_without_reading_an_empty_directory() {
    for role in [Role::Client, Role::Paralegal] {
        let who = actor(role);
        assert!(matches!(
            service(MockStore::new(), identity(&who, 1)).litigators("session", query()),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn litigator_picker_invalid_limits_and_nil_cursor_never_reach_store() {
    let who = actor(Role::Owner);
    for query in [
        CaseReportLitigatorQuery {
            kind: CaseReportKind::CaseState,
            limit: 0,
            after_id: None,
        },
        CaseReportLitigatorQuery {
            kind: CaseReportKind::CaseState,
            limit: 101,
            after_id: None,
        },
        CaseReportLitigatorQuery {
            kind: CaseReportKind::CaseState,
            limit: 2,
            after_id: Some(id(0)),
        },
    ] {
        assert!(matches!(
            service(MockStore::new(), identity(&who, 1)).litigators("session", query),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
}

#[test]
fn litigator_picker_rejects_forged_scope_time_order_identity_and_continuation() {
    let who = actor(Role::Owner);
    for mutation in 0..13 {
        let mut returned = page(&who);
        match mutation {
            0 => returned.scope = CaseReportScope::AssignedCases,
            1 => returned.checked_at -= time::Duration::seconds(1),
            2 => returned.checked_at += time::Duration::seconds(1),
            3 => returned.litigators[1].user_id = id(2),
            4 => returned.litigators.swap(0, 1),
            5 => returned.litigators[0].user_id = id(1),
            6 => returned.litigators[0].user_id = id(0),
            7 => returned.litigators[0].email = "NONCANONICAL@example.test".into(),
            8 => returned.litigators.push(CaseReportLitigator {
                user_id: id(4),
                email: "extra@example.test".into(),
            }),
            9 => returned.next_after_id = Some(id(4)),
            10 => returned.has_more = false,
            11 => {
                returned.litigators.pop();
                returned.next_after_id = Some(id(2));
            }
            _ => {
                returned.checked_at = returned
                    .checked_at
                    .to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap())
            }
        }
        let mut store = MockStore::new();
        store
            .expect_litigators()
            .times(1)
            .returning(move |_, _, _| Ok(returned.clone()));
        inconsistent(service(store, identity(&who, 1)).litigators("session", query()));
    }
}

#[test]
fn litigator_picker_rechecks_full_principal_after_loading() {
    let who = actor(Role::Owner);
    for mutation in 0..4 {
        let returned = page(&who);
        let mut store = MockStore::new();
        store
            .expect_litigators()
            .times(1)
            .returning(move |_, _, _| Ok(returned.clone()));
        let mut auth = super::case_support::MockIdentity::new();
        let mut sequence = Sequence::new();
        let first = who.clone();
        auth.expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(first));
        let mut changed = who.clone();
        match mutation {
            0 => changed.id = id(20),
            1 => changed.email = "changed@example.test".into(),
            2 => changed.role = Role::Litigator,
            _ => (),
        }
        auth.expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                if mutation == 3 {
                    Err(ApplicationError::InvalidSession)
                } else {
                    Ok(changed)
                }
            });
        assert!(matches!(
            service(store, auth).litigators("session", query()),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

struct BackwardClock(AtomicUsize);
impl Clock for BackwardClock {
    fn now(&self) -> time::OffsetDateTime {
        now() - time::Duration::seconds(self.0.fetch_add(1, Ordering::SeqCst) as i64)
    }
}
#[test]
fn litigator_picker_rejects_backward_clock_after_store_read() {
    let who = actor(Role::Owner);
    let returned = page(&who);
    let mut store = MockStore::new();
    store
        .expect_litigators()
        .times(1)
        .returning(move |_, _, _| Ok(returned.clone()));
    let workflow = CaseReportService::new(
        Arc::new(store),
        Arc::new(identity(&who, 1)),
        Arc::new(TestHasher),
        Arc::new(BackwardClock(AtomicUsize::new(0))),
    );
    inconsistent(workflow.litigators("session", query()));
}
