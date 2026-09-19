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
#[path = "deadline_worker_extra_support/recovery.rs"]
mod recovery;

use application::{deadline_worker::*, deadlines::DeadlineAction};
use deadline_backend_support as dl;
use deadline_dispatch_support as dispatch;
use deadline_worker_backend_support as worker;
use deadline_worker_guard_support as guard;
use deadline_worker_retry_support as retry;
use time::Duration;

#[test]
fn repeated_transient_failures_use_persisted_exponential_delays_capped_at_five_minutes() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (source, base) = worker::accepted(&mut db, 830);
    dispatch::advance(&db, &source);
    retry::dispatch_events(&db, 1);
    let jobs = worker::jobs(&mut db);
    assert_eq!(jobs.len(), 1);
    let job = &jobs[0];
    let (runner, clock) = recovery::controlled(&db);
    let unchanged = dispatch::deadline_history(&mut db);
    guard::reject_completion_audit(&mut db);
    let mut at = db.at;
    let mut last = None;
    for (index, seconds) in [1, 2, 4, 8, 16, 32, 64, 128, 256, 300, 300]
        .into_iter()
        .enumerate()
    {
        let attempt = retry::deferred(runner.run_next().unwrap());
        assert_eq!(attempt.job_id, job.id);
        assert_eq!(attempt.attempt_number, u64::try_from(index + 1).unwrap());
        assert_eq!(attempt.failure_kind, DeadlineWorkerFailureKind::Transient);
        assert_eq!(attempt.error_code, DeadlineWorkerErrorCode::ExecutionFailed);
        assert_eq!(attempt.failed_at, at);
        assert_eq!(attempt.retry_at, at + Duration::seconds(seconds));
        let checked = attempt.checked_base.as_ref().unwrap();
        assert_eq!(checked.revision, base.revision);
        assert_eq!(checked.receipt.capture_digest, base.receipt.capture_digest);
        assert_eq!(
            checked.receipt.submission_digest,
            base.receipt.submission_digest
        );
        assert_eq!(
            runner.latest_attempt(job.id).unwrap(),
            Some(attempt.clone())
        );
        assert!(runner.result(job.id).unwrap().is_none());
        assert_eq!(dispatch::deadline_history(&mut db), unchanged);
        retry::assert_counts(&mut db, 0, 0, i64::try_from(index + 1).unwrap());
        let probes: i64 = db
            .admin
            .query_one("SELECT last_value FROM worker_fault_reached", &[])
            .unwrap()
            .get(0);
        assert_eq!(probes, i64::try_from(index + 1).unwrap());
        clock.set(attempt.retry_at - Duration::nanoseconds(1));
        retry::assert_idle(&mut db, &runner);
        let after: i64 = db
            .admin
            .query_one("SELECT last_value FROM worker_fault_reached", &[])
            .unwrap()
            .get(0);
        assert_eq!(after, probes, "not-yet-due work must not execute");
        at = attempt.retry_at;
        clock.set(at);
        last = Some(attempt);
    }
    let attempts = retry::snapshot(&mut db)["attempts"].clone();
    guard::allow_completion_audit(&mut db);
    let completed = retry::completed(runner.run_next().unwrap());
    assert_eq!(completed.completed_at, at);
    let next = worker::current(&db, base.id);
    assert_eq!(next.receipt.action, DeadlineAction::Reevaluate);
    retry::assert_revision_result(&completed, job, &base, &next);
    assert_eq!(runner.latest_attempt(job.id).unwrap(), last);
    assert_eq!(retry::snapshot(&mut db)["attempts"], attempts);
    retry::assert_counts(&mut db, 1, 1, 11);
    drop(runner);
    let reopened = guard::open_at(&db, at);
    assert_eq!(
        reopened.result(job.id).unwrap().as_ref(),
        Some(completed.as_ref())
    );
    assert_eq!(reopened.latest_attempt(job.id).unwrap(), last);
    retry::assert_idle(&mut db, &reopened);
}

#[test]
fn dump_restore_preserves_completed_results_and_pending_attempts_before_retry_without_migration() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let bases = retry::pair(&mut db);
    let jobs = worker::jobs(&mut db);
    let runner = worker::open(&db);
    guard::reject_completion_audit(&mut db);
    let attempt = retry::deferred(runner.run_next().unwrap());
    assert!(guard::fault_reached(&mut db));
    guard::allow_completion_audit(&mut db);
    let healthy = retry::completed(runner.run_next().unwrap());
    assert_ne!(healthy.job_id, attempt.job_id);
    let failed_job = jobs.iter().find(|job| job.id == attempt.job_id).unwrap();
    let failed_base = bases
        .iter()
        .find(|base| base.id == failed_job.deadline_id)
        .unwrap();
    retry::assert_attempt(&mut db, failed_job, failed_base, &attempt);
    let healthy_job = jobs.iter().find(|job| job.id == healthy.job_id).unwrap();
    let completed_row = worker::result_row(&mut db, healthy_job);
    let failed_row = retry::attempt_row(&mut db, &attempt);
    retry::assert_counts(&mut db, 1, 1, 1);
    drop(runner);
    let before = retry::snapshot(&mut db);
    recovery::restore(&mut db);
    assert_eq!(retry::snapshot(&mut db), before);
    // Normal startup must validate restored functions, permissions and history.
    // No migrate call can repair a broken dump before this assertion.
    let restored = guard::open_at(&db, attempt.retry_at - Duration::nanoseconds(1));
    assert_eq!(
        restored.result(healthy_job.id).unwrap().as_ref(),
        Some(healthy.as_ref())
    );
    assert_eq!(
        restored.latest_attempt(failed_job.id).unwrap(),
        Some(attempt.clone())
    );
    assert!(restored.result(failed_job.id).unwrap().is_none());
    retry::assert_idle(&mut db, &restored);
    assert_eq!(retry::snapshot(&mut db), before);
    drop(restored);
    let due = guard::open_at(&db, attempt.retry_at);
    let completed = retry::completed(due.run_next().unwrap());
    assert_eq!(completed.job_id, failed_job.id);
    let next = worker::current(&db, failed_base.id);
    retry::assert_revision_result(&completed, failed_job, failed_base, &next);
    assert_eq!(completed.completed_at, attempt.retry_at);
    assert_eq!(worker::result_row(&mut db, healthy_job), completed_row);
    assert_eq!(retry::attempt_row(&mut db, &attempt), failed_row);
    retry::assert_counts(&mut db, 2, 2, 1);
    retry::assert_idle(&mut db, &due);
}
