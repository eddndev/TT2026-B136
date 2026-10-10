#![allow(dead_code)]
mod case_administration_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::{
    document_signing::{IndividualSigningStore, SigningBindingStore, SigningCommit},
    ApplicationError,
};
use domain::clock::Clock as _;
use individual_document_backend_support::Harness;
use uuid::Uuid;

#[test]
fn personal_evidence_receipt_and_audit_commit_once_and_survive_reopening() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let operation = Uuid::new_v4();
    let first = test.command(operation);
    let duplicate = test.command(operation);
    let store = test.store();
    let SigningCommit::Applied(receipt) = store.commit(first).unwrap() else {
        panic!("expected new signature")
    };
    assert_eq!(test.mutation_counts(), (1, 1));
    assert_eq!(receipt.accepted_at, test.clock.now());
    assert_eq!(
        receipt.evidence.individual.as_ref().unwrap().accepted_at,
        receipt.accepted_at.unix_timestamp()
    );
    let SigningCommit::Existing(replayed) = store.commit(duplicate).unwrap() else {
        panic!("expected exact replay")
    };
    assert_eq!(replayed, receipt);
    assert_eq!(test.mutation_counts(), (1, 1));
    drop(store);
    assert_eq!(
        test.store()
            .find(&test.actor, test.db.case, test.reference(), operation)
            .unwrap(),
        Some(receipt)
    );
    test.db.migrate();
    assert_eq!(test.mutation_counts(), (1, 1));
}

#[test]
fn account_membership_binding_and_expiry_changes_reject_before_any_document_write() {
    for mutation in 0..4 {
        let Some(mut test) = Harness::new() else {
            return;
        };
        let command = test.command(Uuid::new_v4());
        match mutation {
            0 => {
                test.db
                    .admin
                    .execute(
                        "UPDATE users SET email='changed@example.test' WHERE id=$1",
                        &[&test.actor.id.as_uuid()],
                    )
                    .unwrap();
            }
            1 => {
                test.db
                    .admin
                    .execute(
                        "DELETE FROM case_memberships WHERE user_id=$1",
                        &[&test.actor.id.as_uuid()],
                    )
                    .unwrap();
            }
            2 => {
                let store = signing_binding_support::store(&test.db, &test.clock);
                let context = store.load_context(test.actor.id).unwrap();
                store
                    .withdraw(
                        &test.actor,
                        test.binding_id,
                        context.account,
                        test.clock.now(),
                    )
                    .unwrap();
            }
            _ => test.clock.set(command.check().valid_until + 1),
        }
        let before = test.snapshot();
        assert!(test.store().commit(command).is_err());
        assert_eq!(test.snapshot(), before);
        assert_eq!(test.mutation_counts(), (0, 0));
    }
}

#[test]
fn audit_failure_rolls_back_document_evidence_and_receipt() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let command = test.command(Uuid::new_v4());
    let store = test.store();
    test.db.admin.batch_execute("CREATE FUNCTION reject_personal_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        IF NEW.action='document.individually_signed' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$;
        CREATE TRIGGER reject_personal_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_personal_audit();").unwrap();
    let before = test.snapshot();
    assert!(store.commit(command).is_err());
    assert_eq!(test.snapshot(), before);
}

#[test]
fn a_second_operation_cannot_overwrite_a_signed_version() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let first = test.command(Uuid::new_v4());
    let second = test.command(Uuid::new_v4());
    let store = test.store();
    store.commit(first).unwrap();
    let before = test.snapshot();
    assert!(matches!(
        store.commit(second),
        Err(ApplicationError::DocumentAlreadySealed(_))
    ));
    assert_eq!(test.snapshot(), before);
}

#[test]
fn concurrent_exact_signatures_commit_one_evidence_and_recover_one_receipt() {
    use std::sync::{Arc, Barrier};
    let Some(mut test) = Harness::new() else {
        return;
    };
    let operation = Uuid::new_v4();
    let commands = [test.command(operation), test.command(operation)];
    let stores = [test.store(), test.store()];
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = stores
        .into_iter()
        .zip(commands)
        .map(|(store, command)| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.commit(command).unwrap()
            })
        })
        .collect();
    let mut applied = Vec::new();
    let mut recovered = Vec::new();
    for thread in threads {
        match thread.join().unwrap() {
            SigningCommit::Applied(receipt) => applied.push(receipt),
            SigningCommit::Existing(receipt) => recovered.push(receipt),
        }
    }
    assert_eq!(applied.len(), 1);
    assert_eq!(recovered, applied);
    assert_eq!(test.mutation_counts(), (1, 1));
}
