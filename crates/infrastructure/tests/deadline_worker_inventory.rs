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
#[allow(dead_code)]
mod deadline_worker_guard_support;
#[allow(dead_code)]
mod deadline_worker_retry_support;
mod procedural_fact_backend_support;

use application::{deadline_worker::DeadlineWorkerStore, ApplicationError};
use case_stage_database_support::FixedClock;
use deadline_backend_support as dl;
use deadline_worker_backend_support as worker;
use deadline_worker_guard_support as guard;
use deadline_worker_retry_support as retry;
use infrastructure::{PostgresDeadlineWorkerStore, RingSha256Hasher};
use std::sync::Arc;

#[test]
fn startup_rejects_orphaned_technical_revisions_and_altered_completion_receipts() {
    for remove_result in [true, false] {
        let Some(mut db) = dl::Fixture::new() else {
            return;
        };
        let setup = guard::setup(&mut db);
        let runner = worker::open(&db);
        let completed = retry::completed(runner.run_next().unwrap());
        assert_eq!(completed.job_id, setup.job.id);
        assert_eq!(runner.result(setup.job.id).unwrap(), Some(*completed));
        drop(open(&db).unwrap());
        let original = worker::result_row(&mut db, &setup.job);

        let mut tx = db.admin.transaction().unwrap();
        tx.batch_execute(
            "ALTER TABLE deadline_reevaluation_results DISABLE TRIGGER deadline_worker_result_immutable",
        )
        .unwrap();
        let affected = if remove_result {
            tx.execute(
                "DELETE FROM deadline_reevaluation_results WHERE job_id=$1",
                &[&setup.job.id],
            )
            .unwrap()
        } else {
            let wrong = vec![19_u8; 32];
            tx.execute(
                "UPDATE deadline_reevaluation_results SET result_capture_digest=$2 WHERE job_id=$1",
                &[&setup.job.id, &wrong],
            )
            .unwrap()
        };
        assert_eq!(affected, 1);
        tx.batch_execute(
            "ALTER TABLE deadline_reevaluation_results ENABLE TRIGGER deadline_worker_result_immutable",
        )
        .unwrap();
        tx.commit().unwrap();
        assert_guards_enabled(&mut db);
        let before = retry::snapshot(&mut db);
        if remove_result {
            assert_eq!(worker::result_count(&mut db), 0);
        } else {
            assert_ne!(worker::result_row(&mut db, &setup.job), original);
            assert!(runner.result(setup.job.id).is_err());
        }
        assert!(open(&db).is_err(), "startup accepted changed completion");
        assert_eq!(retry::snapshot(&mut db), before);
    }
}

#[test]
fn startup_reads_every_historical_attempt_even_when_the_latest_attempt_is_valid() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let setup = guard::setup(&mut db);
    let initial = worker::open(&db);
    guard::reject_completion_audit(&mut db);
    let first = retry::deferred(initial.run_next().unwrap());
    assert!(guard::fault_reached(&mut db));
    guard::allow_completion_audit(&mut db);
    assert_eq!(first.attempt_number, 1);
    drop(initial);

    let runner = guard::open_at(&db, first.retry_at);
    guard::reject_completion_audit(&mut db);
    let second = retry::deferred(runner.run_next().unwrap());
    assert!(guard::fault_reached(&mut db));
    guard::allow_completion_audit(&mut db);
    assert_eq!(second.attempt_number, 2);
    assert_eq!(second.job_id, first.job_id);
    assert_eq!(second.job_id, setup.job.id);
    assert_ne!(second.attempt_id, first.attempt_id);
    assert_eq!(
        runner.latest_attempt(setup.job.id).unwrap(),
        Some(second.clone())
    );
    assert!(runner.result(setup.job.id).unwrap().is_none());
    drop(open(&db).unwrap());
    let original = retry::attempt_row(&mut db, &first);
    let saved_digest = first
        .checked_base
        .as_ref()
        .unwrap()
        .receipt
        .capture_digest
        .as_bytes()
        .to_vec();

    replace_attempt_capture(&mut db, first.attempt_id, &[23; 32]);
    assert_guards_enabled(&mut db);
    assert_ne!(retry::attempt_row(&mut db, &first), original);
    let before = retry::snapshot(&mut db);
    assert_eq!(
        runner.latest_attempt(setup.job.id).unwrap(),
        Some(second.clone())
    );
    assert!(
        open(&db).is_err(),
        "startup ignored a corrupt older attempt"
    );
    assert_eq!(retry::snapshot(&mut db), before);

    replace_attempt_capture(&mut db, first.attempt_id, &saved_digest);
    assert_eq!(retry::attempt_row(&mut db, &first), original);
    let restored = open(&db).unwrap();
    assert_eq!(restored.latest_attempt(setup.job.id).unwrap(), Some(second));
}

fn open(db: &dl::Fixture) -> Result<PostgresDeadlineWorkerStore, ApplicationError> {
    PostgresDeadlineWorkerStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

fn replace_attempt_capture(db: &mut dl::Fixture, id: uuid::Uuid, digest: &[u8]) {
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute(
        "ALTER TABLE deadline_reevaluation_attempts DISABLE TRIGGER deadline_worker_attempt_immutable",
    )
    .unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE deadline_reevaluation_attempts SET checked_base_capture_digest=$2 WHERE attempt_id=$1",
            &[&id, &digest],
        )
        .unwrap(),
        1
    );
    tx.batch_execute(
        "ALTER TABLE deadline_reevaluation_attempts ENABLE TRIGGER deadline_worker_attempt_immutable",
    )
    .unwrap();
    tx.commit().unwrap();
}

fn assert_guards_enabled(db: &mut dl::Fixture) {
    let enabled: bool = db
        .admin
        .query_one(
            "SELECT bool_and(tgenabled IN ('O','A')) FROM pg_trigger
        WHERE tgrelid IN ('deadline_reevaluation_results'::regclass,
            'deadline_reevaluation_attempts'::regclass,'case_deadline_revisions'::regclass)",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(enabled);
}
