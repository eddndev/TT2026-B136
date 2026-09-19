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

use application::deadline_worker::{
    DeadlineWorkerErrorCode as Code, DeadlineWorkerFailureKind as Kind, DeadlineWorkerStore,
};
use deadline_backend_support as dl;
use deadline_worker_backend_support as worker;
use deadline_worker_guard_support as guard;
use deadline_worker_retry_support as retry;

#[test]
fn completion_lock_unavailable_is_durably_deferred_with_its_sqlstate_category() {
    assert_deferred("55P03", Code::LockUnavailable, "lock_unavailable");
}

#[test]
fn completion_query_canceled_is_durably_deferred_with_its_sqlstate_category() {
    assert_deferred(
        "57014",
        Code::TransactionInterrupted,
        "transaction_interrupted",
    );
}

fn assert_deferred(sqlstate: &str, expected: Code, stored_code: &str) {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let setup = guard::setup(&mut db);
    let runner = worker::open(&db);
    let before_data = guard::data_snapshot(&mut db);
    let before_audits = guard::audit_count(&mut db);
    reject_completion_with_sqlstate(&mut db, sqlstate);

    let outcome = runner.run_next();
    let reached = guard::fault_reached(&mut db);
    // Remove the selective test fault before assertions or validated reopen.
    guard::allow_completion_audit(&mut db);
    assert!(
        reached,
        "completion audit did not reach SQLSTATE {sqlstate}"
    );
    let attempt = retry::deferred(outcome.unwrap());
    assert_eq!(attempt.failure_kind, Kind::Transient);
    assert_eq!(attempt.error_code, expected);
    assert_eq!(attempt.job_id, setup.job.id);
    assert_eq!(attempt.attempt_number, 1);
    assert!(attempt.retry_at > attempt.failed_at);
    assert_eq!(guard::data_snapshot(&mut db), before_data);
    assert!(runner.result(setup.job.id).unwrap().is_none());
    assert_eq!(
        runner.latest_attempt(setup.job.id).unwrap(),
        Some(attempt.clone())
    );
    retry::assert_counts(&mut db, 0, 0, 1);
    guard::assert_attempt(&mut db, &setup, &attempt, before_audits);
    let row = db
        .admin
        .query_one(
            "SELECT failure_kind,error_code FROM deadline_reevaluation_attempts WHERE attempt_id=$1",
            &[&attempt.attempt_id],
        )
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "transient");
    assert_eq!(row.get::<_, String>(1), stored_code);

    let before_reopen = retry::snapshot(&mut db);
    drop(runner);
    let reopened = worker::open(&db);
    assert!(reopened.result(setup.job.id).unwrap().is_none());
    assert_eq!(
        reopened.latest_attempt(setup.job.id).unwrap(),
        Some(attempt)
    );
    assert_eq!(retry::snapshot(&mut db), before_reopen);
}

fn reject_completion_with_sqlstate(db: &mut dl::Fixture, sqlstate: &str) {
    assert!(matches!(sqlstate, "55P03" | "57014"));
    guard::reject_completion_audit(db);
    let definition: String = db
        .admin
        .query_one(
            "SELECT pg_get_functiondef('reject_worker_completion_audit()'::regprocedure)",
            &[],
        )
        .unwrap()
        .get(0);
    let original = "RAISE EXCEPTION 'injected completion audit failure';";
    assert_eq!(definition.matches(original).count(), 1);
    // Preserve the existing proof that revision and result were both inserted,
    // and its early return allowing the separate deferred audit transaction.
    let changed = definition.replace(
        original,
        &format!("RAISE EXCEPTION 'injected completion audit failure' USING ERRCODE='{sqlstate}';"),
    );
    db.admin.batch_execute(&changed).unwrap();
}
