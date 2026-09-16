mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_result_database_support;
mod hearing_result_restore_support;
mod hearing_result_revalidation_support;
#[path = "hearing_result_revalidation_support/retention_tests.rs"]
mod retention_tests;
#[path = "hearing_result_revalidation_support/source_tests.rs"]
mod source_tests;
mod typed_participant_service_support;

use application::{case_stages::*, cases::*, hearing_results::*, ApplicationError};
use domain::identity::Role;
use hearing_result_database_support::{record, service, Fixture};
use hearing_result_revalidation_support::*;

#[test]
fn membership_role_and_active_identity_are_rechecked_after_result_preparation() {
    for (mutation, expected) in [
        ("DELETE FROM case_memberships WHERE user_id=$1", "hidden"),
        ("UPDATE users SET active=FALSE WHERE id=$1", "inactive"),
        ("UPDATE users SET role='paralegal' WHERE id=$1", "role"),
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let anchor = appointment(&mut db);
        let actor = db.user("litigator", true);
        let command = record(anchor.snapshot.id);
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
fn closing_the_case_after_preparation_rejects_the_result_without_partial_rows_or_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let anchor = appointment(&mut db);
    let command = record(anchor.snapshot.id);
    let draft = service(&db, db.owner, Role::Owner)
        .prepare("session", db.case, command.clone())
        .unwrap();
    let repository = db.store();
    let (actor, case, at) = (db.owner, db.case, db.at);
    let (workflow, captured) = watched(&db, actor, Role::Owner, move |_| {
        repository
            .change_administrative_status(
                actor,
                case,
                CaseRevisionExpectation::new(1),
                CaseAdministrativeStatus::Closed,
                at,
            )
            .unwrap();
    });
    let result = workflow.submit("session", case, command, draft.submission_digest);
    assert!(
        matches!(result, Err(ApplicationError::CaseClosed)),
        "{result:?}"
    );
    unchanged(&mut db, captured);
}

#[test]
fn later_stage_and_administration_are_captured_without_replacing_the_exact_anchor() {
    let Some(mut db) = Fixture::new() else { return };
    let anchor = appointment(&mut db);
    let document = case_stage_database_support::upload(&db, db.case, "accusation.pdf");
    let command = record(anchor.snapshot.id);
    let draft = service(&db, db.owner, Role::Owner)
        .prepare("session", db.case, command.clone())
        .unwrap();
    let stages = case_stage_database_support::service(
        &db,
        db.owner,
        Role::Owner,
        case_stage_database_support::FormatCheck(None),
    );
    let repository = db.store();
    let (actor, case, at) = (db.owner, db.case, db.at);
    let changed =
        case_stage_database_support::creation("Later administrative revision").into_values();
    let editable = changed.editable().clone();
    let workflow = hooked(&db, actor, Role::Owner, move || {
        stages
            .transition(
                "session",
                case,
                CaseStageRevision::FIRST,
                StageTransition::to_intermediate(
                    DeclaredStageTime::instant(at).unwrap(),
                    case_stage_database_support::reference(&document),
                    None,
                ),
            )
            .unwrap();
        repository
            .replace_administration(
                actor,
                case,
                CaseRevisionExpectation::new(1),
                editable.clone(),
                at,
            )
            .unwrap();
    });
    let recorded = workflow
        .submit("session", case, command, draft.submission_digest)
        .unwrap();
    assert_eq!(recorded.snapshot.recorded_administration_revision.get(), 2);
    assert_eq!(
        recorded.snapshot.receipt.submission_digest,
        draft.submission_digest
    );
    assert_eq!(recorded.anchor, draft.anchor);
    assert_eq!(recorded.anchor.scheduling_context.stage_revision.get(), 1);
}

#[test]
fn root_revision_audit_and_deferred_failures_roll_back_the_entire_result_write() {
    for failure in [
        "CREATE FUNCTION reject_result() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected root failure'; END $$; CREATE TRIGGER reject_result BEFORE INSERT ON case_hearing_results FOR EACH ROW EXECUTE FUNCTION reject_result()",
        "CREATE FUNCTION reject_result() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected revision failure'; END $$; CREATE TRIGGER reject_result BEFORE INSERT ON case_hearing_result_revisions FOR EACH ROW EXECUTE FUNCTION reject_result()",
        "ALTER TABLE audit_events ADD CONSTRAINT reject_result CHECK(action<>'hearing_result.recorded')",
        "CREATE FUNCTION reject_result() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected deferred failure'; END $$; CREATE CONSTRAINT TRIGGER reject_result AFTER INSERT ON case_hearing_result_revisions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_result()",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let anchor = appointment(&mut db);
        let command = record(anchor.snapshot.id);
        let draft = service(&db, db.owner, Role::Owner).prepare("session", db.case, command.clone()).unwrap();
        let (workflow, captured) = watched(&db, db.owner, Role::Owner, move |client| { client.batch_execute(failure).unwrap(); });
        let result = workflow.submit("session", db.case, command, draft.submission_digest);
        assert!(matches!(result, Err(ApplicationError::Port(_))), "{result:?}");
        unchanged(&mut db, captured);
    }
}
