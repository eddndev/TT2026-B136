use application::identity::owner_certificates::*;
use domain::{
    identity::Role,
    owner_certificates::{BindingMaterial, BindingRecord, BindingStatement, OwnerAccount},
};
use uuid::Uuid;

use crate::support::*;

fn other_registration(id: Uuid, revision: u64, terminal: bool) -> OwnerBindingReceipt {
    let original = statement();
    let material = original.material();
    let statement = BindingStatement::new(
        OwnerAccount::new(principal().id, Role::Owner, true, revision, 4).unwrap(),
        principal().id,
        BindingMaterial::new(
            material.deployment(),
            id,
            material.root(),
            material.leaf(),
            material.trust_revision(),
        )
        .unwrap(),
    )
    .unwrap();
    let mut value = receipt();
    value.check = check(&statement, &signature(), &trust(), 1000);
    value.record = BindingRecord::registered(statement);
    if terminal {
        value.record = value
            .record
            .withdraw(
                OwnerAccount::new(principal().id, Role::Owner, true, 10, 4).unwrap(),
                1,
            )
            .unwrap();
        value.withdrawn_at = Some(at(3000));
    }
    value
}

#[test]
fn receipt_and_withdrawal_reject_another_binding_of_the_same_owner() {
    for operation in 0..3 {
        let mut harness = Harness::new();
        let observed = harness.state.clone();
        let other = other_registration(Uuid::from_u128(45), 9, operation == 2);
        if operation == 0 {
            observed.lock().unwrap().found = Some(other);
        } else {
            harness.withdrawal(other);
        }
        let service = harness.service();
        let error = if operation == 0 {
            failure(service.receipt("session", binding()))
        } else {
            failure(service.withdraw("session", binding(), 1))
        };
        assert_eq!(kind(&error), &OwnerCertificateError::Inconsistent);
        assert_eq!(
            observed.lock().unwrap().calls,
            [
                "authenticate",
                if operation == 0 {
                    "find"
                } else {
                    "load_withdrawal"
                }
            ]
        );
    }
}

#[test]
fn concurrent_withdrawal_preserves_the_original_terminal_receipt_only() {
    for variant in 0..3 {
        let mut harness = Harness::new();
        let observed = harness.state.clone();
        {
            let mut state = observed.lock().unwrap();
            state.context.account.revision = 11;
            state.context.trust = None;
            state.now = 4000;
        }
        harness.withdrawal(receipt());
        let calls = observed.clone();
        harness
            .store
            .expect_commit_withdrawal()
            .times(1)
            .returning(move |command| {
                calls.lock().unwrap().calls.push("commit_withdrawal");
                assert_eq!(command.account().revision, 11);
                assert_eq!(command.not_before(), at(4000));
                assert_ne!(command.record(), &retired().record);
                Ok(OwnerBindingCommit::Existing(match variant {
                    0 => retired(),
                    1 => receipt(),
                    _ => other_registration(binding(), 8, true),
                }))
            });
        let result = harness.service().withdraw("session", binding(), 1);
        if variant == 0 {
            let value = result.unwrap();
            assert_eq!(value.record, retired().record);
            assert_eq!(value.withdrawn_at, Some(at(3000)));
        } else {
            assert_eq!(kind(&failure(result)), &OwnerCertificateError::Inconsistent);
        }
        let mut expected = vec![
            "authenticate",
            "load_withdrawal",
            "authenticate",
            "commit_withdrawal",
        ];
        if variant == 0 {
            expected.push("authenticate");
        }
        assert_eq!(observed.lock().unwrap().calls, expected);
    }
}
