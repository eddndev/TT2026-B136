use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

fn changed(actor: &Principal, field: u8) -> Principal {
    let mut value = actor.clone();
    match field {
        0 => value.id = UserId::from_uuid(Uuid::from_u128(700)),
        1 => value.email = "changed-current@example.test".into(),
        _ => value.role = Role::Owner,
    }
    value
}

#[test]
fn record_workflow_denies_nonrecording_roles_before_store_and_admission() {
    for role in [Role::Paralegal, Role::Client] {
        for submitting in [false, true] {
            let fixture = Fixture::corrected();
            let expected = confirmation(&fixture.review());
            let mut actor = fixture.actor;
            actor.role = role;
            let harness = harness(MockStore::new(), identity(actor));
            let result = if submitting {
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
    let mut owner = Fixture::corrected();
    owner.actor.role = Role::Owner;
    assert_eq!(submit(owner).group.review.actor.role, Role::Owner);
}

#[test]
fn changed_current_id_email_or_role_prevents_review_and_precommit_disclosure() {
    for field in 0..3 {
        for submitting in [false, true] {
            let fixture = Fixture::corrected();
            let expected = confirmation(&fixture.review());
            let actor = fixture.actor.clone();
            let different = changed(&actor, field);
            let mut current = crate::case_support::MockIdentity::new();
            let mut sequence = mockall::Sequence::new();
            current
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| Ok(actor));
            current
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| Ok(different));
            let harness = harness(fixture.store(), current);
            let result = if submitting {
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
            assert!(matches!(result, Err(ApplicationError::InvalidSession)));
            assert_eq!(harness.validator.calls(), 1);
        }
    }
}

#[test]
fn changed_full_principal_after_commit_prevents_returning_the_real_group() {
    for field in 0..3 {
        let fixture = Fixture::corrected();
        let expected = confirmation(&fixture.review());
        let committed = Arc::new(AtomicBool::new(false));
        let observed = committed.clone();
        let mut store = fixture.store();
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, prepared| {
                let operation = prepared.into_operation(now())?;
                observed.store(true, Ordering::SeqCst);
                Ok(operation)
            });
        let actor = fixture.actor.clone();
        let different = changed(&actor, field);
        let observed = committed.clone();
        let mut current = crate::case_support::MockIdentity::new();
        current.expect_authenticate().returning(move |_| {
            Ok(if observed.load(Ordering::SeqCst) {
                different.clone()
            } else {
                actor.clone()
            })
        });
        let harness = harness(store, current);
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
fn expired_session_and_store_access_denial_do_not_reach_support_admission() {
    let fixture = Fixture::corrected();
    let mut current = crate::case_support::MockIdentity::new();
    current
        .expect_authenticate()
        .times(1)
        .return_once(|_| Err(ApplicationError::InvalidSession));
    let expired = harness(MockStore::new(), current);
    assert!(matches!(
        expired
            .service
            .prepare("expired", fixture.case_id, fixture.command.clone()),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(expired.events().is_empty());
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(|_, _, _, _| Err(ApplicationError::PermissionDenied));
    let denied = harness(store, identity(fixture.actor));
    assert!(matches!(
        denied
            .service
            .prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(denied.events().is_empty());
}
