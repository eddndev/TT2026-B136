use crate::{
    deadline_backend_support as dl, deadline_dispatch_support as dispatch,
    deadline_tracked_backend_support as tracked, deadline_worker_backend_support as worker,
};
use application::{
    deadline_dispatch::DeadlineDispatchStream, deadline_reevaluation::*,
    deadline_tracking::TrackingPolicies, deadline_worker::*, deadlines::*,
    procedural_facts::FactDetail,
};
use infrastructure::RingSha256Hasher;
use uuid::Uuid;

pub fn clear_events(db: &mut dl::Fixture) {
    let store = dispatch::open(db);
    dispatch::drain_existing(db, &store);
    assert!(worker::jobs(db).is_empty());
}

pub fn commit(
    db: &dl::Fixture,
    command: &DeadlineCommand,
    policies: Option<TrackingPolicies>,
    parent: Option<&FactDetail>,
) -> DeadlineDetail {
    let store = dl::store(db);
    let prepared = prepare_tracked_deadline_change(
        &RingSha256Hasher,
        DeadlineActorSnapshot::User {
            id: db.owner,
            email: "owner@example.test".into(),
        },
        db.case,
        command.clone(),
        store.prepare(db.owner, db.case, command).unwrap(),
        policies,
        parent,
    )
    .unwrap();
    store.commit(db.owner, prepared).unwrap()
}

pub fn event_job(db: &mut dl::Fixture, operation: Uuid) -> (worker::Job, SourceEventReference) {
    let batch = dispatch::dispatch(&dispatch::open(db), DeadlineDispatchStream::Events, 20);
    assert_eq!((batch.selected, batch.inserted), (1, 1));
    let event = batch.event.unwrap();
    assert_eq!(event.operation_id, operation);
    let row = db.admin.query_one(
        "SELECT id,operation_id,deadline_id FROM deadline_reevaluation_jobs WHERE event_sequence=$1",
        &[&i64::try_from(event.sequence).unwrap()]).unwrap();
    (
        worker::Job {
            id: row.get(0),
            operation_id: DeadlineOperationId::from_uuid(row.get(1)),
            deadline_id: DeadlineId::from_uuid(row.get(2)),
        },
        event,
    )
}

pub fn revision(
    db: &mut dl::Fixture,
    base: &DeadlineDetail,
    job: &worker::Job,
    event: SourceEventReference,
) -> (DeadlineDetail, DeadlineWorkerResult) {
    let runner = worker::open(db);
    let DeadlineWorkerRun::Completed(result) = runner.run_next().unwrap() else {
        panic!("one eligible source event must complete")
    };
    assert_eq!(result.job_id, job.id);
    let next = worker::current(db, base.id);
    worker::assert_technical(
        base,
        &next,
        job,
        TechnicalCause::SourceEvent {
            job_id: job.id,
            event,
        },
    );
    worker::assert_result(db, job, base, Some(&next), "revision");
    let DeadlineWorkerOutcome::Revision { revision, receipt } = &result.outcome else {
        panic!("changed dependency must record a technical revision")
    };
    assert_eq!(*revision, next.revision);
    assert_eq!(receipt.as_ref(), &next.receipt);
    assert_eq!(
        runner.result(job.id).unwrap().as_ref(),
        Some(result.as_ref())
    );
    (next, *result)
}

pub fn observed(value: &DeadlineDetail, role: ObservationRole) -> &ObservationEntry {
    value
        .tracking
        .as_ref()
        .unwrap()
        .observations
        .entries
        .iter()
        .find(|entry| entry.role == role)
        .unwrap()
}

pub fn history(db: &mut dl::Fixture, values: &[DeadlineDetail], results: &[DeadlineWorkerResult]) {
    let snapshot = worker::snapshot(db);
    let runner = worker::open(db);
    for result in results {
        assert_eq!(runner.result(result.job_id).unwrap().as_ref(), Some(result));
    }
    assert_eq!(worker::snapshot(db), snapshot);
    tracked::assert_readback(db, dl::store(db).as_ref(), values);
    worker::assert_idle(db, &runner);
}
