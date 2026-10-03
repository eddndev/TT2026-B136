use application::ApplicationError;
use infrastructure::with_validated_postgres;
use uuid::Uuid;

use crate::support::*;

fn advance_account(db: &mut Fixture) {
    let owner = db.owner.as_uuid();
    for active in [false, true] {
        db.admin
            .execute(
                "UPDATE users SET active=$2,revision=revision+1,
            auth_generation=auth_generation+1 WHERE id=$1",
                &[&owner, &active],
            )
            .unwrap();
    }
    db.admin
        .execute(
            "UPDATE users SET revision=revision+1 WHERE id=$1",
            &[&owner],
        )
        .unwrap();
}

#[test]
fn restored_account_counters_cannot_precede_registration_or_withdrawal_captures() {
    let mut rejected = Vec::new();
    for terminal in [false, true] {
        for counter in ["revision", "auth_generation"] {
            let Some((mut db, clock, _)) = fixture() else {
                return;
            };
            db.user("owner", false);
            advance_account(&mut db);
            let owner = db.owner;
            let actor = principal(&mut db, owner);
            let workflow = service(store(&db, &clock), &clock, actor);
            let id = Uuid::from_u128(801);
            let (prepared, signature) = preparation(&workflow, id);
            let original = workflow.register(TOKEN, &prepared, &signature).unwrap();
            assert_eq!(original.record.registration().owner().revision(), 3);
            assert_eq!(original.record.registration().owner().generation(), 2);
            if terminal {
                advance_account(&mut db);
                let receipt = workflow.withdraw(TOKEN, id, 1).unwrap();
                let capture = receipt.record.withdrawal().unwrap().owner();
                assert_eq!((capture.revision(), capture.generation()), (6, 4));
            }
            // Model a mixed restored users row, with every evidence byte intact.
            let mut corruption = db.admin.transaction().unwrap();
            corruption
                .batch_execute("ALTER TABLE users DISABLE TRIGGER member_access_guard")
                .unwrap();
            corruption
                .execute(
                    &format!("UPDATE users SET {counter}={counter}-1 WHERE id=$1"),
                    &[&owner.as_uuid()],
                )
                .unwrap();
            corruption
                .batch_execute("ALTER TABLE users ENABLE TRIGGER member_access_guard")
                .unwrap();
            corruption.commit().unwrap();
            let before = snapshot(&mut db);
            let open_rejected =
                PostgresOwnerCertificateStore::open(&db.runtime_url, clock.clone()).is_err();
            let check_rejected =
                with_validated_postgres(&db.runtime_url, |_| Ok::<(), ApplicationError>(()))
                    .is_err();
            assert_eq!(snapshot(&mut db), before);
            rejected.push((terminal, counter, open_rejected, check_rejected));
        }
    }
    assert!(
        rejected.iter().all(|(_, _, open, check)| *open && *check),
        "historical capture exceeds a restored account counter: {rejected:?}"
    );
}

#[test]
fn current_role_and_activity_changes_preserve_valid_historical_certificate_inventory() {
    let Some((mut db, clock, trust)) = fixture() else {
        return;
    };
    db.user("owner", false);
    advance_account(&mut db);
    let owner = db.owner;
    let actor = principal(&mut db, owner);
    let workflow = service(store(&db, &clock), &clock, actor);
    let id = Uuid::from_u128(802);
    let (prepared, signature) = preparation(&workflow, id);
    workflow.register(TOKEN, &prepared, &signature).unwrap();
    advance_account(&mut db);
    let terminal = workflow.withdraw(TOKEN, id, 1).unwrap();
    clock.set(trust.inspection.valid_until + 1);
    db.admin
        .execute(
            "UPDATE users SET role='litigator',active=false,
        revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
            &[&owner.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    let reopened = store(&db, &clock);
    with_validated_postgres(&db.runtime_url, |_| Ok::<(), ApplicationError>(())).unwrap();
    assert!(matches!(
        reopened.find(owner, id),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .execute(
            "UPDATE users SET role='owner',active=true,
        revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
            &[&owner.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    assert_eq!(reopened.find(owner, id).unwrap(), Some(terminal));
    assert_eq!(snapshot(&mut db), before);
}
