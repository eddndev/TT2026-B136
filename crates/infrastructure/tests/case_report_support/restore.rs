use crate::case_report_support::*;
use application::{
    case_reports::*,
    cases::CaseRepository,
    documents::{CaseDocumentStore, DocumentRecord},
};
use domain::{
    crypto::{DocumentId, DocumentVersion, Sha256Digest},
    identity::UserId,
};
use infrastructure::PostgresCaseDocumentStore;
use time::Duration;

#[test]
fn dump_restore_preserves_ready_bytes_notice_and_interrupted_capture_for_restart() {
    restore_reports(CaseReportKind::CaseState);
}

#[test]
fn dump_restore_preserves_activity_author_after_reassignment_and_exact_report_replay() {
    restore_reports(CaseReportKind::LitigatorActivity);
}

fn restore_reports(kind: CaseReportKind) {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    db.case = seed_case(&mut db, "Confidential captured title", at);
    let original_author =
        (kind == CaseReportKind::LitigatorActivity).then(|| upload_then_reassign(&mut db));
    let actor = owner(&mut db);
    let timer = clock(at);
    let initial = store(&db, timer.clone());
    let mut ready_command = command(at);
    ready_command.filters.kind = kind;
    let ready = finish(&initial, &actor, ready_command.clone(), at);
    let pdf = initial
        .download(&actor, ready.id, CaseReportFormat::Pdf, at)
        .unwrap();
    let csv = initial
        .download(&actor, ready.id, CaseReportFormat::Csv, at)
        .unwrap();
    let mut pending_command = command(at);
    pending_command.filters.kind = kind;
    let pending = request(&initial, &actor, pending_command, at).unwrap();
    let claim = initial.claim_next(at).unwrap().unwrap();
    assert_eq!(claim.lease.report_id, pending.id);
    let snapshot = initial.capture(&claim.lease, at).unwrap();
    assert_eq!(snapshot.filters.kind, kind);
    if let Some(author) = original_author {
        let activity = snapshot.activity.as_ref().unwrap();
        assert!(activity.documents_complete);
        assert_eq!(activity.rows.len(), 1);
        assert_eq!(activity.rows[0].case_id, db.case);
        assert_eq!(activity.rows[0].litigator_id, author);
        assert_eq!(activity.rows[0].documents_uploaded, 1);
        assert_eq!(activity.rows[0].procedural_activities, 0);
        assert_eq!(activity.rows[0].deadlines_attended, 0);
        let who = activity
            .actors
            .iter()
            .find(|who| who.user_id == author)
            .unwrap();
        assert_eq!(who.email, format!("{author}@example.test"));
    } else {
        assert!(snapshot.activity.is_none());
    }
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
        request(&restored, &actor, ready_command, later).unwrap(),
        ready
    );
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

fn upload_then_reassign(db: &mut Fixture) -> UserId {
    let author = db.user("litigator", true);
    let replacement = db.user("litigator", false);
    let document = DocumentRecord::pending(
        DocumentId::new(),
        DocumentVersion::initial(),
        "activity-restore.txt".into(),
        Sha256Digest::from_array([3; 32]),
        vec![8; 80],
    )
    .unwrap();
    PostgresCaseDocumentStore::open(&db.runtime_url)
        .unwrap()
        .insert(author, db.case, document, db.at)
        .unwrap();
    let cases = db.store();
    cases
        .remove_member(db.case, author, db.owner, db.at)
        .unwrap();
    cases
        .add_member(db.case, replacement, db.owner, db.at)
        .unwrap();
    author
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
