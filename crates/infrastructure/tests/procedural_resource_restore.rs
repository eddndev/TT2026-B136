mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;
mod procedural_resource_support;
use application::procedural_resources::*;
use domain::identity::Role;
use procedural_resource_support::*;

#[test]
fn restore_keeps_resource_act_receipts_and_original_author_and_can_append() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist(&workflow, db.case, registration(&db));
    let second = persist(&workflow, db.case, act(&first));
    let archived = persist(
        &workflow,
        db.case,
        ResourceCommand {
            operation_id: ResourceOperationId::new(),
            resource_id: first.id,
            change: ResourceChange::Archive {
                expected_revision: second.revision,
                reason: text("Organizational archive"),
            },
        },
    );
    drop(workflow);
    let reader = db.user("owner", false);
    db.admin
        .execute(
            "UPDATE users SET active=FALSE,email='former@example.test' WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    db.migrate();
    assert_eq!(snapshot(&mut db), before);
    dump_restore(&mut db);
    assert_eq!(snapshot(&mut db), before);
    let restored = store(&db);
    for detail in [&first, &second, &archived] {
        assert_eq!(
            restored
                .get(reader, db.case, first.id, Some(detail.revision), db.at)
                .unwrap(),
            *detail
        );
        assert_eq!(detail.recorded_by.id, db.owner);
        assert_eq!(detail.recorded_by.email, "owner@example.test");
    }
    assert_eq!(
        restored
            .history(
                reader,
                db.case,
                first.id,
                ResourceHistoryQuery::new(20, None).unwrap(),
                db.at
            )
            .unwrap()
            .revisions,
        vec![archived.clone(), second, first.clone()]
    );
    let successor = persist(
        &service(&db, reader, Role::Owner),
        db.case,
        ResourceCommand {
            operation_id: ResourceOperationId::new(),
            resource_id: first.id,
            change: ResourceChange::Reactivate {
                expected_revision: archived.revision,
                reason: text("Resume organization"),
            },
        },
    );
    assert_eq!(successor.revision.get(), 4);
    assert_eq!(successor.recorded_by.id, reader);
    assert_eq!(successor.sources, first.sources);
    assert_eq!(
        successor.receipt.previous.unwrap().capture_digest,
        archived.receipt.capture_digest
    );
}

fn dump_restore(db: &mut Fixture) {
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("procedural-resources.dump");
    run(std::process::Command::new("pg_dump")
        .args([
            "--dbname",
            &db.admin_url,
            "--schema",
            &db.schema,
            "--format=custom",
            "--file",
        ])
        .arg(&dump));
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    run(std::process::Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", &db.admin_url])
        .arg(&dump));
}
fn run(command: &mut std::process::Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
