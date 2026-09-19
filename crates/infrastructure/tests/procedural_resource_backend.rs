mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;
mod procedural_resource_support;
use application::{procedural_resources::*, ApplicationError};
use domain::identity::Role;
use infrastructure::RingSha256Hasher;
use procedural_resource_support::*;

#[test]
fn registration_act_and_history_keep_exact_receipts_and_original_sources() {
    let Some(db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let command = registration(&db);
    let first = persist(&workflow, db.case, command);
    assert_eq!(first.recorded_by.email, "owner@example.test");
    assert_eq!(first.recorded_at, db.at);
    resource_receipt_matches(&RingSha256Hasher, &first).unwrap();
    let second = persist(&workflow, db.case, act(&first));
    assert_eq!(second.revision.get(), 2);
    assert_eq!(second.sources, first.sources);
    assert_eq!(second.act.as_ref().unwrap().revision.get(), 1);
    let old_act = second.act.as_ref().unwrap();
    let corrected = persist(
        &workflow,
        db.case,
        ResourceCommand {
            operation_id: ResourceOperationId::new(),
            resource_id: first.id,
            change: ResourceChange::CorrectAct {
                expected_revision: second.revision,
                act_id: old_act.id,
                expected_act_revision: old_act.revision,
                values: old_act.values.clone(),
                reason: text("Correct declared act"),
            },
        },
    );
    assert_eq!(corrected.act.as_ref().unwrap().revision.get(), 2);
    assert_eq!(corrected.act.as_ref().unwrap().supports, old_act.supports);
    let archived = persist(
        &workflow,
        db.case,
        ResourceCommand {
            operation_id: ResourceOperationId::new(),
            resource_id: first.id,
            change: ResourceChange::Archive {
                expected_revision: corrected.revision,
                reason: text("Organizational archive"),
            },
        },
    );
    assert_eq!(archived.status, ResourceStatus::Archived);
    assert!(archived.act.is_none());
    let history = workflow
        .history(
            "session",
            db.case,
            first.id,
            ResourceHistoryQuery::new(20, None).unwrap(),
        )
        .unwrap();
    assert_eq!(
        history.revisions,
        vec![archived, corrected, second, first.clone()]
    );
    assert_eq!(
        workflow
            .get("session", db.case, first.id, Some(first.revision))
            .unwrap(),
        first
    );
    assert!(workflow
        .list(
            "session",
            db.case,
            ResourceQuery::new(1, None, None, Some(ResourceStatus::Active)).unwrap()
        )
        .unwrap()
        .resources
        .is_empty());
}

#[test]
fn exact_operation_replays_without_new_revision_and_different_command_conflicts() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let command = registration(&db);
    let saved = persist(&workflow, db.case, command.clone());
    assert_eq!(
        workflow
            .submit(
                "session",
                db.case,
                command.clone(),
                saved.receipt.submission_digest
            )
            .unwrap(),
        saved
    );
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM case_procedural_resource_revisions",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
    let mut conflict = command;
    conflict.resource_id = ResourceId::new();
    assert!(matches!(
        workflow.prepare("session", db.case, conflict),
        Err(ApplicationError::ProceduralResource(
            ProceduralResourceError::OperationConflict
        ))
    ));
}

#[test]
fn current_membership_and_role_are_checked_before_protected_reads_and_writes() {
    let Some(mut db) = Fixture::new() else { return };
    let command = registration(&db);
    let saved = persist(
        &service(&db, db.owner, Role::Owner),
        db.case,
        command.clone(),
    );
    let paralegal = db.user("paralegal", true);
    let reader = service(&db, paralegal, Role::Paralegal);
    assert_eq!(
        reader.get("session", db.case, saved.id, None).unwrap(),
        saved
    );
    assert!(matches!(
        reader.prepare("session", db.case, act(&saved)),
        Err(ApplicationError::PermissionDenied)
    ));
    let client = db.user("client", true);
    assert!(matches!(
        service(&db, client, Role::Client).get("session", db.case, saved.id, None),
        Err(ApplicationError::PermissionDenied)
    ));
    let adapter = store(&db);
    assert!(matches!(
        adapter.get(client, db.case, saved.id, None, db.at),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        adapter.prepare(
            paralegal,
            db.case,
            &act(&saved),
            &application::documents::StageSupportReadLimits::standard()
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE user_id=$1",
            &[&paralegal.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        reader.get("session", db.case, saved.id, None),
        Err(ApplicationError::CaseNotFound)
    ));
}
