mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;
use application::{cases::*, procedural_facts::*};
use domain::identity::Role;
use procedural_fact_backend_support::*;

#[test]
fn dump_restore_preserves_exact_fact_history_after_administration_and_author_change() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let resolution = persist(&workflow, db.case, record());
    let notification = persist(&workflow, db.case, notify(resolution_ref(&resolution)));
    let corrected = persist(&workflow, db.case, correct(&resolution));
    let withdrawn = persist(&workflow, db.case, withdraw(&corrected));
    let notification_corrected = persist(&workflow, db.case, correct(&notification));
    let notification_withdrawn = persist(&workflow, db.case, withdraw(&notification_corrected));
    let reader = db.user("owner", false);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(0),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    db.admin
        .execute(
            "UPDATE users SET active=FALSE,email='former@example.test' WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    let case_before = db.snapshot();
    db.migrate();
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(db.snapshot(), case_before);
    dump_restore(&mut db);
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(db.snapshot(), case_before);
    let restored = store(&db);
    for exact in [
        &resolution,
        &notification,
        &corrected,
        &withdrawn,
        &notification_corrected,
        &notification_withdrawn,
    ] {
        assert_eq!(
            restored
                .get(
                    reader,
                    db.case,
                    exact.snapshot.target(),
                    Some(exact.snapshot.metadata().revision),
                    db.at
                )
                .unwrap(),
            *exact
        );
    }
    db.store()
        .change_administrative_status(
            reader,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Active,
            db.at,
        )
        .unwrap();
    let new = persist(
        &service(&db, reader, Role::Owner),
        db.case,
        notify(resolution_ref(&withdrawn)),
    );
    assert_eq!(
        new.sources.resolved.resolution.unwrap().status,
        FactStatus::Withdrawn
    );
    assert_eq!(
        new.snapshot
            .metadata()
            .recorded_administration
            .revision()
            .unwrap()
            .get(),
        2
    );
}
fn dump_restore(db: &mut Fixture) {
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("procedural-facts.dump");
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
