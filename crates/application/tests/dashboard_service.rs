#[allow(dead_code)]
#[path = "case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;

use application::{dashboard::*, identity::Principal, ApplicationError};
use domain::identity::{Role, UserId};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

type Events = Arc<Mutex<Vec<&'static str>>>;
struct Store {
    events: Events,
    result: Mutex<Option<Result<DashboardSnapshot, ApplicationError>>>,
}
impl DashboardStore for Store {
    fn read(&self, _actor: UserId) -> Result<DashboardSnapshot, ApplicationError> {
        self.events.lock().unwrap().push("store");
        self.result.lock().unwrap().take().unwrap()
    }
}
fn actor(role: Role) -> Principal {
    Principal {
        id: UserId::new(),
        email: "operator@example.test".into(),
        role,
    }
}
fn snapshot(scope: DashboardScope) -> DashboardSnapshot {
    DashboardSnapshot {
        scope,
        checked_at: time::OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap(),
        active_cases: 2,
        pending_contracts: 3,
        deadlines_overdue: 1,
        deadlines_due_48h: 2,
        deadlines_due_7d: 4,
        deadlines_unresolved: 1,
        workload: vec![],
    }
}
fn workflow(
    replies: Vec<Result<Principal, ApplicationError>>,
    result: Result<DashboardSnapshot, ApplicationError>,
) -> (DashboardService, Events) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let log = events.clone();
    let replies = Mutex::new(VecDeque::from(replies));
    let mut identity = case_support::MockIdentity::new();
    identity
        .expect_authenticate()
        .withf(|token| token == "session")
        .times(0..=2)
        .returning(move |_| {
            log.lock().unwrap().push("auth");
            replies.lock().unwrap().pop_front().unwrap()
        });
    let store = Arc::new(Store {
        events: events.clone(),
        result: Mutex::new(Some(result)),
    });
    (DashboardService::new(store, Arc::new(identity)), events)
}
#[test]
fn authorized_scopes_are_preserved_after_final_authentication() {
    for (role, scope) in [
        (Role::Owner, DashboardScope::Office),
        (Role::Litigator, DashboardScope::AssignedCases),
    ] {
        let actor = actor(role);
        let expected = snapshot(scope);
        let (service, events) = workflow(vec![Ok(actor.clone()), Ok(actor)], Ok(expected.clone()));
        assert_eq!(service.read("session").unwrap(), expected);
        assert_eq!(*events.lock().unwrap(), ["auth", "store", "auth"]);
    }
}
#[test]
fn paralegals_clients_and_invalid_sessions_never_reach_the_store() {
    for role in [Role::Paralegal, Role::Client] {
        let (service, events) =
            workflow(vec![Ok(actor(role))], Ok(snapshot(DashboardScope::Office)));
        assert!(matches!(
            service.read("session"),
            Err(ApplicationError::PermissionDenied)
        ));
        assert_eq!(*events.lock().unwrap(), ["auth"]);
    }
    let (service, events) = workflow(
        vec![Err(ApplicationError::InvalidSession)],
        Ok(snapshot(DashboardScope::Office)),
    );
    assert!(matches!(
        service.read("session"),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(*events.lock().unwrap(), ["auth"]);
}
#[test]
fn revocation_and_changed_identity_prevent_disclosure_after_loading() {
    let principal = actor(Role::Owner);
    for change in 0..4 {
        let mut later = principal.clone();
        match change {
            0 => later.id = UserId::new(),
            1 => later.email = "changed@example.test".into(),
            2 => later.role = Role::Litigator,
            _ => (),
        }
        let reply = if change == 3 {
            Err(ApplicationError::InvalidSession)
        } else {
            Ok(later)
        };
        let (service, events) = workflow(
            vec![Ok(principal.clone()), reply],
            Ok(snapshot(DashboardScope::Office)),
        );
        assert!(matches!(
            service.read("session"),
            Err(ApplicationError::InvalidSession)
        ));
        assert_eq!(*events.lock().unwrap(), ["auth", "store", "auth"]);
    }
}
#[test]
fn invalid_scope_windows_or_workload_order_cannot_escape_the_port() {
    let principal = actor(Role::Litigator);
    for change in 0..4 {
        let mut value = snapshot(DashboardScope::AssignedCases);
        match change {
            0 => value.scope = DashboardScope::Office,
            1 => value.deadlines_due_48h = value.deadlines_due_7d + 1,
            2 => {
                value.workload = vec![DashboardWorkload {
                    user_id: principal.id,
                    email: principal.email.clone(),
                    active_cases: value.active_cases + 1,
                }]
            }
            _ => {
                value.workload = vec![
                    DashboardWorkload {
                        user_id: principal.id,
                        email: principal.email.clone(),
                        active_cases: 1
                    };
                    2
                ]
            }
        }
        let (service, _) = workflow(vec![Ok(principal.clone())], Ok(value));
        assert!(matches!(
            service.read("session"),
            Err(ApplicationError::Port(_))
        ));
    }
}
#[test]
fn store_failure_is_preserved_without_a_second_authentication() {
    let (service, events) = workflow(
        vec![Ok(actor(Role::Owner))],
        Err(ApplicationError::Port("offline".into())),
    );
    assert!(
        matches!(service.read("session"), Err(ApplicationError::Port(message)) if message == "offline")
    );
    assert_eq!(*events.lock().unwrap(), ["auth", "store"]);
}
