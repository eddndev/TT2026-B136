use super::*;
use std::sync::Mutex;

#[test]
fn failed_replacement_audit_leaves_neither_mark_new_root_nor_linked_rows() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, _, command) = setup_joint(&mut db);
    let workflow = service(&db, seed.actor.clone());
    let draft = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    db.admin.batch_execute("CREATE FUNCTION reject_replacement_audit() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.action='measure_administrative.recorded' THEN RAISE EXCEPTION 'injected replacement audit failure';
        END IF; RETURN NEW; END; $$;
        CREATE TRIGGER reject_replacement_audit BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_replacement_audit()").unwrap();
    let before = snapshot(&mut db);
    assert!(workflow
        .submit("session", db.case, command.clone(), confirmation(&draft))
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_replacement_audit ON audit_events;
        DROP FUNCTION reject_replacement_audit()",
        )
        .unwrap();
    let stored = workflow
        .submit("session", db.case, command, confirmation(&draft))
        .unwrap();
    assert_eq!(stored.capture.records.len(), 2);
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn replacement_rechecks_current_membership_after_retained_support_admission() {
    let Some(mut db) = Fixture::new() else { return };
    let (_, _, _, command) = setup_joint(&mut db);
    let user = db.user("litigator", true);
    let actor = crate::measure_fixture::principal(&mut db, user);
    let draft = service(&db, actor.clone())
        .prepare("session", db.case, command.clone())
        .unwrap();
    let case = db.case;
    let admin_url = db.admin_url.clone();
    let called = Arc::new(Mutex::new(false));
    let captured = called.clone();
    let workflow = service_with_format(
        &db,
        actor,
        FormatCheck(Some(Box::new(move || {
            let mut client = postgres::Client::connect(&admin_url, postgres::NoTls).unwrap();
            client
                .execute(
                    "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                    &[&case.as_uuid(), &user.as_uuid()],
                )
                .unwrap();
            *captured.lock().unwrap() = true;
        }))),
    );
    let before = snapshot(&mut db);
    assert!(workflow
        .submit("session", db.case, command, confirmation(&draft))
        .is_err());
    assert!(*called.lock().unwrap());
    assert_eq!(snapshot(&mut db), before);
}
