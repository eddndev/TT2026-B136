mod agenda_support;
#[allow(dead_code)]
mod case_support;

use agenda_support::*;
use application::{
    agenda::*, hearings::HearingStatusFilter, identity::Principal, ApplicationError,
};
use case_support::MockIdentity;
use domain::identity::{Role, UserId};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

type Events = Arc<Mutex<Vec<&'static str>>>;
struct Store {
    events: Events,
    requests: Mutex<Vec<(UserId, AgendaQuery)>>,
    result: Mutex<Option<Result<AgendaPage, ApplicationError>>>,
}
impl AgendaStore for Store {
    fn list(&self, actor: UserId, query: AgendaQuery) -> Result<AgendaPage, ApplicationError> {
        self.events.lock().unwrap().push("store");
        self.requests.lock().unwrap().push((actor, query));
        self.result.lock().unwrap().take().expect("one read")
    }
}
fn principal(role: Role) -> Principal {
    Principal {
        id: UserId::new(),
        email: "operator@example.test".into(),
        role,
    }
}
fn workflow(
    replies: Vec<Result<Principal, ApplicationError>>,
    result: Result<AgendaPage, ApplicationError>,
) -> (AgendaService, Arc<Store>, Events) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let store = Arc::new(Store {
        events: events.clone(),
        requests: Mutex::new(Vec::new()),
        result: Mutex::new(Some(result)),
    });
    let log = events.clone();
    let replies = Mutex::new(VecDeque::from(replies));
    let mut identity = MockIdentity::new();
    identity
        .expect_authenticate()
        .withf(|token| token == "session")
        .times(0..=2)
        .returning(move |_| {
            log.lock().unwrap().push("auth");
            replies
                .lock()
                .unwrap()
                .pop_front()
                .expect("bounded authentication")
        });
    (
        AgendaService::new(store.clone(), Arc::new(identity)),
        store,
        events,
    )
}

#[test]
fn staff_read_the_selected_families_and_preserve_empty_continuation_under_reauthentication() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        for kind in [AgendaKind::All, AgendaKind::Hearing, AgendaKind::Deadline] {
            let actor = principal(role);
            let q = AgendaQuery::new(
                1,
                from(),
                until(),
                kind,
                HearingStatusFilter::Scheduled,
                None,
            )
            .unwrap();
            let after = AgendaCursor::new(
                from(),
                if kind == AgendaKind::Deadline {
                    AgendaItemKind::Deadline
                } else {
                    AgendaItemKind::Hearing
                },
                uuid::Uuid::nil(),
            )
            .unwrap();
            let expected = AgendaPage {
                checked_at: checked_at(),
                items: vec![],
                complete: false,
                next_after: Some(after),
            };
            let (service, store, events) = workflow(
                vec![Ok(actor.clone()), Ok(actor.clone())],
                Ok(expected.clone()),
            );
            assert_eq!(service.list("session", q).unwrap(), expected);
            assert_eq!(*store.requests.lock().unwrap(), vec![(actor.id, q)]);
            assert_eq!(*events.lock().unwrap(), vec!["auth", "store", "auth"]);
        }
    }
}

#[test]
fn clients_and_invalid_sessions_are_denied_before_the_store_for_every_family() {
    for kind in [AgendaKind::All, AgendaKind::Hearing, AgendaKind::Deadline] {
        let q = AgendaQuery::new(
            1,
            from(),
            until(),
            kind,
            HearingStatusFilter::Scheduled,
            None,
        )
        .unwrap();
        let (service, store, events) =
            workflow(vec![Ok(principal(Role::Client))], Ok(page(vec![])));
        assert!(matches!(
            service.list("session", q),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(store.requests.lock().unwrap().is_empty());
        assert_eq!(*events.lock().unwrap(), vec!["auth"]);
    }
    let (service, store, events) = workflow(
        vec![Err(ApplicationError::InvalidSession)],
        Ok(page(vec![])),
    );
    assert!(matches!(
        service.list("session", query(1)),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(store.requests.lock().unwrap().is_empty());
    assert_eq!(*events.lock().unwrap(), vec!["auth"]);
}

#[test]
fn a_revoked_session_or_any_changed_principal_field_prevents_disclosure() {
    let actor = principal(Role::Owner);
    for mutation in 0..4 {
        let mut later = actor.clone();
        match mutation {
            0 => later.id = UserId::new(),
            1 => later.email = "changed@example.test".into(),
            2 => later.role = Role::Paralegal,
            _ => (),
        }
        let reply = if mutation == 3 {
            Err(ApplicationError::InvalidSession)
        } else {
            Ok(later)
        };
        let (service, _, events) =
            workflow(vec![Ok(actor.clone()), reply], Ok(page(vec![item(1)])));
        assert!(matches!(
            service.list("session", query(1)),
            Err(ApplicationError::InvalidSession)
        ));
        assert_eq!(*events.lock().unwrap(), vec!["auth", "store", "auth"]);
    }
}

#[test]
fn inconsistent_pages_are_rejected_and_store_failures_are_preserved() {
    let actor = principal(Role::Owner);
    let (service, _, events) = workflow(vec![Ok(actor.clone())], Ok(page(vec![item(2), item(1)])));
    assert!(matches!(
        service.list("session", query(2)),
        Err(ApplicationError::Port(_))
    ));
    assert_eq!(*events.lock().unwrap(), vec!["auth", "store"]);
    let (service, _, events) = workflow(
        vec![Ok(actor)],
        Err(ApplicationError::Port("offline".into())),
    );
    assert!(
        matches!(service.list("session", query(1)), Err(ApplicationError::Port(message)) if message == "offline")
    );
    assert_eq!(*events.lock().unwrap(), vec!["auth", "store"]);
}
