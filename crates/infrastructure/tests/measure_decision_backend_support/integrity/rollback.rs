use super::*;

#[test]
fn audit_and_deferred_owner_failures_roll_back_every_group_table_atomically() {
    for failure in [
        "CREATE FUNCTION reject_measure_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected audit failure'; END $$; CREATE TRIGGER reject_measure_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_measure_audit()",
        "CREATE FUNCTION reject_measure_owner() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected deferred owner failure'; END $$; CREATE CONSTRAINT TRIGGER reject_measure_owner AFTER INSERT ON case_measure_operations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_measure_owner()",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let seed = setup(&mut db);
        let draft = service(&db, seed.actor.clone()).prepare("session", db.case, seed.command.clone()).unwrap();
        let url = db.admin_url.clone();
        let workflow = service_with_format(&db, seed.actor, FormatCheck(Some(Box::new(move || {
            postgres::Client::connect(&url, postgres::NoTls).unwrap().batch_execute(failure).unwrap();
        }))));
        let before = snapshot(&mut db);
        assert!(workflow.submit("session", db.case, seed.command, confirmation(&draft)).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}
