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
mod deadline_worker_retry_support;
mod procedural_fact_backend_support;

use application::{
    deadline_technical::DeadlineReevaluationNoChange,
    deadline_tracking::DeadlineReviewState,
    deadline_worker::{DeadlineWorkerOutcome, DeadlineWorkerRun, DeadlineWorkerStore},
    deadlines::DeadlineAttention,
};
use deadline_backend_support as dl;
use deadline_dispatch_support as dispatch;
use deadline_tracked_backend_support as tracked;
use deadline_worker_backend_support as worker;
use deadline_worker_guard_support as guard;
use deadline_worker_retry_support as retry;
use std::sync::{Arc, Barrier};
use time::Duration;

#[test]
fn deferred_job_survives_reopen_and_does_not_delay_another_eligible_job() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let bases = retry::pair(&mut db);
    let jobs = worker::jobs(&mut db);
    assert_eq!(jobs.len(), 2);
    let runner = worker::open(&db);
    let before_history = dispatch::deadline_history(&mut db);
    guard::reject_completion_audit(&mut db);
    let attempt = retry::deferred(runner.run_next().unwrap());
    assert!(guard::fault_reached(&mut db));
    guard::allow_completion_audit(&mut db);
    let failed_job = jobs.iter().find(|job| job.id == attempt.job_id).unwrap();
    let base = bases
        .iter()
        .find(|base| base.id == failed_job.deadline_id)
        .unwrap();
    retry::assert_attempt(&mut db, failed_job, base, &attempt);
    retry::assert_counts(&mut db, 0, 0, 1);
    assert_eq!(dispatch::deadline_history(&mut db), before_history);
    assert!(runner.result(failed_job.id).unwrap().is_none());
    let original_attempt = retry::attempt_row(&mut db, &attempt);

    let healthy = retry::completed(runner.run_next().unwrap());
    assert_ne!(healthy.job_id, failed_job.id);
    let healthy_job = jobs.iter().find(|job| job.id == healthy.job_id).unwrap();
    let healthy_base = bases
        .iter()
        .find(|base| base.id == healthy.deadline_id)
        .unwrap();
    let healthy_revision = worker::current(&db, healthy.deadline_id);
    retry::assert_revision_result(&healthy, healthy_job, healthy_base, &healthy_revision);
    worker::assert_result(
        &mut db,
        healthy_job,
        healthy_base,
        Some(&healthy_revision),
        "revision",
    );
    retry::assert_counts(&mut db, 1, 1, 1);
    let healthy_row = worker::result_row(&mut db, healthy_job);
    drop(runner);

    let before_retry = attempt.retry_at - Duration::nanoseconds(1);
    assert!(before_retry >= db.at);
    let unchanged = retry::snapshot(&mut db);
    let waiting = guard::open_at(&db, before_retry);
    assert_eq!(
        waiting.latest_attempt(failed_job.id).unwrap(),
        Some(attempt.clone())
    );
    assert!(waiting.result(failed_job.id).unwrap().is_none());
    assert_eq!(
        waiting.result(healthy_job.id).unwrap().as_ref(),
        Some(healthy.as_ref())
    );
    retry::assert_idle(&mut db, &waiting);
    assert_eq!(retry::snapshot(&mut db), unchanged);
    drop(waiting);

    let due = guard::open_at(&db, attempt.retry_at);
    let completed = retry::completed(due.run_next().unwrap());
    assert_eq!(completed.job_id, failed_job.id);
    assert_eq!(completed.completed_at, attempt.retry_at);
    let next = worker::current(&db, base.id);
    retry::assert_revision_result(&completed, failed_job, base, &next);
    assert_eq!(next.recorded_at, attempt.retry_at);
    let row = worker::result_row(&mut db, failed_job);
    assert_eq!(
        row["completed_at_seconds"].as_i64(),
        Some(attempt.retry_at.unix_timestamp())
    );
    assert_eq!(
        row["completed_at_nanoseconds"].as_u64(),
        Some(u64::from(attempt.retry_at.nanosecond()))
    );
    retry::assert_counts(&mut db, 2, 2, 1);
    assert_eq!(retry::attempt_row(&mut db, &attempt), original_attempt);
    assert_eq!(worker::result_row(&mut db, healthy_job), healthy_row);
    assert_eq!(due.latest_attempt(failed_job.id).unwrap(), Some(attempt));
    assert_eq!(
        due.result(failed_job.id).unwrap().as_ref(),
        Some(completed.as_ref())
    );
    assert!(due.latest_attempt(healthy_job.id).unwrap().is_none());
    tracked::assert_readback(&db, dl::store(&db).as_ref(), &[base.clone(), next]);
    retry::assert_idle(&mut db, &due);
}

#[test]
fn two_workers_complete_once_and_replay_the_result_after_later_human_changes() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (source, base) = worker::accepted(&mut db, 703);
    let changed = dispatch::advance(&db, &source);
    retry::dispatch_events(&db, 1);
    let jobs = worker::jobs(&mut db);
    assert_eq!(jobs.len(), 1);
    let job = &jobs[0];
    let first = worker::open(&db);
    let second = worker::open(&db);
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = [first, second]
        .into_iter()
        .map(|runner| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                runner.run_next().unwrap()
            })
        })
        .collect();
    barrier.wait();
    let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(
        outcomes
            .iter()
            .filter(|r| matches!(r, DeadlineWorkerRun::Completed(_)))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|r| matches!(r, DeadlineWorkerRun::Idle))
            .count(),
        1
    );
    // Discard the completion payload; recovery must use the committed result.
    drop(outcomes);
    retry::assert_counts(&mut db, 1, 1, 0);
    let technical = worker::current(&db, base.id);
    let runner = worker::open(&db);
    let result = runner.result(job.id).unwrap().unwrap();
    retry::assert_revision_result(&result, job, &base, &technical);
    let original_result = worker::result_row(&mut db, job);
    let original_revision = tracked::revision_row(&mut db, &technical);
    drop(runner);

    let responsible = db.user("paralegal", true);
    let [corrected, attended] = retry::correct_and_attend(&db, &technical, &changed, responsible);
    assert_ne!(attended.definition.title, technical.definition.title);
    assert_eq!(attended.responsible.id, responsible);
    assert!(matches!(
        attended.attention,
        DeadlineAttention::Recorded { .. }
    ));
    assert_eq!(attended.review_state(), DeadlineReviewState::Accepted);
    let history = dispatch::deadline_history(&mut db);
    let before = retry::snapshot(&mut db);
    let reopened = worker::open(&db);
    assert_eq!(reopened.result(job.id).unwrap(), Some(result));
    assert!(reopened.latest_attempt(job.id).unwrap().is_none());
    retry::assert_idle(&mut db, &reopened);
    assert_eq!(retry::snapshot(&mut db), before);
    assert_eq!(worker::current(&db, base.id), attended);
    assert_eq!(dispatch::deadline_history(&mut db), history);
    assert_eq!(worker::result_row(&mut db, job), original_result);
    assert_eq!(
        tracked::revision_row(&mut db, &technical),
        original_revision
    );
    tracked::assert_readback(
        &db,
        dl::store(&db).as_ref(),
        &[base, technical, corrected, attended],
    );
    retry::assert_counts(&mut db, 1, 1, 0);
}

#[test]
fn human_correction_invalidates_old_base_backoff_without_overwriting_the_new_head() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (source, base) = worker::accepted(&mut db, 704);
    let changed = dispatch::advance(&db, &source);
    retry::dispatch_events(&db, 1);
    let jobs = worker::jobs(&mut db);
    assert_eq!(jobs.len(), 1);
    let job = &jobs[0];
    let runner = worker::open(&db);
    guard::reject_completion_audit(&mut db);
    let attempt = retry::deferred(runner.run_next().unwrap());
    assert!(guard::fault_reached(&mut db));
    guard::allow_completion_audit(&mut db);
    retry::assert_attempt(&mut db, job, &base, &attempt);
    let original_attempt = retry::attempt_row(&mut db, &attempt);
    drop(runner);

    let responsible = db.user("paralegal", true);
    let [corrected, attended] = retry::correct_and_attend(&db, &base, &changed, responsible);
    assert_ne!(attended.receipt.capture_digest, base.receipt.capture_digest);
    assert_eq!(attended.responsible.id, responsible);
    assert!(matches!(
        attended.attention,
        DeadlineAttention::Recorded { .. }
    ));
    assert_eq!(attended.review_state(), DeadlineReviewState::Accepted);
    assert!(db.at < attempt.retry_at);
    let before_history = dispatch::deadline_history(&mut db);
    let reopened = worker::open(&db);
    assert_eq!(
        reopened.latest_attempt(job.id).unwrap(),
        Some(attempt.clone())
    );
    let completed = retry::completed(reopened.run_next().unwrap());
    assert_eq!(completed.job_id, job.id);
    assert_eq!(completed.command.operation_id, job.operation_id);
    assert_eq!(completed.base.revision, attended.revision);
    assert_eq!(
        completed.base.receipt.submission_digest,
        attended.receipt.submission_digest
    );
    assert_eq!(
        completed.base.receipt.capture_digest,
        attended.receipt.capture_digest
    );
    assert_eq!(completed.completed_at, db.at);
    let DeadlineWorkerOutcome::NoChange { reason, checked } = &completed.outcome else {
        panic!("a human-accepted event must not create another revision")
    };
    assert_eq!(*reason, DeadlineReevaluationNoChange::AlreadyObserved);
    assert_eq!(
        checked.as_ref().unwrap().observations,
        attended.tracking.as_ref().unwrap().observations
    );
    worker::assert_result(&mut db, job, &attended, None, "already_observed");
    let row = worker::result_row(&mut db, job);
    assert!(row["checked_observations_canonical"].is_string());
    assert!(row["checked_administration_evidence_digest"].is_string());
    assert_eq!(dispatch::deadline_history(&mut db), before_history);
    assert_eq!(worker::current(&db, base.id), attended);
    assert_eq!(retry::attempt_row(&mut db, &attempt), original_attempt);
    assert_eq!(reopened.latest_attempt(job.id).unwrap(), Some(attempt));
    retry::assert_counts(&mut db, 0, 1, 1);
    assert_eq!(
        reopened.result(job.id).unwrap().as_ref(),
        Some(completed.as_ref())
    );
    tracked::assert_readback(&db, dl::store(&db).as_ref(), &[base, corrected, attended]);
    retry::assert_idle(&mut db, &reopened);
}
