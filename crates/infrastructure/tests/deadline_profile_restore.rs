mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_profile_database_support;

use application::deadline_profiles::*;
use deadline_profile_database_support::*;
use domain::identity::Role;

#[test]
fn dump_restore_preserves_global_and_private_history_and_continues_the_event_sequence() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let mut histories = Vec::new();
    for case in [None, Some(db.case)] {
        let collection = case
            .map(DeadlineProfileCollection::ForCase)
            .unwrap_or(DeadlineProfileCollection::Global);
        let first = persist(&workflow, collection, publish(case));
        let second = persist(&workflow, collection, replace(&first));
        let third = persist(&workflow, collection, retire(&second));
        histories.push((collection, vec![first, second, third]));
    }
    let collection = DeadlineProfileCollection::ForCase(db.case);
    let active = persist(&workflow, collection, publish(Some(db.case)));
    let reader = db.user("owner", false);
    db.admin
        .execute(
            "UPDATE users SET active=FALSE,email='former@example.test' WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    let old_sequence = sequence_state(&mut db);
    assert!(old_sequence.1);
    db.migrate();
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(sequence_state(&mut db), old_sequence);
    dump_restore(&mut db);
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(sequence_state(&mut db), old_sequence);
    let restored = store(&db);
    for (collection, revisions) in &histories {
        for exact in revisions {
            assert_eq!(
                restored
                    .get(reader, *collection, exact.id, Some(exact.revision), db.at)
                    .unwrap(),
                *exact
            );
            assert_eq!(exact.recorded_by.id, db.owner);
            assert_eq!(exact.recorded_by.email, "owner@example.test");
        }
        let latest = revisions.last().unwrap();
        assert_eq!(
            restored
                .get(reader, *collection, latest.id, None, db.at)
                .unwrap(),
            *latest
        );
        assert_eq!(
            restored
                .history(
                    reader,
                    *collection,
                    latest.id,
                    DeadlineProfileHistoryQuery::new(20, None).unwrap(),
                    db.at
                )
                .unwrap()
                .revisions,
            revisions
                .iter()
                .rev()
                .map(DeadlineProfileHistoryEntry::from)
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(
        restored
            .get(reader, collection, active.id, Some(active.revision), db.at)
            .unwrap(),
        active
    );
    let successor = persist(
        &service(&db, reader, Role::Owner),
        collection,
        replace(&active),
    );
    assert_eq!(successor.revision.get(), 2);
    assert_eq!(successor.recorded_by.id, reader);
    let row = db.admin.query_one("SELECT sequence,source_kind,source_id,revision,case_id,hearing_id FROM deadline_source_events WHERE operation_id=$1",
        &[&successor.receipt.operation_id.as_uuid()]).unwrap();
    assert_eq!(row.get::<_, i64>(0), old_sequence.0 + 1);
    assert_eq!(row.get::<_, String>(1), "profile");
    assert_eq!(row.get::<_, uuid::Uuid>(2), successor.id.as_uuid());
    assert_eq!(row.get::<_, i64>(3), 2);
    assert_eq!(row.get::<_, Option<uuid::Uuid>>(4), Some(db.case.as_uuid()));
    assert!(row.get::<_, Option<uuid::Uuid>>(5).is_none());
}

fn sequence_state(db: &mut Fixture) -> (i64, bool) {
    let row = db
        .admin
        .query_one(
            "SELECT last_value,is_called FROM deadline_source_events_sequence",
            &[],
        )
        .unwrap();
    (row.get(0), row.get(1))
}
fn dump_restore(db: &mut Fixture) {
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("deadline-profiles.dump");
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
