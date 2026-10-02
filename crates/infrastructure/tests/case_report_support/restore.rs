use crate::case_report_support::*;
use application::case_reports::*;
use time::Duration;

#[test]
fn dump_restore_preserves_ready_bytes_notice_and_interrupted_capture_for_restart() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Confidential captured title", at);
    let actor = owner(&mut db);
    let timer = clock(at);
    let initial = store(&db, timer.clone());
    let ready = finish(&initial, &actor, command(at), at);
    let pdf = initial
        .download(&actor, ready.id, CaseReportFormat::Pdf, at)
        .unwrap();
    let csv = initial
        .download(&actor, ready.id, CaseReportFormat::Csv, at)
        .unwrap();
    let pending = request(&initial, &actor, command(at), at).unwrap();
    let claim = initial.claim_next(at).unwrap().unwrap();
    assert_eq!(claim.lease.report_id, pending.id);
    let snapshot = initial.capture(&claim.lease, at).unwrap();
    drop(initial);
    let before = ledger(&mut db);
    db.migrate();
    assert_eq!(ledger(&mut db), before);
    let temp = tempfile::tempdir().unwrap();
    let dump = temp.path().join("case-reports.dump");
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
    assert_eq!(ledger(&mut db), before);
    let later = claim.lease.expires_at + Duration::seconds(1);
    timer.set(later);
    let restored = store(&db, timer);
    assert_eq!(
        restored
            .download(&actor, ready.id, CaseReportFormat::Pdf, later)
            .unwrap(),
        pdf
    );
    assert_eq!(
        restored
            .download(&actor, ready.id, CaseReportFormat::Csv, later)
            .unwrap(),
        csv
    );
    let resumed = restored.claim_next(later).unwrap().unwrap();
    assert_eq!(resumed.lease.report_id, pending.id);
    assert_eq!(resumed.snapshot, Some(snapshot.clone()));
    restored
        .complete(&resumed.lease, &snapshot, artifacts(&snapshot), later)
        .unwrap();
    assert_eq!(count(&mut db, "case_report_notices"), 2);
    assert_eq!(count(&mut db, "case_report_artifacts"), 4);
}

#[test]
fn private_snapshot_and_artifact_payloads_are_not_stored_as_plaintext() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Secret report title marker", at);
    let actor = owner(&mut db);
    let store = store(&db, clock(at));
    finish(&store, &actor, command(at), at);
    for table in ["case_report_snapshots", "case_report_artifacts"] {
        let rows = db
            .admin
            .query(&format!("SELECT to_jsonb(r)::text FROM {table} r"), &[])
            .unwrap();
        assert!(!rows.is_empty());
        for row in rows {
            let serialized: String = row.get(0);
            assert!(!serialized.contains("Secret report title marker"));
            assert!(!serialized.contains("report fixture payload"));
            assert!(!serialized.contains("7265706f72742066697874757265207061796c6f6164"));
        }
    }
}
