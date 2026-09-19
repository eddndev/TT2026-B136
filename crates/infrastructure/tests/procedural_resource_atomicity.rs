mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;
mod procedural_resource_support;
use application::{procedural_resources::*, ApplicationError};
use domain::identity::Role;
use procedural_resource_support::*;

#[test]
fn audit_failure_rolls_back_registration_and_operation_can_be_retried() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let command = registration(&db);
    let draft = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    db.admin.batch_execute("CREATE FUNCTION reject_resource_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected resource audit failure'; END; $$; CREATE TRIGGER reject_resource_audit BEFORE INSERT ON audit_events FOR EACH ROW WHEN (NEW.action='procedural_resource.register') EXECUTE FUNCTION reject_resource_audit()").unwrap();
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.submit("session", db.case, command.clone(), draft.submission_digest),
        Err(ApplicationError::Port(_))
    ));
    assert_eq!(snapshot(&mut db), before);
    db.admin.batch_execute("DROP TRIGGER reject_resource_audit ON audit_events; DROP FUNCTION reject_resource_audit()").unwrap();
    let saved = workflow
        .submit("session", db.case, command.clone(), draft.submission_digest)
        .unwrap();
    assert_eq!(saved.receipt.operation_id, command.operation_id);
}

#[test]
fn duplicate_act_identity_cannot_append_a_second_act_or_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist(&workflow, db.case, registration(&db));
    let command = act(&first);
    let second = persist(&workflow, db.case, command.clone());
    let ResourceChange::RecordAct { act_id, values, .. } = command.change else {
        unreachable!()
    };
    let command = ResourceCommand {
        operation_id: ResourceOperationId::new(),
        resource_id: first.id,
        change: ResourceChange::RecordAct {
            expected_revision: second.revision,
            act_id,
            values,
        },
    };
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM case_procedural_resource_revisions",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(workflow.prepare("session", db.case, command).is_err());
    let after: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM case_procedural_resource_revisions",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(after, count);
}

#[test]
fn revocation_during_document_admission_prevents_commit_without_reserving_operation() {
    let Some(mut db) = Fixture::new() else { return };
    let actor = db.user("litigator", true);
    let command = registration(&db);
    let draft = service(&db, actor, Role::Litigator)
        .prepare("session", db.case, command.clone())
        .unwrap();
    let url = db.admin_url.clone();
    let format = case_stage_database_support::FormatCheck(Some(Box::new(move || {
        postgres::Client::connect(&url, postgres::NoTls)
            .unwrap()
            .execute(
                "DELETE FROM case_memberships WHERE user_id=$1",
                &[&actor.as_uuid()],
            )
            .unwrap();
    })));
    let workflow = service_with_format(&db, actor, Role::Litigator, std::sync::Arc::new(format));
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.submit("session", db.case, command.clone(), draft.submission_digest),
        Err(ApplicationError::CaseNotFound)
    ));
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .execute(
            "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)",
            &[&db.case.as_uuid(), &actor.as_uuid()],
        )
        .unwrap();
    let saved = service(&db, actor, Role::Litigator)
        .submit("session", db.case, command.clone(), draft.submission_digest)
        .unwrap();
    assert_eq!(saved.receipt.operation_id, command.operation_id);
}
