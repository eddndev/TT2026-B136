use crate::{
    case_stage_database_support::FixedClock, deadline_backend_support as dl,
    deadline_dispatch_support as dispatch, deadline_tracked_backend_support as tracked,
};
use application::{
    deadline_profiles::DeadlineProfileDetail, deadline_reevaluation::*, deadline_worker::*,
    deadlines::*, procedural_facts::FactDetail,
};
use infrastructure::{PostgresDeadlineWorkerStore, RingSha256Hasher};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

pub struct Job {
    pub id: Uuid,
    pub operation_id: DeadlineOperationId,
    pub deadline_id: DeadlineId,
}

pub fn open(db: &dl::Fixture) -> PostgresDeadlineWorkerStore {
    PostgresDeadlineWorkerStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
    .unwrap()
}

pub fn inputs(db: &mut dl::Fixture) -> (DeadlineProfileDetail, FactDetail) {
    let profile = dl::profile(db);
    let source = dl::source(db);
    let dispatcher = dispatch::open(db);
    dispatch::drain_existing(db, &dispatcher);
    assert!(jobs(db).is_empty());
    (profile, source)
}

pub fn accepted(db: &mut dl::Fixture, id: u128) -> (FactDetail, DeadlineDetail) {
    let (profile, source) = inputs(db);
    let command = dispatch::command(db, &profile, &source, id);
    let store = dl::store(db);
    let prepared =
        tracked::tracked_prepared(db, store.as_ref(), &command, Some(tracked::policies()));
    (source, store.commit(db.owner, prepared).unwrap())
}

pub fn jobs(db: &mut dl::Fixture) -> Vec<Job> {
    db.admin
        .query(
            "SELECT id,operation_id,deadline_id FROM deadline_reevaluation_jobs
        ORDER BY deadline_id,id",
            &[],
        )
        .unwrap()
        .into_iter()
        .map(|row| Job {
            id: row.get(0),
            operation_id: DeadlineOperationId::from_uuid(row.get(1)),
            deadline_id: DeadlineId::from_uuid(row.get(2)),
        })
        .collect()
}

pub fn current(db: &dl::Fixture, id: DeadlineId) -> DeadlineDetail {
    dl::store(db)
        .get(db.owner, db.case, id, None, db.at)
        .unwrap()
}

pub fn assert_preserved(base: &DeadlineDetail, next: &DeadlineDetail) {
    assert_eq!(next.definition, base.definition);
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(next.responsible, base.responsible);
    assert_eq!(next.attention, base.attention);
    assert_eq!(next.status, base.status);
    deadline_successor_matches(&RingSha256Hasher, base, next).unwrap();
    deadline_receipt_matches(&RingSha256Hasher, next).unwrap();
}

pub fn assert_technical(
    base: &DeadlineDetail,
    next: &DeadlineDetail,
    job: &Job,
    cause: TechnicalCause,
) {
    assert_eq!(next.id, job.deadline_id);
    assert_eq!(next.revision.get(), base.revision.get() + 1);
    assert_eq!(next.receipt.action, DeadlineAction::Reevaluate);
    assert_eq!(next.receipt.operation_id, job.operation_id);
    assert_eq!(next.receipt.expected_revision, base.revision.get());
    assert_eq!(
        next.recorded_by,
        DeadlineActorSnapshot::Technical {
            service: TechnicalService::DeadlineReevaluator,
            policy_version: 1,
        }
    );
    let DeadlineReceiptVersion::Tracked(receipt) = &next.receipt.version else {
        panic!("technical revision requires a tracked receipt");
    };
    assert_eq!(receipt.cause, Some(cause));
    assert_eq!(
        receipt.predecessor,
        Some(PredecessorReceipt {
            submission_digest: base.receipt.submission_digest,
            capture_digest: base.receipt.capture_digest,
        })
    );
}

pub fn assert_result(
    db: &mut dl::Fixture,
    job: &Job,
    base: &DeadlineDetail,
    next: Option<&DeadlineDetail>,
    outcome: &str,
) {
    let row = db
        .admin
        .query_one(
            "SELECT base_revision,base_submission_digest,base_capture_digest,outcome,
            result_revision,result_submission_digest,result_capture_digest,
            completed_at_seconds,completed_at_nanoseconds
         FROM deadline_reevaluation_results WHERE job_id=$1",
            &[&job.id],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), i64::from(base.revision.get()));
    assert_eq!(
        row.get::<_, Vec<u8>>(1),
        base.receipt.submission_digest.as_bytes().to_vec()
    );
    assert_eq!(
        row.get::<_, Vec<u8>>(2),
        base.receipt.capture_digest.as_bytes().to_vec()
    );
    assert_eq!(row.get::<_, String>(3), outcome);
    assert_eq!(
        row.get::<_, Option<i64>>(4),
        next.map(|value| i64::from(value.revision.get()))
    );
    assert_eq!(
        row.get::<_, Option<Vec<u8>>>(5),
        next.map(|value| value.receipt.submission_digest.as_bytes().to_vec())
    );
    assert_eq!(
        row.get::<_, Option<Vec<u8>>>(6),
        next.map(|value| value.receipt.capture_digest.as_bytes().to_vec())
    );
    assert_eq!(row.get::<_, i64>(7), db.at.unix_timestamp());
    assert_eq!(
        row.get::<_, i32>(8),
        i32::try_from(db.at.nanosecond()).unwrap()
    );
    let action = if next.is_some() {
        "deadline.reevaluated"
    } else {
        "deadline.reevaluation_no_change"
    };
    let audits = db
        .admin
        .query(
            "SELECT actor,resource FROM audit_events WHERE action=$1 AND resource LIKE $2",
            &[&action, &format!("%{}%", job.id)],
        )
        .unwrap();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].get::<_, String>(0), "deadline_reevaluator");
    let resource: String = audits[0].get(1);
    for value in [
        job.id.to_string(),
        job.operation_id.to_string(),
        base.id.to_string(),
        base.case_id.to_string(),
        base.receipt.submission_digest.to_hex(),
        base.receipt.capture_digest.to_hex(),
    ] {
        assert!(resource.contains(&value), "audit omits {value}");
    }
}

pub fn snapshot(db: &mut dl::Fixture) -> Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'cursor',(SELECT jsonb_agg(to_jsonb(c)) FROM deadline_dispatch_cursor c),
        'jobs',(SELECT jsonb_agg(to_jsonb(j) ORDER BY id) FROM deadline_reevaluation_jobs j),
        'results',(SELECT jsonb_agg(to_jsonb(r) ORDER BY job_id) FROM deadline_reevaluation_results r),
        'revisions',(SELECT jsonb_agg(to_jsonb(d) ORDER BY deadline_id,revision) FROM case_deadline_revisions d),
        'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[])
        .unwrap().get(0)
}

pub fn result_row(db: &mut dl::Fixture, job: &Job) -> Value {
    db.admin
        .query_one(
            "SELECT to_jsonb(r) FROM deadline_reevaluation_results r WHERE job_id=$1",
            &[&job.id],
        )
        .unwrap()
        .get(0)
}

pub fn result_count(db: &mut dl::Fixture) -> i64 {
    db.admin
        .query_one("SELECT count(*) FROM deadline_reevaluation_results", &[])
        .unwrap()
        .get(0)
}

pub fn assert_idle(db: &mut dl::Fixture, worker: &dyn DeadlineWorkerStore) {
    let before = snapshot(db);
    assert!(matches!(
        worker.run_next().unwrap(),
        DeadlineWorkerRun::Idle
    ));
    assert_eq!(snapshot(db), before);
}
