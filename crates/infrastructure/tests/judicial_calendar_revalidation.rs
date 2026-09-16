mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod judicial_calendar_database_support;
mod judicial_calendar_interleaved_support;
use application::{judicial_calendars::*, ApplicationError};
use domain::identity::Role;
use judicial_calendar_database_support::*;
use judicial_calendar_interleaved_support::*;
use postgres::{Client, NoTls};

#[test]
fn prepared_calendar_is_denied_if_owner_is_disabled_or_demoted_before_commit() {
    for disabled in [true, false] {
        let Some(mut db) = Fixture::new() else { return };
        let command = publish();
        let draft = service(&db, db.owner, Role::Owner)
            .prepare("session", command.clone())
            .unwrap();
        let before = snapshot(&mut db);
        let (url, owner) = (db.admin_url.clone(), db.owner);
        let workflow = hooked(&db, owner, move || {
            let mut admin = Client::connect(&url, NoTls).unwrap();
            admin
                .execute(
                    if disabled {
                        "UPDATE users SET active=FALSE WHERE id=$1"
                    } else {
                        "UPDATE users SET role='litigator' WHERE id=$1"
                    },
                    &[&owner.as_uuid()],
                )
                .unwrap();
        });
        let result = workflow.submit("session", command, draft.submission_digest);
        if disabled {
            assert!(matches!(result, Err(ApplicationError::InvalidSession)));
        } else {
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
        }
        assert_eq!(snapshot(&mut db), before);
    }
}
#[test]
fn failed_audit_insert_rolls_back_both_calendar_tables() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let command = publish();
    let draft = workflow.prepare("session", command.clone()).unwrap();
    db.admin.batch_execute("CREATE FUNCTION reject_calendar_audit() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.action='judicial_calendar.published' THEN RAISE EXCEPTION 'injected audit failure'; END IF; RETURN NEW; END; $$;
        CREATE TRIGGER reject_calendar_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_calendar_audit()").unwrap();
    let before = snapshot(&mut db);
    assert!(workflow
        .submit("session", command, draft.submission_digest)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
}
#[test]
fn stale_expected_revision_and_incorrect_receipt_never_append_calendar_or_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let command = publish();
    let draft = workflow.prepare("session", command.clone()).unwrap();
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.submit(
            "session",
            command.clone(),
            domain::crypto::Sha256Digest::from_array([0; 32])
        ),
        Err(ApplicationError::JudicialCalendar(
            JudicialCalendarError::SubmissionMismatch
        ))
    ));
    assert_eq!(snapshot(&mut db), before);
    let first = workflow
        .submit("session", command, draft.submission_digest)
        .unwrap();
    let stale = replace(&first);
    let draft = workflow.prepare("session", stale.clone()).unwrap();
    persist(&workflow, retire(&first));
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.submit("session", stale, draft.submission_digest),
        Err(ApplicationError::JudicialCalendar(
            JudicialCalendarError::RevisionConflict
        ))
    ));
    assert_eq!(snapshot(&mut db), before);
}
