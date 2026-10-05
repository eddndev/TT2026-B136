use super::*;
use application::identity::Principal;
use domain::identity::{Role, UserId};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[test]
fn owner_and_litigator_prepare_under_their_full_current_principal() {
    for role in [Role::Owner, Role::Litigator] {
        let mut fixture = Fixture::schedule();
        fixture.actor.role = role;
        let expected = fixture.review();
        let h = harness(fixture.store(), identity(fixture.actor));
        assert_eq!(
            h.service
                .prepare("session", fixture.case_id, fixture.command)
                .unwrap(),
            expected
        );
    }
}

#[test]
fn read_only_roles_and_invalid_sessions_are_refused_before_store_or_admission() {
    for role in [Role::Paralegal, Role::Client] {
        for submit in [false, true] {
            let mut fixture = Fixture::schedule();
            let expected = confirmation(&fixture.review());
            fixture.actor.role = role;
            let h = harness(MockStore::new(), identity(fixture.actor));
            let result = if submit {
                h.service
                    .submit("session", fixture.case_id, fixture.command, expected)
                    .map(|_| ())
            } else {
                h.service
                    .prepare("session", fixture.case_id, fixture.command)
                    .map(|_| ())
            };
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
            assert!(h.events().is_empty());
            assert_eq!(h.validator.calls(), 0);
        }
    }
    let fixture = Fixture::schedule();
    let mut identity = crate::case_support::MockIdentity::new();
    identity
        .expect_authenticate()
        .times(1)
        .return_once(|_| Err(ApplicationError::InvalidSession));
    let h = harness(MockStore::new(), identity);
    assert!(matches!(
        h.service
            .prepare("expired", fixture.case_id, fixture.command),
        Err(ApplicationError::InvalidSession)
    ));
}

fn changed(actor: &Principal, field: u8) -> Principal {
    let mut result = actor.clone();
    match field {
        0 => result.id = UserId::from_uuid(Uuid::from_u128(600)),
        1 => result.email = "changed@example.test".into(),
        _ => result.role = Role::Owner,
    }
    result
}

#[test]
fn every_principal_field_is_reauthenticated_before_review_or_commit() {
    for field in 0..3 {
        for submit in [false, true] {
            let fixture = Fixture::schedule();
            let expected = confirmation(&fixture.review());
            let mut identity = crate::case_support::MockIdentity::new();
            let mut sequence = mockall::Sequence::new();
            let actor = fixture.actor.clone();
            let other = changed(&actor, field);
            identity
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| Ok(actor));
            identity
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| Ok(other));
            let h = harness(fixture.store(), identity);
            let result = if submit {
                h.service
                    .submit("session", fixture.case_id, fixture.command, expected)
                    .map(|_| ())
            } else {
                h.service
                    .prepare("session", fixture.case_id, fixture.command)
                    .map(|_| ())
            };
            assert!(matches!(result, Err(ApplicationError::InvalidSession)));
            assert_eq!(h.validator.calls(), 1);
        }
    }
}

#[test]
fn changed_current_principal_after_commit_prevents_disclosure() {
    let fixture = Fixture::schedule();
    let expected = confirmation(&fixture.review());
    let committed = Arc::new(AtomicBool::new(false));
    let did_commit = committed.clone();
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, prepared| {
            let result = prepared.into_operation(now())?;
            did_commit.store(true, Ordering::SeqCst);
            Ok(result)
        });
    let mut identity = crate::case_support::MockIdentity::new();
    let actor = fixture.actor;
    let changed = changed(&actor, 1);
    let observed = committed.clone();
    identity.expect_authenticate().returning(move |_| {
        Ok(if observed.load(Ordering::SeqCst) {
            changed.clone()
        } else {
            actor.clone()
        })
    });
    let h = harness(store, identity);
    assert!(matches!(
        h.service
            .submit("session", fixture.case_id, fixture.command, expected),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(committed.load(Ordering::SeqCst));
}

#[test]
fn store_access_and_head_or_source_refusals_are_preserved_without_retry() {
    let fixture = Fixture::schedule();
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(|_, _, _, _| Err(ApplicationError::PermissionDenied));
    let h = harness(store, identity(fixture.actor.clone()));
    assert!(matches!(
        h.service
            .prepare("session", fixture.case_id, fixture.command.clone()),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(h.events().is_empty());
    let expected = confirmation(&fixture.review());
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, _| Err(PrecautionaryHearingError::OperationConflict.into()));
    let h = harness(store, identity(fixture.actor));
    assert!(matches!(
        h.service
            .submit("session", fixture.case_id, fixture.command, expected),
        Err(ApplicationError::PrecautionaryHearing(
            PrecautionaryHearingError::OperationConflict
        ))
    ));
    assert_eq!(h.validator.calls(), 1);
}

#[test]
fn replay_reauthenticates_before_disclosing_historical_evidence() {
    let fixture = Fixture::schedule();
    let original = fixture.operation(at());
    let mut identity = crate::case_support::MockIdentity::new();
    let mut sequence = mockall::Sequence::new();
    let actor = fixture.actor.clone();
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(actor));
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(|_| Err(ApplicationError::InvalidSession));
    let h = harness(fixture.replay_store(original), identity);
    assert!(matches!(
        h.service
            .prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(h.events().is_empty());
}
