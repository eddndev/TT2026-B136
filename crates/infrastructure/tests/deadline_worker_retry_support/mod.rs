use crate::{
    deadline_backend_support as dl, deadline_dispatch_support as dispatch,
    deadline_tracked_backend_support as tracked, deadline_worker_backend_support as worker,
    procedural_fact_backend_support as facts,
};
use application::{
    deadline_dispatch::DeadlineDispatchStream,
    deadline_worker::{
        DeadlineWorkerAttempt, DeadlineWorkerFailureKind, DeadlineWorkerOutcome,
        DeadlineWorkerResult, DeadlineWorkerRun, DeadlineWorkerStore,
    },
    deadlines::*,
    procedural_facts::{FactDeclaration, FactDetail},
};
use domain::deadline_triggers::TriggerSourceRef;
use serde_json::Value;

pub fn pair(db: &mut dl::Fixture) -> Vec<DeadlineDetail> {
    let (profile, source) = worker::inputs(db);
    let store = dl::store(db);
    let bases = [701, 702]
        .into_iter()
        .map(|id| {
            let command = dispatch::command(db, &profile, &source, id);
            let prepared =
                tracked::tracked_prepared(db, store.as_ref(), &command, Some(tracked::policies()));
            store.commit(db.owner, prepared).unwrap()
        })
        .collect();
    dispatch::advance(db, &source);
    dispatch_events(db, 2);
    bases
}

pub fn dispatch_events(db: &dl::Fixture, count: u32) {
    let batch = dispatch::dispatch(&dispatch::open(db), DeadlineDispatchStream::Events, 20);
    assert_eq!((batch.selected, batch.inserted), (count, count));
}

pub fn completed(value: DeadlineWorkerRun) -> Box<DeadlineWorkerResult> {
    let DeadlineWorkerRun::Completed(result) = value else {
        panic!("the eligible valid job must complete")
    };
    result
}

pub fn deferred(value: DeadlineWorkerRun) -> DeadlineWorkerAttempt {
    let DeadlineWorkerRun::Deferred(attempt) = value else {
        panic!("the rolled-back execution must retain a durable attempt")
    };
    attempt
}

pub fn snapshot(db: &mut dl::Fixture) -> Value {
    let mut value = worker::snapshot(db);
    let attempts: Value = db
        .admin
        .query_one(
            "SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY job_id,attempt_number),'[]'::jsonb)
         FROM deadline_reevaluation_attempts a",
            &[],
        )
        .unwrap()
        .get(0);
    value
        .as_object_mut()
        .unwrap()
        .insert("attempts".into(), attempts);
    value
}

pub fn attempt_row(db: &mut dl::Fixture, attempt: &DeadlineWorkerAttempt) -> Value {
    db.admin
        .query_one(
            "SELECT to_jsonb(a) FROM deadline_reevaluation_attempts a WHERE attempt_id=$1",
            &[&attempt.attempt_id],
        )
        .unwrap()
        .get(0)
}

pub fn assert_attempt(
    db: &mut dl::Fixture,
    job: &worker::Job,
    base: &DeadlineDetail,
    attempt: &DeadlineWorkerAttempt,
) {
    assert_eq!(attempt.job_id, job.id);
    assert_eq!(attempt.attempt_number, 1);
    assert_eq!(attempt.failed_at, db.at);
    assert!(attempt.retry_at > attempt.failed_at);
    assert_eq!(attempt.failure_kind, DeadlineWorkerFailureKind::Transient);
    let checked = attempt.checked_base.as_ref().unwrap();
    assert_eq!(checked.revision, base.revision);
    assert_eq!(
        checked.receipt.submission_digest,
        base.receipt.submission_digest
    );
    assert_eq!(checked.receipt.capture_digest, base.receipt.capture_digest);
    let count: i64 = db
        .admin
        .query_one("SELECT count(*) FROM deadline_reevaluation_attempts", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
    let rows = db
        .admin
        .query(
            "SELECT actor,resource FROM audit_events
         WHERE action='deadline.reevaluation_deferred'",
            &[],
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, String>(0), "deadline_reevaluator");
    let resource: String = rows[0].get(1);
    for value in [
        job.id.to_string(),
        job.operation_id.to_string(),
        attempt.attempt_id.to_string(),
        base.receipt.submission_digest.to_hex(),
        base.receipt.capture_digest.to_hex(),
    ] {
        assert!(resource.contains(&value), "deferred audit omits {value}");
    }
}

pub fn assert_revision_result(
    value: &DeadlineWorkerResult,
    job: &worker::Job,
    base: &DeadlineDetail,
    next: &DeadlineDetail,
) {
    assert_eq!(value.job_id, job.id);
    assert_eq!(value.case_id, base.case_id);
    assert_eq!(value.deadline_id, base.id);
    assert_eq!(value.command.operation_id, job.operation_id);
    assert_eq!(value.base.revision, base.revision);
    assert_eq!(
        value.base.receipt.submission_digest,
        base.receipt.submission_digest
    );
    assert_eq!(
        value.base.receipt.capture_digest,
        base.receipt.capture_digest
    );
    let DeadlineWorkerOutcome::Revision { revision, receipt } = &value.outcome else {
        panic!("the followed source change must produce a revision")
    };
    assert_eq!(*revision, next.revision);
    assert_eq!(receipt.as_ref(), &next.receipt);
    worker::assert_preserved(base, next);
}

pub fn correct_and_attend(
    db: &dl::Fixture,
    base: &DeadlineDetail,
    source: &FactDetail,
    responsible: domain::identity::UserId,
) -> [DeadlineDetail; 2] {
    let store = dl::store(db);
    let mut command = dl::correct(base);
    let definition = dl::definition_mut(&mut command);
    definition.input.selection.source =
        FactDeclaration::Known(TriggerSourceRef::Resolution(facts::resolution_ref(source)));
    definition.responsible = responsible;
    let prepared =
        tracked::tracked_prepared(db, store.as_ref(), &command, Some(tracked::policies()));
    let corrected = store.commit(db.owner, prepared).unwrap();
    let prepared = tracked::tracked_prepared(db, store.as_ref(), &dl::attention(&corrected), None);
    let attended = store.commit(db.owner, prepared).unwrap();
    [corrected, attended]
}

pub fn assert_counts(db: &mut dl::Fixture, revisions: i64, results: i64, attempts: i64) {
    let row = db
        .admin
        .query_one(
            "SELECT
        (SELECT count(*) FROM case_deadline_revisions WHERE action='reevaluate'),
        (SELECT count(*) FROM deadline_reevaluation_results),
        (SELECT count(*) FROM deadline_reevaluation_attempts),
        (SELECT count(*) FROM audit_events WHERE action='deadline.reevaluated')",
            &[],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), revisions);
    assert_eq!(row.get::<_, i64>(1), results);
    assert_eq!(row.get::<_, i64>(2), attempts);
    assert_eq!(row.get::<_, i64>(3), revisions);
}

pub fn assert_idle(db: &mut dl::Fixture, runner: &dyn DeadlineWorkerStore) {
    let before = snapshot(db);
    assert!(matches!(
        runner.run_next().unwrap(),
        DeadlineWorkerRun::Idle
    ));
    assert_eq!(snapshot(db), before);
}
