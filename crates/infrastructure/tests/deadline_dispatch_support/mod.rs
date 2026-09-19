use crate::{
    case_stage_database_support::FixedClock, deadline_backend_support as dl,
    procedural_fact_backend_support as facts,
};
use application::{
    deadline_dispatch::*, deadline_profiles::DeadlineProfileDetail, deadlines::*,
    procedural_facts::*,
};
use domain::identity::Role;
use infrastructure::{PostgresDeadlineDispatchStore, RingSha256Hasher};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

pub fn open(db: &dl::Fixture) -> PostgresDeadlineDispatchStore {
    PostgresDeadlineDispatchStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
    .unwrap()
}

pub fn dispatch(
    store: &dyn DeadlineDispatchStore,
    stream: DeadlineDispatchStream,
    limit: u32,
) -> DeadlineDispatchBatch {
    store
        .dispatch(DeadlineDispatchRequest {
            stream,
            limit: DeadlineDispatchLimit::new(limit).unwrap(),
        })
        .unwrap()
}

pub fn id(value: u128) -> DeadlineId {
    DeadlineId::from_uuid(Uuid::from_u128(value))
}

pub fn command(
    db: &dl::Fixture,
    profile: &DeadlineProfileDetail,
    source: &FactDetail,
    value: u128,
) -> DeadlineCommand {
    let mut command = dl::command(db, profile, source);
    command.deadline_id = id(value);
    command
}

pub fn legacy(
    db: &dl::Fixture,
    profile: &DeadlineProfileDetail,
    source: &FactDetail,
    value: u128,
) -> DeadlineDetail {
    dl::persist(
        &dl::service(db, db.owner, Role::Owner),
        db.case,
        command(db, profile, source, value),
    )
}

pub fn advance(db: &dl::Fixture, source: &FactDetail) -> FactDetail {
    facts::persist(
        &facts::service(db, db.owner, Role::Owner),
        db.case,
        facts::correct(source),
    )
}

pub fn event_sequence(db: &mut dl::Fixture, source: &FactDetail) -> u64 {
    let operation = source.snapshot.metadata().receipt.operation_id.as_uuid();
    let sequence: i64 = db
        .admin
        .query_one(
            "SELECT sequence FROM deadline_source_events WHERE operation_id=$1",
            &[&operation],
        )
        .unwrap()
        .get(0);
    u64::try_from(sequence).unwrap()
}

pub fn drain_existing(db: &mut dl::Fixture, store: &dyn DeadlineDispatchStore) {
    let row = db
        .admin
        .query_one(
            "SELECT (SELECT count(*) FROM deadline_source_events),
                (SELECT count(*) FROM case_deadlines)",
            &[],
        )
        .unwrap();
    let events: i64 = row.get(0);
    assert!(row.get::<_, i64>(1) <= 100);
    for _ in 0..=events {
        let batch = dispatch(store, DeadlineDispatchStream::Events, 100);
        if batch.event.is_none() {
            assert_eq!(batch.selected, 0);
            assert_eq!(batch.inserted, 0);
            return;
        }
        assert!(batch.completed_scan);
    }
    panic!("finite initial event stream did not reach idle");
}

pub fn snapshot(db: &mut dl::Fixture) -> Value {
    db.admin
        .query_one(
            "SELECT jsonb_build_object(
        'cursor',(SELECT jsonb_agg(to_jsonb(c)) FROM deadline_dispatch_cursor c),
        'jobs',(SELECT jsonb_agg(to_jsonb(j) ORDER BY id) FROM deadline_reevaluation_jobs j),
        'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))",
            &[],
        )
        .unwrap()
        .get(0)
}

pub fn event_jobs(db: &mut dl::Fixture, sequence: u64) -> Vec<DeadlineId> {
    db.admin.query(
        "SELECT deadline_id FROM deadline_reevaluation_jobs WHERE event_sequence=$1 ORDER BY deadline_id",
        &[&i64::try_from(sequence).unwrap()],
    ).unwrap().into_iter().map(|row| DeadlineId::from_uuid(row.get(0))).collect()
}

pub fn bootstrap_jobs(db: &mut dl::Fixture) -> Vec<DeadlineId> {
    db.admin
        .query(
            "SELECT deadline_id FROM deadline_reevaluation_jobs
         WHERE event_sequence IS NULL AND bootstrap_policy_version=1 ORDER BY deadline_id",
            &[],
        )
        .unwrap()
        .into_iter()
        .map(|row| DeadlineId::from_uuid(row.get(0)))
        .collect()
}

pub fn rolled_back_source_event(db: &mut dl::Fixture, source: &FactDetail) -> i64 {
    let workflow = facts::service(db, db.owner, Role::Owner);
    let command = facts::correct(source);
    let prepared = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    let before = facts::snapshot(db);
    let old: i64 = db
        .admin
        .query_one(
            "SELECT last_value FROM deadline_source_events_sequence",
            &[],
        )
        .unwrap()
        .get(0);
    db.admin
        .batch_execute(
            "CREATE FUNCTION reject_dispatch_fixture_audit() RETURNS trigger
        LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected audit failure'; END; $$;
        CREATE TRIGGER reject_dispatch_fixture_audit BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_dispatch_fixture_audit()",
        )
        .unwrap();
    assert!(matches!(
        workflow.submit("session", db.case, command, prepared.submission_digest),
        Err(application::ApplicationError::Port(_))
    ));
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_dispatch_fixture_audit ON audit_events;
        DROP FUNCTION reject_dispatch_fixture_audit()",
        )
        .unwrap();
    assert_eq!(facts::snapshot(db), before);
    let gap: i64 = db
        .admin
        .query_one(
            "SELECT last_value FROM deadline_source_events_sequence",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(
        gap > old,
        "the rejected source write must consume a real sequence"
    );
    assert!(!db
        .admin
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM deadline_source_events WHERE sequence=$1)",
            &[&gap],
        )
        .unwrap()
        .get::<_, bool>(0));
    gap
}

pub fn deadline_history(db: &mut dl::Fixture) -> Value {
    db.admin
        .query_one(
            "SELECT jsonb_agg(to_jsonb(r) ORDER BY deadline_id,revision)
        FROM case_deadline_revisions r",
            &[],
        )
        .unwrap()
        .get(0)
}
