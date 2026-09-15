mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;

use application::case_stages::*;
use application::cases::*;
use case_stage_database_support::*;
use domain::{cases::CaseId, identity::Role};

#[test]
fn full_dump_restore_preserves_union_snapshots_closed_history_and_inactive_captured_author() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let initial = CaseId::new();
    let original = db
        .store()
        .register_penal(db.owner, initial, creation("Initial"), db.at)
        .unwrap()
        .initial_stage
        .unwrap();
    let document = upload(&db, db.case, "adoption.pdf");
    let first = service(&db, db.owner, Role::Owner, FormatCheck(None))
        .adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &document, CaseStage::Intermediate),
        )
        .unwrap();
    let document = upload(&db, initial, "accusation.pdf");
    let second = service(&db, db.owner, Role::Owner, FormatCheck(None))
        .transition(
            "session",
            initial,
            CaseStageRevision::FIRST,
            StageTransition::to_intermediate(
                DeclaredStageTime::instant(db.at).unwrap(),
                reference(&document),
                None,
            ),
        )
        .unwrap();
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let reader = db.user("owner", false);
    db.admin
        .execute(
            "UPDATE users SET active=FALSE,email='changed@example.test' WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    let documents: serde_json::Value = db
        .admin
        .query_one(
            "SELECT jsonb_agg(to_jsonb(d) ORDER BY id,version) FROM documents d",
            &[],
        )
        .unwrap()
        .get(0);
    db.migrate();
    assert_eq!(snapshot(&mut db), before);
    let temp = tempfile::tempdir().unwrap();
    let dump = temp.path().join("stages.dump");
    let output = std::process::Command::new("pg_dump")
        .args([
            "--dbname",
            &db.admin_url,
            "--schema",
            &db.schema,
            "--format=custom",
            "--file",
        ])
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    let output = std::process::Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", &db.admin_url])
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(snapshot(&mut db), before);
    let restored: serde_json::Value = db
        .admin
        .query_one(
            "SELECT jsonb_agg(to_jsonb(d) ORDER BY id,version) FROM documents d",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(restored, documents);
    let stages = store(&db);
    assert_eq!(stages.get(reader, db.case, db.at).unwrap(), first);
    assert_eq!(stages.get(reader, initial, db.at).unwrap(), second);
    let history = stages
        .history(
            reader,
            initial,
            CaseStageQuery::new(10, None).unwrap(),
            db.at,
        )
        .unwrap();
    assert_eq!(history.entries[1], CaseStageEntry::Initial(original));
    assert_eq!(history.entries[0].recorded_by().email, "owner@example.test");
}

#[test]
fn canonical_checks_and_sequence_trigger_work_with_empty_restore_search_path() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let record = upload(&db, db.case, "support.pdf");
    let value = row_value(&db, &record);
    let signed = signed_value(&mut db.admin, &value);
    let mut runtime = db.runtime();
    runtime.batch_execute("SET search_path=''").unwrap();
    runtime.execute(&format!("INSERT INTO {0}.case_stage_revisions SELECT (jsonb_populate_record(NULL::{0}.case_stage_revisions,$1)).*",db.schema),&[&signed]).unwrap();
    let detail = store(&db).get(db.owner, db.case, db.at).unwrap();
    assert_eq!(detail.current.stage(), Some(CaseStage::Investigation));
}
