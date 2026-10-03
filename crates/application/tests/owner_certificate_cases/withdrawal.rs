use application::{identity::owner_certificates::*, ApplicationError};
use domain::identity::{Role, UserId};
use uuid::Uuid;

use crate::support::*;

#[test]
fn withdrawal_preserves_original_evidence_after_expiry_without_crypto_or_current_trust() {
    let mut harness = Harness::new();
    let observed = harness.state.clone();
    {
        let mut state = observed.lock().unwrap();
        state.now = 3000;
        state.context.trust = None;
        state.context.account.revision = 10;
    }
    harness.withdrawal(receipt());
    let calls = observed.clone();
    harness
        .store
        .expect_commit_withdrawal()
        .times(1)
        .returning(move |command| {
            calls.lock().unwrap().calls.push("commit_withdrawal");
            assert_eq!(command.account().revision, 10);
            assert_eq!(command.account().auth_generation, 4);
            assert_eq!(command.original().record, receipt().record);
            assert_eq!(command.original().check, receipt().check);
            assert_eq!(command.record(), &retired().record);
            assert_eq!(command.not_before(), at(3000));
            Ok(OwnerBindingCommit::Applied(retired()))
        });
    let result = harness.service().withdraw("session", binding(), 1).unwrap();
    assert_eq!(result.record, retired().record);
    assert_eq!(result.check, receipt().check);
    assert_eq!(result.trust, receipt().trust);
    assert_eq!(
        observed.lock().unwrap().calls,
        [
            "authenticate",
            "load_withdrawal",
            "authenticate",
            "commit_withdrawal"
        ]
    );
}

#[test]
fn terminal_withdrawal_is_an_exact_receipt_and_other_expectations_do_not_write() {
    for expected in [0, 1, 2, u32::MAX] {
        let mut harness = Harness::new();
        let observed = harness.state.clone();
        {
            let mut state = observed.lock().unwrap();
            state.now = 4000;
            state.context.account.revision = 11;
            state.context.trust = None;
        }
        if expected == 1 {
            harness.withdrawal(retired());
        }
        let result = harness.service().withdraw("session", binding(), expected);
        if expected == 1 {
            assert_eq!(result.unwrap().withdrawn_at, Some(at(3000)));
            assert_eq!(
                observed.lock().unwrap().calls,
                ["authenticate", "load_withdrawal", "authenticate"]
            );
        } else {
            assert_eq!(
                kind(&failure(result)),
                &OwnerCertificateError::RevisionConflict
            );
            assert_eq!(observed.lock().unwrap().calls, ["authenticate"]);
        }
    }
}

#[test]
fn receipt_requires_current_self_owner_before_and_after_reading_historical_evidence() {
    for variant in 0..4 {
        let harness = Harness::new();
        let observed = harness.state.clone();
        {
            let mut state = observed.lock().unwrap();
            state.found = Some(retired());
            state.now = 4000;
            state.context.trust = None;
            match variant {
                0 => state.principal.as_mut().unwrap().role = Role::Client,
                1 => state.found.as_mut().unwrap().owner = UserId::from_uuid(Uuid::from_u128(2)),
                2 => state.expire_on_find = true,
                _ => {}
            }
        }
        let result = harness.service().receipt("session", binding());
        match variant {
            0 => {
                assert!(matches!(
                    failure(result),
                    ApplicationError::PermissionDenied
                ));
                assert_eq!(observed.lock().unwrap().calls, ["authenticate"]);
            }
            1 => assert_eq!(kind(&failure(result)), &OwnerCertificateError::Inconsistent),
            2 => assert!(matches!(failure(result), ApplicationError::InvalidSession)),
            _ => {
                assert_eq!(result.unwrap().unwrap().record, retired().record);
                assert_eq!(
                    observed.lock().unwrap().calls,
                    ["authenticate", "find", "authenticate"]
                );
            }
        }
    }
}
