mod alert_support;
#[allow(dead_code)]
mod case_support;

use alert_support::*;
use application::{alerts::*, identity::Principal, ApplicationError};
use case_support::MockIdentity;
use domain::identity::Role;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

fn workflow(
    auth: Vec<Result<Principal, ApplicationError>>,
    replies: Vec<Reply>,
) -> (AlertService, Arc<Store>, Events) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let store = Arc::new(Store {
        events: events.clone(),
        calls: Mutex::new(vec![]),
        replies: Mutex::new(replies),
    });
    let pending = Mutex::new(VecDeque::from(auth));
    let log = events.clone();
    let mut identity = MockIdentity::new();
    identity
        .expect_authenticate()
        .withf(|token| token == "session")
        .times(0..=4)
        .returning(move |_| {
            log.lock().unwrap().push("auth");
            pending
                .lock()
                .unwrap()
                .pop_front()
                .expect("bounded authentication")
        });
    (
        AlertService::new(store.clone(), Arc::new(identity)),
        store,
        events,
    )
}
fn invoke(service: &AlertService, method: u8) -> Result<(), ApplicationError> {
    match method {
        0 => service.preferences("session").map(|_| ()),
        1 => service
            .save_preferences("session", preference_command())
            .map(|_| ()),
        2 => service.list("session", query()).map(|_| ()),
        3 => service.get("session", id(1)).map(|_| ()),
        _ => service
            .mark_read(
                "session",
                AlertReadCommand {
                    operation_id: operation(21),
                    alert_id: id(1),
                },
            )
            .map(|_| ()),
    }
}
fn reply(actor: &Principal, method: u8) -> Reply {
    match method {
        0 => Reply::Preferences(Ok(AlertPreferences::initial(
            actor.id,
            AlertEmailTransport::Disabled,
        ))),
        1 => Reply::Preferences(Ok(saved(actor.id, &preference_command()))),
        2 => Reply::Page(Ok(page(actor.id))),
        3 => Reply::Detail(Ok(AlertDetail {
            checked_at: at(),
            alert: record(actor.id, 1),
        })),
        _ => {
            let mut alert = record(actor.id, 1);
            alert.read_at = Some(at());
            Reply::Read(Ok(AlertReadReceipt {
                operation_id: operation(21),
                checked_at: at(),
                alert,
            }))
        }
    }
}

#[test]
fn every_personal_operation_authorizes_staff_and_reauthenticates_after_the_store() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        for method in 0..5 {
            let actor = actor(role);
            let (service, store, events) = workflow(
                vec![Ok(actor.clone()), Ok(actor.clone())],
                vec![reply(&actor, method)],
            );
            invoke(&service, method).unwrap();
            assert_eq!(*events.lock().unwrap(), vec!["auth", "store", "auth"]);
            let expected = match method {
                0 => Call::Preferences(actor.id),
                1 => Call::Save(actor.id, preference_command()),
                2 => Call::List(actor.id, query()),
                3 => Call::Get(actor.id, id(1)),
                _ => Call::Read(
                    actor.id,
                    AlertReadCommand {
                        operation_id: operation(21),
                        alert_id: id(1),
                    },
                ),
            };
            assert_eq!(*store.calls.lock().unwrap(), vec![expected]);
        }
    }
}

#[test]
fn client_and_invalid_session_never_reach_the_store() {
    for method in 0..5 {
        for client in [true, false] {
            let auth = if client {
                Ok(actor(Role::Client))
            } else {
                Err(ApplicationError::InvalidSession)
            };
            let (service, store, events) = workflow(vec![auth], vec![]);
            let result = invoke(&service, method);
            if client {
                assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
            } else {
                assert!(matches!(result, Err(ApplicationError::InvalidSession)));
            }
            assert!(store.calls.lock().unwrap().is_empty());
            assert_eq!(*events.lock().unwrap(), vec!["auth"]);
        }
    }
}

#[test]
fn changed_identity_id_email_role_or_revoked_session_prevents_every_response() {
    for method in 0..5 {
        for changed in 0..4 {
            let actor = actor(Role::Owner);
            let mut later = actor.clone();
            match changed {
                0 => later.id = domain::identity::UserId::new(),
                1 => later.email = "changed@example.test".into(),
                2 => later.role = Role::Paralegal,
                _ => (),
            }
            let after = if changed == 3 {
                Err(ApplicationError::InvalidSession)
            } else {
                Ok(later)
            };
            let (service, _, events) =
                workflow(vec![Ok(actor.clone()), after], vec![reply(&actor, method)]);
            assert!(matches!(
                invoke(&service, method),
                Err(ApplicationError::InvalidSession)
            ));
            assert_eq!(*events.lock().unwrap(), vec!["auth", "store", "auth"]);
        }
    }
}

#[test]
fn response_binding_is_checked_before_disclosure_or_a_second_authentication() {
    for method in 0..5 {
        let actor = actor(Role::Owner);
        let foreign = actor_support_foreign(&actor);
        let (service, _, events) = workflow(vec![Ok(actor)], vec![reply(&foreign, method)]);
        assert!(matches!(
            invoke(&service, method),
            Err(ApplicationError::Alert(AlertError::Stored(_)))
        ));
        assert_eq!(*events.lock().unwrap(), vec!["auth", "store"]);
    }
}
fn actor_support_foreign(value: &Principal) -> Principal {
    Principal {
        id: domain::identity::UserId::new(),
        ..value.clone()
    }
}

#[test]
fn preference_replays_preserve_committed_values_and_conflicts_are_not_relabelled() {
    let actor = actor(Role::Litigator);
    let command = preference_command();
    let expected = saved(actor.id, &command);
    let (service, store, _) = workflow(
        (0..4).map(|_| Ok(actor.clone())).collect(),
        vec![
            Reply::Preferences(Ok(expected.clone())),
            Reply::Preferences(Ok(expected.clone())),
        ],
    );
    assert_eq!(
        service
            .save_preferences("session", command.clone())
            .unwrap(),
        expected
    );
    assert_eq!(
        service
            .save_preferences("session", command.clone())
            .unwrap(),
        expected
    );
    assert_eq!(
        *store.calls.lock().unwrap(),
        vec![
            Call::Save(actor.id, command.clone()),
            Call::Save(actor.id, command)
        ]
    );
    for error in [AlertError::RevisionConflict, AlertError::OperationConflict] {
        let operation_conflict = matches!(error, AlertError::OperationConflict);
        let (service, _, events) = workflow(
            vec![Ok(actor.clone())],
            vec![Reply::Preferences(Err(error.into()))],
        );
        let error = service
            .save_preferences("session", preference_command())
            .unwrap_err();
        if operation_conflict {
            assert!(matches!(
                error,
                ApplicationError::Alert(AlertError::OperationConflict)
            ));
        } else {
            assert!(matches!(
                error,
                ApplicationError::Alert(AlertError::RevisionConflict)
            ));
        }
        assert_eq!(*events.lock().unwrap(), vec!["auth", "store"]);
    }
}

#[test]
fn marking_read_preserves_subject_evidence_and_returns_the_idempotent_receipt() {
    let actor = actor(Role::Paralegal);
    let mut alert = record(actor.id, 1);
    alert.read_at = Some(at());
    let expected = AlertReadReceipt {
        operation_id: operation(21),
        checked_at: at(),
        alert,
    };
    let command = AlertReadCommand {
        operation_id: operation(21),
        alert_id: id(1),
    };
    let (service, store, _) = workflow(
        (0..4).map(|_| Ok(actor.clone())).collect(),
        vec![
            Reply::Read(Ok(expected.clone())),
            Reply::Read(Ok(expected.clone())),
        ],
    );
    assert_eq!(service.mark_read("session", command).unwrap(), expected);
    assert_eq!(service.mark_read("session", command).unwrap(), expected);
    assert_eq!(
        *store.calls.lock().unwrap(),
        vec![Call::Read(actor.id, command), Call::Read(actor.id, command)]
    );
    assert!(matches!(
        expected.alert.kind,
        AlertKind::OverdueUnattended { .. }
    ));
    assert_eq!(expected.alert.state, AlertState::Active);
}

#[test]
fn detail_and_read_results_must_bind_the_requested_alert_and_operation() {
    let actor = actor(Role::Owner);
    for invalid in 0..3 {
        let mut alert = record(actor.id, 1);
        alert.read_at = Some(at());
        let mut receipt = AlertReadReceipt {
            operation_id: operation(21),
            checked_at: at(),
            alert,
        };
        match invalid {
            0 => receipt.operation_id = operation(22),
            1 => receipt.alert.id = id(2),
            _ => receipt.alert.read_at = None,
        }
        let (service, _, events) =
            workflow(vec![Ok(actor.clone())], vec![Reply::Read(Ok(receipt))]);
        assert!(matches!(
            invoke(&service, 4),
            Err(ApplicationError::Alert(AlertError::Stored(_)))
        ));
        assert_eq!(*events.lock().unwrap(), vec!["auth", "store"]);
    }
    let (service, _, events) = workflow(
        vec![Ok(actor.clone())],
        vec![Reply::Detail(Ok(AlertDetail {
            checked_at: at(),
            alert: record(actor.id, 2),
        }))],
    );
    assert!(matches!(
        invoke(&service, 3),
        Err(ApplicationError::Alert(AlertError::Stored(_)))
    ));
    assert_eq!(*events.lock().unwrap(), vec!["auth", "store"]);
}
