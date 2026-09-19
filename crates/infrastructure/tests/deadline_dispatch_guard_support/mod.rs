#![allow(dead_code)]

use crate::{deadline_backend_support as dl, deadline_dispatch_support as dispatch};
use application::{deadlines::DeadlineDetail, procedural_facts::FactDetail};
use postgres::{error::SqlState, GenericClient};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn setup(ids: &[u128]) -> Option<(dl::Fixture, FactDetail, Vec<DeadlineDetail>)> {
    let mut db = dl::Fixture::new()?;
    let profile = dl::profile(&db);
    let source = dl::source(&db);
    let deadlines = ids
        .iter()
        .map(|id| dispatch::legacy(&db, &profile, &source, *id))
        .collect();
    let store = dispatch::open(&db);
    dispatch::drain_existing(&mut db, &store);
    drop(store);
    let changed = dispatch::advance(&db, &source);
    Some((db, changed, deadlines))
}

pub fn insert_job<C: GenericClient>(
    client: &mut C,
    db: &dl::Fixture,
    deadline: u128,
    event: Option<i64>,
) {
    let policy = event.is_none().then_some(1_i16);
    let seconds = db.at.unix_timestamp();
    let nanos = i32::try_from(db.at.nanosecond()).unwrap();
    let affected = client
        .execute(
            "INSERT INTO deadline_reevaluation_jobs(id,operation_id,deadline_id,case_id,
            event_sequence,bootstrap_policy_version,created_at_seconds,created_at_nanoseconds)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8)",
            &[
                &Uuid::new_v4(),
                &Uuid::new_v4(),
                &dispatch::id(deadline).as_uuid(),
                &db.case.as_uuid(),
                &event,
                &policy,
                &seconds,
                &nanos,
            ],
        )
        .unwrap();
    assert_eq!(affected, 1);
}

pub fn snapshot(db: &mut dl::Fixture) -> Value {
    json!({"dispatch":dispatch::snapshot(db),"deadlines":dl::snapshot(db)})
}

pub fn jobs(db: &mut dl::Fixture) -> Value {
    db.admin.query_one(
        "SELECT coalesce(jsonb_agg(to_jsonb(j) ORDER BY id),'[]') FROM deadline_reevaluation_jobs j",
        &[],
    ).unwrap().get(0)
}

pub fn rejects_in_savepoint<C: GenericClient>(client: &mut C, sql: &str) {
    client.batch_execute("SAVEPOINT rejected_cursor").unwrap();
    let error = client.batch_execute(sql).unwrap_err();
    assert_eq!(
        error.code(),
        Some(&SqlState::CHECK_VIOLATION),
        "{sql}: {error:?}"
    );
    client
        .batch_execute("ROLLBACK TO SAVEPOINT rejected_cursor; RELEASE SAVEPOINT rejected_cursor")
        .unwrap();
}
