use super::*;
use domain::crypto::Sha256Digest;

#[test]
fn wrong_confirmation_stale_predecessor_and_terminal_successor_leave_no_partial_rows() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let service = service(&db, actor.clone());
    let draft = service
        .prepare("session", db.case, command.clone())
        .unwrap();
    let before = snapshot(&mut db);
    let mut wrong = confirmation(&draft);
    wrong.review_digest = Sha256Digest::from_array([0x99; 32]);
    assert!(service
        .submit("session", db.case, command.clone(), wrong)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    let first = persist(&db, actor.clone(), command);
    let second = persist(&db, actor.clone(), replacement(&first));
    let before = snapshot(&mut db);
    assert!(service
        .prepare("session", db.case, cancellation(&first))
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    let cancelled = persist(&db, actor, cancellation(&second));
    let before = snapshot(&mut db);
    assert!(service
        .prepare("session", db.case, replacement(&cancelled))
        .is_err());
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn audit_and_deferred_capture_failures_roll_back_root_revision_and_audit_atomically() {
    for failure in [
        "CREATE FUNCTION reject_precautionary_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected audit failure'; END $$; CREATE TRIGGER reject_precautionary_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_precautionary_audit()",
        "CREATE FUNCTION reject_precautionary_capture() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected deferred capture failure'; END $$; CREATE CONSTRAINT TRIGGER reject_precautionary_capture AFTER INSERT ON case_precautionary_hearing_revisions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_precautionary_capture()",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let (actor, command) = setup(&mut db);
        let draft = service(&db, actor.clone()).prepare("session", db.case, command.clone()).unwrap();
        let url = db.admin_url.clone();
        let workflow = service_with_format(&db, actor, FormatCheck(Some(Box::new(move || {
            postgres::Client::connect(&url, postgres::NoTls).unwrap().batch_execute(failure).unwrap();
        }))));
        let before = snapshot(&mut db);
        assert!(workflow.submit("session", db.case, command, confirmation(&draft)).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}
