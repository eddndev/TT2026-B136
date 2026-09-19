mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
#[allow(dead_code)]
mod deadline_worker_backend_support;
mod deadline_worker_guard_support;
mod procedural_fact_backend_support;

use application::{
    deadline_reevaluation::TechnicalCause,
    deadline_worker::{DeadlineWorkerRun, DeadlineWorkerStore},
    deadlines::{DeadlineOperationId, DeadlineRevision},
};
use deadline_backend_support as dl;
use deadline_worker_backend_support as worker;
use deadline_worker_guard_support as guard;
use domain::crypto::Sha256Digest;
use uuid::Uuid;

#[test]
fn technical_revision_authenticates_job_operation_and_exact_durable_event() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let setup = guard::setup(&mut db);
    let before = worker::snapshot(&mut db);
    for mutation in 0..4 {
        let mut command = setup.command.clone();
        let TechnicalCause::SourceEvent {
            mut job_id,
            mut event,
        } = command.cause
        else {
            panic!("fixture requires a source-event job")
        };
        match mutation {
            0 => job_id = Uuid::from_u128(900_001),
            1 => command.operation_id = DeadlineOperationId::from_uuid(Uuid::from_u128(900_002)),
            2 => event.operation_id = Uuid::from_u128(900_003),
            3 => event.sequence = setup.later_sequence,
            _ => unreachable!(),
        }
        command.cause = TechnicalCause::SourceEvent { job_id, event };
        // The event is older than its head, so pure preparation cannot verify
        // its immutable operation. The real SQL guard must authenticate it.
        let forged = guard::prepare(&db, &setup, command);
        let mut runtime = db.runtime();
        let mut tx = runtime.transaction().unwrap();
        let error = guard::sql::insert_revision(&mut tx, &forged).unwrap_err();
        guard::assert_check(&error);
        drop(tx);
        assert_eq!(worker::snapshot(&mut db), before, "mutation {mutation}");
    }
    guard::complete_and_reopen(&mut db, &setup);
}

#[test]
fn neither_technical_revision_nor_revision_result_can_commit_alone() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let setup = guard::setup(&mut db);
    let before = worker::snapshot(&mut db);
    let mut runtime = db.runtime();
    let mut tx = runtime.transaction().unwrap();
    guard::sql::insert_revision(&mut tx, &setup.expected).unwrap();
    let visible: i64 = tx
        .query_one(
            "SELECT count(*) FROM case_deadline_revisions WHERE operation_id=$1",
            &[&setup.job.operation_id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(visible, 1, "the deferred check runs after a real insert");
    guard::assert_check(&tx.commit().unwrap_err());
    assert_eq!(worker::snapshot(&mut db), before);

    let mut tx = runtime.transaction().unwrap();
    let error =
        guard::sql::insert_result(&mut tx, setup.job.id, &setup.base, &setup.expected).unwrap_err();
    guard::assert_check(&error);
    drop(tx);
    assert_eq!(worker::snapshot(&mut db), before);
    guard::complete_and_reopen(&mut db, &setup);
}

#[test]
fn revision_result_requires_exact_base_and_produced_receipts() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let setup = guard::setup(&mut db);
    let before = worker::snapshot(&mut db);
    let wrong = Sha256Digest::from_bytes(&[17; 32]).unwrap();
    for mutation in 0..6 {
        let mut base = setup.base.clone();
        let mut next = setup.expected.clone();
        match mutation {
            0 => base.receipt.submission_digest = wrong,
            1 => base.receipt.capture_digest = wrong,
            2 => next.receipt.submission_digest = wrong,
            3 => next.receipt.capture_digest = wrong,
            4 => base.revision = DeadlineRevision::new(base.revision.get() + 1).unwrap(),
            5 => next.revision = DeadlineRevision::new(next.revision.get() + 1).unwrap(),
            _ => unreachable!(),
        }
        let mut runtime = db.runtime();
        let mut tx = runtime.transaction().unwrap();
        guard::sql::insert_revision(&mut tx, &setup.expected).unwrap();
        let error = guard::sql::insert_result(&mut tx, setup.job.id, &base, &next).unwrap_err();
        guard::assert_check(&error);
        drop(tx);
        assert_eq!(worker::snapshot(&mut db), before, "mutation {mutation}");
    }
    guard::complete_and_reopen(&mut db, &setup);
}

#[test]
fn completion_audit_failure_rolls_back_revision_and_result_but_records_retry() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let setup = guard::setup(&mut db);
    let runner = worker::open(&db);
    let before = guard::data_snapshot(&mut db);
    let audit_count = guard::audit_count(&mut db);
    guard::reject_completion_audit(&mut db);
    let DeadlineWorkerRun::Deferred(attempt) = runner.run_next().unwrap() else {
        panic!("a usable connection must durably record the completion failure")
    };
    assert!(guard::fault_reached(&mut db));
    assert_eq!(guard::data_snapshot(&mut db), before);
    assert!(runner.result(setup.job.id).unwrap().is_none());
    assert_eq!(attempt.job_id, setup.job.id);
    assert_eq!(attempt.attempt_number, 1);
    assert!(attempt.retry_at > attempt.failed_at);
    assert_eq!(
        runner.latest_attempt(setup.job.id).unwrap(),
        Some(attempt.clone())
    );
    guard::assert_attempt(&mut db, &setup, &attempt, audit_count);
    guard::allow_completion_audit(&mut db);

    let unchanged = worker::snapshot(&mut db);
    assert!(matches!(
        runner.run_next().unwrap(),
        DeadlineWorkerRun::Idle
    ));
    assert_eq!(worker::snapshot(&mut db), unchanged);
    let resumed = guard::open_at(&db, attempt.retry_at);
    assert!(matches!(
        resumed.run_next().unwrap(),
        DeadlineWorkerRun::Completed(_)
    ));
    let actual = worker::current(&db, setup.base.id);
    worker::assert_preserved(&setup.base, &actual);
    worker::assert_technical(&setup.base, &actual, &setup.job, setup.command.cause);
    assert_eq!(actual.recorded_at, attempt.retry_at);
    assert!(resumed.result(setup.job.id).unwrap().is_some());
    assert_eq!(resumed.latest_attempt(setup.job.id).unwrap(), Some(attempt));
    worker::assert_idle(&mut db, &resumed);
}
