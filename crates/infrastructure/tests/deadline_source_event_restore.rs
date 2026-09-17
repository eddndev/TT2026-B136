mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_result_database_support;
mod judicial_calendar_database_support;
mod procedural_fact_backend_support;

use case_administration_support::Fixture;
use domain::identity::Role;
use hearing_database_support as hearings;
use hearing_result_database_support as results;
use judicial_calendar_database_support as calendars;
use procedural_fact_backend_support as facts;
use serde_json::Value;

#[test]
fn dump_restore_keeps_all_source_events_and_the_next_committed_sequence() {
    let Some(mut db) = Fixture::new() else { return };
    hearings::complete(&mut db);
    let hearing = hearings::persist(
        &hearings::service(&db, db.owner, Role::Owner),
        db.case,
        hearings::schedule(),
    );
    results::persist(
        &results::service(&db, db.owner, Role::Owner),
        db.case,
        results::record(hearing.snapshot.id),
    );
    let calendar_service = calendars::service(&db, db.owner, Role::Owner);
    let calendar = calendars::persist(&calendar_service, calendars::publish());
    calendars::persist(&calendar_service, calendars::retire(&calendar));
    let service = facts::service(&db, db.owner, Role::Owner);
    let resolution = facts::persist(&service, db.case, facts::record());
    let notice = facts::persist(
        &service,
        db.case,
        facts::notify(facts::resolution_ref(&resolution)),
    );
    facts::persist(&service, db.case, facts::withdraw(&notice));
    let before = snapshot(&mut db);
    let old_sequence = sequence_state(&mut db);
    db.migrate();
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(sequence_state(&mut db), old_sequence);
    dump_restore(&mut db);
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(sequence_state(&mut db), old_sequence);
    db.store();
    let changed = facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        facts::correct(&resolution),
    );
    let row = db
        .admin
        .query_one(
            "SELECT sequence,revision FROM deadline_source_events WHERE operation_id=$1",
            &[&changed.snapshot.metadata().receipt.operation_id.as_uuid()],
        )
        .unwrap();
    assert!(row.get::<_, i64>(0) > old_sequence.0);
    assert_eq!(row.get::<_, i64>(1), 2);
}

#[test]
fn startup_inventory_rejects_an_event_with_a_forged_operation_on_a_valid_source() {
    let Some(mut db) = Fixture::new() else { return };
    facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        facts::record(),
    );
    db.store();
    db.admin
        .batch_execute("ALTER TABLE deadline_source_events DISABLE TRIGGER USER")
        .unwrap();
    db.admin
        .execute(
            "UPDATE deadline_source_events SET operation_id=$1",
            &[&uuid::Uuid::new_v4()],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE deadline_source_events ENABLE TRIGGER USER")
        .unwrap();
    assert!(infrastructure::PostgresCaseRepository::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher)
    )
    .is_err());
}

#[test]
fn startup_inventory_rejects_a_sequence_behind_committed_events() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let first = facts::persist(&workflow, db.case, facts::record());
    facts::persist(&workflow, db.case, facts::correct(&first));
    db.store();
    db.admin
        .batch_execute("SELECT setval('deadline_source_events_sequence',1,FALSE)")
        .unwrap();
    assert!(infrastructure::PostgresCaseRepository::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher)
    )
    .is_err());
}

fn snapshot(db: &mut Fixture) -> Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'events',(SELECT jsonb_agg(to_jsonb(e) ORDER BY sequence) FROM deadline_source_events e),
        'facts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY family,id,revision) FROM case_procedural_fact_revisions r),
        'results',(SELECT jsonb_agg(to_jsonb(r) ORDER BY result_id,revision) FROM case_hearing_result_revisions r),
        'calendars',(SELECT jsonb_agg(to_jsonb(r) ORDER BY calendar_id,revision) FROM judicial_calendar_revisions r),
        'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
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
    let dump = directory.path().join("deadline-source-events.dump");
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
