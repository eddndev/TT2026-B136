use super::*;
use domain::identity::{Role, UserId};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[test]
fn paralegal_and_client_cannot_prepare_or_submit_or_reach_the_store() {
    for role in [Role::Paralegal, Role::Client] {
        for submit in [false, true] {
            let mut fixture = Fixture::single();
            let expected = confirmation(&fixture.review());
            fixture.actor.role = role;
            let harness = harness(MockStore::new(), identity(fixture.actor));
            let result = if submit {
                harness
                    .service
                    .submit("session", fixture.case_id, fixture.command, expected)
                    .map(|_| ())
            } else {
                harness
                    .service
                    .prepare("session", fixture.case_id, fixture.command)
                    .map(|_| ())
            };
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
            assert!(harness.events().is_empty());
            assert_eq!(harness.validator.calls(), 0);
        }
    }
}

fn different(actor: &Principal, field: u8) -> Principal {
    let mut changed = actor.clone();
    match field {
        0 => changed.id = UserId::from_uuid(Uuid::from_u128(700)),
        1 => changed.email = "changed@example.test".into(),
        _ => changed.role = Role::Owner,
    }
    changed
}

#[test]
fn a_changed_current_id_email_or_role_blocks_prepared_review_disclosure() {
    for field in 0..3 {
        let fixture = Fixture::single();
        let mut identity = crate::case_support::MockIdentity::new();
        let mut sequence = mockall::Sequence::new();
        let actor = fixture.actor.clone();
        let changed = different(&actor, field);
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(actor));
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(changed));
        let harness = harness(fixture.store(), identity);
        assert!(matches!(
            harness
                .service
                .prepare("session", fixture.case_id, fixture.command),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

#[test]
fn expired_session_before_lookup_has_no_store_or_crypto_side_effects() {
    let fixture = Fixture::single();
    let mut identity = crate::case_support::MockIdentity::new();
    identity
        .expect_authenticate()
        .times(1)
        .return_once(|_| Err(ApplicationError::InvalidSession));
    let harness = harness(MockStore::new(), identity);
    assert!(matches!(
        harness
            .service
            .prepare("expired", fixture.case_id, fixture.command),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(harness.events().is_empty());
}

#[test]
fn session_expiry_after_support_admission_prevents_commit() {
    let fixture = Fixture::single();
    let expected = confirmation(&fixture.review());
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
    let harness = harness(fixture.store(), identity);
    assert!(matches!(
        harness
            .service
            .submit("session", fixture.case_id, fixture.command, expected),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(harness.validator.calls(), 1);
}

#[test]
fn principal_changes_after_commit_prevent_disclosure_of_the_committed_group() {
    for field in 0..3 {
        let fixture = Fixture::single();
        let expected = confirmation(&fixture.review());
        let committed = Arc::new(AtomicBool::new(false));
        let mut store = fixture.store();
        let did_commit = committed.clone();
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, prepared| {
                let operation = prepared.into_operation(now())?;
                did_commit.store(true, Ordering::SeqCst);
                Ok(operation)
            });
        let actor = fixture.actor.clone();
        let changed = different(&actor, field);
        let observed = committed.clone();
        let mut identity = crate::case_support::MockIdentity::new();
        identity.expect_authenticate().returning(move |_| {
            Ok(if observed.load(Ordering::SeqCst) {
                changed.clone()
            } else {
                actor.clone()
            })
        });
        let harness = harness(store, identity);
        assert!(matches!(
            harness
                .service
                .submit("session", fixture.case_id, fixture.command, expected),
            Err(ApplicationError::InvalidSession)
        ));
        assert!(committed.load(Ordering::SeqCst));
    }
}

#[test]
fn current_case_access_denial_is_preserved_before_crypto() {
    let fixture = Fixture::single();
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(|_, _, _, _| Err(ApplicationError::PermissionDenied));
    let harness = harness(store, identity(fixture.actor));
    assert!(matches!(
        harness
            .service
            .prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(harness.events().is_empty());
}

#[test]
fn a_commit_failure_is_preserved_without_retry() {
    let fixture = Fixture::single();
    let expected = confirmation(&fixture.review());
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, _| Err(ApplicationError::PermissionDenied));
    let harness = harness(store, identity(fixture.actor));
    assert!(matches!(
        harness
            .service
            .submit("session", fixture.case_id, fixture.command, expected),
        Err(ApplicationError::PermissionDenied)
    ));
}
