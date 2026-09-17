mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_revalidation_support;

use application::{cases::*, hearings::*, ApplicationError};
use domain::identity::Role;
use hearing_database_support::*;
use hearing_revalidation_support::*;

#[test]
fn membership_role_and_active_identity_are_rechecked_after_preparation() {
    for (mutation, expected) in [
        ("DELETE FROM case_memberships WHERE user_id=$1", "hidden"),
        ("UPDATE users SET active=FALSE WHERE id=$1", "inactive"),
        ("UPDATE users SET role='paralegal' WHERE id=$1", "role"),
    ] {
        let Some(mut db) = Fixture::new() else { return };
        complete(&mut db);
        let actor = db.user("litigator", true);
        let command = schedule();
        let draft = service(&db, actor, Role::Litigator)
            .prepare("session", db.case, command.clone())
            .unwrap();
        let (workflow, captured) = watched(&db, actor, Role::Litigator, move |client| {
            client.execute(mutation, &[&actor.as_uuid()]).unwrap();
        });
        let result = workflow.submit("session", db.case, command, draft.submission_digest);
        match expected {
            "hidden" => assert!(
                matches!(result, Err(ApplicationError::CaseNotFound)),
                "{result:?}"
            ),
            "inactive" => assert!(
                matches!(result, Err(ApplicationError::InvalidSession)),
                "{result:?}"
            ),
            _ => assert!(
                matches!(result, Err(ApplicationError::PermissionDenied)),
                "{result:?}"
            ),
        }
        unchanged(&mut db, captured);
    }
}

#[test]
fn closure_and_administration_changes_after_preparation_do_not_leave_a_hearing() {
    for close in [false, true] {
        let Some(mut db) = Fixture::new() else { return };
        complete(&mut db);
        let command = schedule();
        let draft = service(&db, db.owner, Role::Owner)
            .prepare("session", db.case, command.clone())
            .unwrap();
        let repository = db.store();
        let (actor, case, at) = (db.owner, db.case, db.at);
        let (workflow, captured) = watched(&db, actor, Role::Owner, move |_| {
            if close {
                repository
                    .change_administrative_status(
                        actor,
                        case,
                        CaseRevisionExpectation::new(1),
                        CaseAdministrativeStatus::Closed,
                        at,
                    )
                    .unwrap();
            } else {
                repository
                    .replace_administration(
                        actor,
                        case,
                        CaseRevisionExpectation::new(1),
                        case_stage_database_support::creation("Updated hearing context")
                            .into_values()
                            .editable()
                            .clone(),
                        at,
                    )
                    .unwrap();
            }
        });
        let result = workflow.submit("session", case, command, draft.submission_digest);
        if close {
            assert!(
                matches!(result, Err(ApplicationError::CaseClosed)),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(
                    result,
                    Err(ApplicationError::Hearing(HearingError::ContextConflict))
                ),
                "{result:?}"
            );
        }
        unchanged(&mut db, captured);
    }
}

#[test]
fn used_operations_and_submission_mismatches_do_not_change_rows_or_audit() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let command = schedule();
    let draft = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    let before = snapshot(&mut db.admin);
    let mismatch = domain::crypto::Sha256Digest::from_bytes(&[0; 32]).unwrap();
    assert!(matches!(
        workflow.submit("session", db.case, command.clone(), mismatch),
        Err(ApplicationError::Hearing(HearingError::SubmissionMismatch))
    ));
    assert_eq!(snapshot(&mut db.admin), before);
    workflow
        .submit("session", db.case, command.clone(), draft.submission_digest)
        .unwrap();
    let before = snapshot(&mut db.admin);
    assert!(matches!(
        workflow.submit("session", db.case, command.clone(), draft.submission_digest),
        Err(ApplicationError::Hearing(HearingError::OperationConflict))
    ));
    let mut other = schedule();
    other.operation_id = command.operation_id;
    assert!(matches!(
        workflow.prepare("session", db.case, other),
        Err(ApplicationError::Hearing(HearingError::OperationConflict))
    ));
    assert_eq!(snapshot(&mut db.admin), before);
}

#[test]
fn insert_audit_and_deferred_failures_roll_back_the_entire_hearing_write() {
    for failure in [
        "CREATE FUNCTION reject_hearing() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected root failure'; END $$; CREATE TRIGGER reject_hearing BEFORE INSERT ON case_hearings FOR EACH ROW EXECUTE FUNCTION reject_hearing()",
        "CREATE FUNCTION reject_hearing() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected revision failure'; END $$; CREATE TRIGGER reject_hearing BEFORE INSERT ON case_hearing_revisions FOR EACH ROW EXECUTE FUNCTION reject_hearing()",
        "ALTER TABLE audit_events ADD CONSTRAINT reject_hearing CHECK(action<>'hearing.scheduled')",
        "CREATE FUNCTION reject_hearing() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected deferred failure'; END $$; CREATE CONSTRAINT TRIGGER reject_hearing AFTER INSERT ON case_hearing_revisions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_hearing()",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        complete(&mut db);
        let command = schedule();
        let draft = service(&db, db.owner, Role::Owner).prepare("session", db.case, command.clone()).unwrap();
        let (workflow, captured) = watched(&db, db.owner, Role::Owner, move |client| { client.batch_execute(failure).unwrap(); });
        let result = workflow.submit("session", db.case, command, draft.submission_digest);
        assert!(matches!(result, Err(ApplicationError::Port(_))), "{result:?}");
        unchanged(&mut db, captured);
    }
}
