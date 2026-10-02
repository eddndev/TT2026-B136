use crate::case_report_support::*;
use application::{case_reports::*, ApplicationError};

#[test]
fn request_audit_failure_rolls_back_the_queued_job_before_replay() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let actor = owner(&mut db);
    let store = store(&db, clock(at));
    let command = command(at);
    let before = ledger(&mut db);
    reject_audit(
        &mut db,
        "case_report.request",
        "(SELECT count(*) FROM case_report_jobs)=1",
    );
    assert!(request(&store, &actor, command.clone(), at).is_err());
    assert!(fault_reached(&mut db));
    assert_eq!(ledger(&mut db), before);
    allow_audit(&mut db);
    request(&store, &actor, command, at).unwrap();
    assert_eq!(count(&mut db, "case_report_jobs"), 1);
    assert_eq!(audits(&mut db, "case_report.request"), 1);
}

#[test]
fn capture_audit_failure_cannot_leave_a_snapshot_or_advance_the_job() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Atomic capture", at);
    let actor = owner(&mut db);
    let store = store(&db, clock(at));
    request(&store, &actor, command(at), at).unwrap();
    let claim = store.claim_next(at).unwrap().unwrap();
    let before = ledger(&mut db);
    reject_audit(
        &mut db,
        "case_report.capture",
        "(SELECT count(*) FROM case_report_snapshots)=1",
    );
    assert!(store.capture(&claim.lease, at).is_err());
    assert!(fault_reached(&mut db));
    assert_eq!(ledger(&mut db), before);
    allow_audit(&mut db);
    let captured = store.capture(&claim.lease, at).unwrap();
    assert_eq!(captured.cases.len(), 1);
    assert_eq!(count(&mut db, "case_report_snapshots"), 1);
    assert_eq!(audits(&mut db, "case_report.capture"), 1);
}

#[test]
fn ready_both_artifacts_and_notice_rollback_when_the_last_audit_insert_fails() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Private report content", at);
    let actor = owner(&mut db);
    let store = store(&db, clock(at));
    let report = request(&store, &actor, command(at), at).unwrap();
    let claim = store.claim_next(at).unwrap().unwrap();
    let snapshot = store.capture(&claim.lease, at).unwrap();
    let before = ledger(&mut db);
    reject_audit(&mut db, "case_report.complete",
        "(SELECT count(*) FROM case_report_artifacts)=2 AND (SELECT count(*) FROM case_report_notices)=1");
    assert!(store
        .complete(&claim.lease, &snapshot, artifacts(&snapshot), at)
        .is_err());
    assert!(fault_reached(&mut db));
    assert_eq!(ledger(&mut db), before);
    allow_audit(&mut db);
    assert!(matches!(
        store.download(&actor, report.id, CaseReportFormat::Csv, at),
        Err(ApplicationError::CaseReport(CaseReportError::NotReady))
    ));
    let ready = store
        .complete(&claim.lease, &snapshot, artifacts(&snapshot), at)
        .unwrap();
    assert!(matches!(ready.state, CaseReportState::Ready { .. }));
    for expected in artifacts(&snapshot) {
        let received = store
            .download(&actor, report.id, expected.format, at)
            .unwrap();
        assert_eq!(received.artifact, expected);
        assert_eq!(received.report, ready);
    }
    assert_eq!(count(&mut db, "case_report_artifacts"), 2);
    assert_eq!(count(&mut db, "case_report_notices"), 1);
    assert_eq!(audits(&mut db, "case_report.complete"), 1);
    assert!(store
        .complete(&claim.lease, &snapshot, artifacts(&snapshot), at)
        .is_err());
    assert_eq!(count(&mut db, "case_report_notices"), 1);
}

#[test]
fn mismatched_or_missing_artifact_cannot_commit_any_output() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Paired artifact", at);
    let actor = owner(&mut db);
    let store = store(&db, clock(at));
    request(&store, &actor, command(at), at).unwrap();
    let claim = store.claim_next(at).unwrap().unwrap();
    let snapshot = store.capture(&claim.lease, at).unwrap();
    let expected = artifacts(&snapshot);
    let mut wrong_digest = expected.clone();
    wrong_digest[1].snapshot_digest = domain::crypto::Sha256Digest::from_array([0; 32]);
    let invalid = [
        vec![expected[0].clone()],
        vec![expected[0].clone(), expected[0].clone()],
        wrong_digest,
    ];
    let before = ledger(&mut db);
    for files in invalid {
        assert!(store.complete(&claim.lease, &snapshot, files, at).is_err());
        assert_eq!(ledger(&mut db), before);
    }
    store
        .complete(&claim.lease, &snapshot, expected, at)
        .unwrap();
}
