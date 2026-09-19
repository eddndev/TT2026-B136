mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod procedural_fact_backend_support;
mod procedural_resource_support;
mod resource_activity_support;
use application::resource_activities::*;
use domain::identity::Role;
use resource_activity_support::*;

#[test]
fn dump_restore_preserves_exact_associations_sources_and_authors_and_allows_new_links() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let linked = persist(&workflow, db.case, captures.resource.id, captures.link());
    let unlinked = persist(
        &workflow,
        db.case,
        captures.resource.id,
        unlink(&linked, captures.head.revision),
    );
    drop(workflow);
    let reader = db.user("owner", false);
    db.admin
        .execute(
            "UPDATE users SET active=FALSE,email='former@example.test' WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let before = business_and_audit(&mut db);
    let independent = independent_rows(&mut db);
    db.migrate();
    assert_eq!(business_and_audit(&mut db), before);
    dump_restore(&mut db);
    assert_eq!(business_and_audit(&mut db), before);
    assert_eq!(independent_rows(&mut db), independent);
    let adapter = store(&db);
    for expected in [&linked, &unlinked] {
        let actual = adapter
            .get(
                reader,
                db.case,
                captures.resource.id,
                linked.id,
                Some(expected.revision),
                db.at,
            )
            .unwrap();
        assert_eq!(actual.association, *expected);
        assert_eq!(actual.association.recorded_by.email, "owner@example.test");
        assert_eq!(actual.association.sources.resource, captures.resource);
        assert_eq!(actual.association.sources.act, Some(captures.act.clone()));
    }
    assert_eq!(
        adapter
            .history(
                reader,
                db.case,
                captures.resource.id,
                linked.id,
                history_query(),
                db.at
            )
            .unwrap()
            .revisions,
        vec![unlinked, linked]
    );
    let replacement = persist(
        &service(&db, reader, Role::Owner),
        db.case,
        captures.resource.id,
        captures.link(),
    );
    assert_eq!(replacement.recorded_by.id, reader);
    assert_eq!(replacement.revision, ResourceActivityRevision::initial());
    assert_eq!(independent_rows(&mut db), independent);
}

fn dump_restore(db: &mut Fixture) {
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("resource-activities.dump");
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
