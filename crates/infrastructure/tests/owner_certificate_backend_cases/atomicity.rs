use uuid::Uuid;

use crate::{capture, support::*};

#[test]
fn rejected_audit_insertion_rolls_back_each_mutation_and_preserves_every_user_field() {
    let Some((mut db, clock, _)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let id = Uuid::from_u128(61);
    let command = capture::registration(repository.clone(), &clock, actor.clone(), id);
    let before = snapshot(&mut db);
    db.admin.batch_execute(&format!("ALTER TABLE audit_events ADD CONSTRAINT reject_owner_registration CHECK(action <> '{REGISTER}') NOT VALID")).unwrap();
    assert!(repository.commit_registration(command).is_err());
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .batch_execute("ALTER TABLE audit_events DROP CONSTRAINT reject_owner_registration")
        .unwrap();
    let command = capture::registration(repository.clone(), &clock, actor.clone(), id);
    let registered = applied(repository.commit_registration(command).unwrap());
    let command = capture::withdrawal(repository.clone(), &clock, actor.clone(), id);
    let before = snapshot(&mut db);
    db.admin.batch_execute(&format!("ALTER TABLE audit_events ADD CONSTRAINT reject_owner_withdrawal CHECK(action <> '{WITHDRAW}') NOT VALID")).unwrap();
    assert!(repository.commit_withdrawal(command).is_err());
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .batch_execute("ALTER TABLE audit_events DROP CONSTRAINT reject_owner_withdrawal")
        .unwrap();
    assert_eq!(
        store(&db, &clock).find(owner, id).unwrap(),
        Some(registered.clone())
    );
    let command = capture::withdrawal(repository.clone(), &clock, actor, id);
    let retired = applied(repository.commit_withdrawal(command).unwrap());
    assert_eq!(counts(&mut db), (1, 1, 2));
    assert_audit(&db, &[(&registered, false), (&retired, true)]);
}
