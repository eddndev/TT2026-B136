mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod judicial_calendar_database_support;
use application::judicial_calendars::*;
use domain::identity::Role;
use judicial_calendar_database_support::*;

#[test]
fn dump_restore_preserves_all_calendar_revisions_receipts_and_original_author() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist(&workflow, publish());
    let second = persist(&workflow, replace(&first));
    let third = persist(&workflow, retire(&second));
    let separate = persist(&workflow, publish());
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
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("calendars.dump");
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
    assert_eq!(snapshot(&mut db), before);
    let restored = store(&db);
    for exact in [&first, &second, &third, &separate] {
        assert_eq!(
            restored
                .get(reader, exact.id, Some(exact.revision), db.at)
                .unwrap(),
            *exact
        );
    }
    let successor = persist(&service(&db, reader, Role::Owner), replace(&separate));
    assert_eq!(successor.revision.get(), 2);
    assert_eq!(successor.recorded_by.id, reader);
}
fn run(command: &mut std::process::Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
