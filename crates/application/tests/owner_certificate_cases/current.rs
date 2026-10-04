use application::{identity::owner_certificates::OwnerCertificateError, ApplicationError};
use domain::identity::{Role, UserId};
use uuid::Uuid;

use crate::support::*;

#[test]
fn current_receipt_returns_own_unwithdrawn_history_or_none_without_current_trust() {
    for found in [None, Some(receipt())] {
        let mut harness = Harness::new();
        let observed = harness.state.clone();
        {
            let mut state = observed.lock().unwrap();
            state.now = 4000;
            state.context.trust = None;
        }
        let expected = found.clone();
        let calls = observed.clone();
        harness
            .store
            .expect_find_current()
            .times(1)
            .returning(move |actor| {
                assert_eq!(actor, principal().id);
                calls.lock().unwrap().calls.push("find_current");
                Ok(found.clone())
            });
        assert_eq!(
            harness.service().current_receipt("session").unwrap(),
            expected
        );
        assert_eq!(
            observed.lock().unwrap().calls,
            ["authenticate", "find_current", "authenticate"]
        );
    }
}

#[test]
fn current_receipt_rejects_unauthorized_principals_before_querying_the_store() {
    for role in [
        None,
        Some(Role::Litigator),
        Some(Role::Paralegal),
        Some(Role::Client),
    ] {
        let harness = Harness::new();
        let observed = harness.state.clone();
        observed.lock().unwrap().principal = role.map(|role| application::identity::Principal {
            role,
            ..principal()
        });
        let error = failure(harness.service().current_receipt("session"));
        match role {
            None => assert!(matches!(error, ApplicationError::InvalidSession)),
            Some(_) => assert!(matches!(error, ApplicationError::PermissionDenied)),
        }
        assert_eq!(observed.lock().unwrap().calls, ["authenticate"]);
    }
}

#[test]
fn current_receipt_rejects_foreign_terminal_and_inconsistent_evidence() {
    for variant in 0..3 {
        let mut value = receipt();
        match variant {
            0 => value.owner = UserId::from_uuid(Uuid::from_u128(99)),
            1 => value = retired(),
            _ => value.check.certificate.der.push(0),
        }
        let mut harness = Harness::new();
        let observed = harness.state.clone();
        let calls = observed.clone();
        harness
            .store
            .expect_find_current()
            .times(1)
            .returning(move |actor| {
                assert_eq!(actor, principal().id);
                calls.lock().unwrap().calls.push("find_current");
                Ok(Some(value.clone()))
            });
        assert_eq!(
            kind(&failure(harness.service().current_receipt("session"))),
            &OwnerCertificateError::Inconsistent
        );
        assert_eq!(
            observed.lock().unwrap().calls,
            ["authenticate", "find_current"]
        );
    }
}

#[test]
fn current_receipt_reauthenticates_full_principal_after_both_presence_and_absence() {
    for found in [None, Some(receipt())] {
        for revoked in [false, true] {
            let mut harness = Harness::new();
            let observed = harness.state.clone();
            let calls = observed.clone();
            let found = found.clone();
            harness
                .store
                .expect_find_current()
                .times(1)
                .returning(move |actor| {
                    assert_eq!(actor, principal().id);
                    let mut state = calls.lock().unwrap();
                    state.calls.push("find_current");
                    if revoked {
                        state.principal = None;
                    } else {
                        state.principal.as_mut().unwrap().email = "changed@example.test".into();
                    }
                    Ok(found.clone())
                });
            assert!(matches!(
                harness.service().current_receipt("session"),
                Err(ApplicationError::InvalidSession)
            ));
            assert_eq!(
                observed.lock().unwrap().calls,
                ["authenticate", "find_current", "authenticate"]
            );
        }
    }
}
