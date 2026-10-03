use std::{
    sync::{Arc, Barrier},
    thread,
};

use application::ApplicationError;
use uuid::Uuid;

use crate::{capture, support::*};

fn race(
    stores: [Arc<PostgresOwnerCertificateStore>; 2],
    commands: [VerifiedOwnerRegistration; 2],
) -> Vec<Result<OwnerBindingCommit, ApplicationError>> {
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = stores
        .into_iter()
        .zip(commands)
        .map(|(store, command)| {
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                store.commit_registration(command)
            })
        })
        .collect();
    handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect()
}

#[test]
fn same_uuid_races_are_idempotent_and_distinct_live_uuids_have_one_winner() {
    let Some((mut db, clock, _)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let id = Uuid::from_u128(41);
    let commands = [
        capture::registration(repository.clone(), &clock, actor.clone(), id),
        capture::registration(repository.clone(), &clock, actor.clone(), id),
    ];
    let mut results = race([store(&db, &clock), store(&db, &clock)], commands);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Ok(OwnerBindingCommit::Applied(_))))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Ok(OwnerBindingCommit::Existing(_))))
            .count(),
        1
    );
    let receipts: Vec<_> = results
        .drain(..)
        .map(|result| match result.unwrap() {
            OwnerBindingCommit::Applied(value) | OwnerBindingCommit::Existing(value) => value,
        })
        .collect();
    assert_eq!(receipts[0], receipts[1]);
    assert_eq!(counts(&mut db), (1, 0, 1));
    let workflow = service(repository.clone(), &clock, actor.clone());
    let retired = workflow.withdraw(TOKEN, id, 1).unwrap();
    let commands = [
        capture::registration(
            repository.clone(),
            &clock,
            actor.clone(),
            Uuid::from_u128(42),
        ),
        capture::registration(repository, &clock, actor, Uuid::from_u128(43)),
    ];
    let results = race([store(&db, &clock), store(&db, &clock)], commands);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(
                r,
                Err(ApplicationError::OwnerCertificate(
                    OwnerCertificateError::ActiveBinding
                ))
            ))
            .count(),
        1
    );
    let winners: Vec<_> = results
        .into_iter()
        .filter_map(|result| match result {
            Ok(OwnerBindingCommit::Applied(receipt)) => Some(receipt),
            Err(ApplicationError::OwnerCertificate(OwnerCertificateError::ActiveBinding)) => None,
            _ => panic!("competing live bindings must have one applied result and one conflict"),
        })
        .collect();
    assert_eq!(winners.len(), 1);
    assert_eq!(counts(&mut db), (2, 1, 3));
    assert_audit(
        &db,
        &[
            (&receipts[0], false),
            (&retired, true),
            (&winners[0], false),
        ],
    );
}

#[test]
fn racing_withdrawals_and_late_retry_keep_one_original_terminal_receipt() {
    let Some((mut db, clock, trust)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let id = Uuid::from_u128(51);
    let command = capture::registration(repository.clone(), &clock, actor.clone(), id);
    let registered = applied(repository.commit_registration(command).unwrap());
    let commands = [
        capture::withdrawal(repository.clone(), &clock, actor.clone(), id),
        capture::withdrawal(repository.clone(), &clock, actor.clone(), id),
    ];
    let retry = capture::withdrawal(repository.clone(), &clock, actor, id);
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = [store(&db, &clock), store(&db, &clock)]
        .into_iter()
        .zip(commands)
        .map(|(store, command)| {
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                store.commit_withdrawal(command)
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap().unwrap())
        .collect();
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, OwnerBindingCommit::Applied(_)))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, OwnerBindingCommit::Existing(_)))
            .count(),
        1
    );
    let values: Vec<_> = results
        .into_iter()
        .map(|result| match result {
            OwnerBindingCommit::Applied(value) | OwnerBindingCommit::Existing(value) => value,
        })
        .collect();
    assert_eq!(values[0], values[1]);
    db.admin
        .execute(
            "UPDATE users SET revision=revision+1 WHERE id=$1",
            &[&owner.as_uuid()],
        )
        .unwrap();
    clock.set(trust.inspection.valid_until + 1);
    let before = snapshot(&mut db);
    assert_eq!(
        existing(repository.commit_withdrawal(retry).unwrap()),
        values[0]
    );
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(counts(&mut db), (1, 1, 2));
    assert_audit(&db, &[(&registered, false), (&values[0], true)]);
}
