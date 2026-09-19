use crate::{deadline_backend_support as dl, deadline_dispatch_support as dispatch};
use application::{
    deadline_dispatch::*, deadline_profiles::DeadlineProfileDetail, deadlines::*,
    procedural_facts::FactDetail,
};
use postgres::Error;
use serde_json::Value;
use uuid::Uuid;

pub fn request(stream: DeadlineDispatchStream, limit: u32) -> DeadlineDispatchRequest {
    DeadlineDispatchRequest {
        stream,
        limit: DeadlineDispatchLimit::new(limit).unwrap(),
    }
}

pub fn seed(
    db: &dl::Fixture,
    ids: &[u128],
) -> (DeadlineProfileDetail, FactDetail, Vec<DeadlineDetail>) {
    let profile = dl::profile(db);
    let source = dl::source(db);
    let rows = ids
        .iter()
        .map(|id| dispatch::legacy(db, &profile, &source, *id))
        .collect();
    (profile, source, rows)
}

pub fn snapshot(db: &mut dl::Fixture) -> Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'cursor',(SELECT jsonb_agg(to_jsonb(c)) FROM deadline_dispatch_cursor c),
        'jobs',(SELECT jsonb_agg(to_jsonb(j) ORDER BY id) FROM deadline_reevaluation_jobs j),
        'roots',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id) FROM case_deadlines d),
        'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY deadline_id,revision) FROM case_deadline_revisions r),
        'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[])
        .unwrap().get(0)
}

pub fn jobs(db: &mut dl::Fixture) -> Value {
    db.admin
        .query_one(
            "SELECT coalesce(jsonb_agg(to_jsonb(j) ORDER BY deadline_id,id),'[]'::jsonb)
        FROM deadline_reevaluation_jobs j",
            &[],
        )
        .unwrap()
        .get(0)
}

pub fn dispatch_audit_count(db: &mut dl::Fixture) -> i64 {
    db.admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='deadline.dispatch_advanced'",
            &[],
        )
        .unwrap()
        .get(0)
}

pub fn reject_audit_after_writes(db: &mut dl::Fixture) {
    db.admin.batch_execute(&format!("CREATE SEQUENCE dispatch_fault_reached;
        GRANT USAGE ON SEQUENCE dispatch_fault_reached TO {};
        CREATE FUNCTION reject_dispatch_audit_after_writes() RETURNS trigger
        LANGUAGE plpgsql AS $$ BEGIN
        IF NOT EXISTS(SELECT 1 FROM deadline_reevaluation_jobs) THEN
            RAISE EXCEPTION 'dispatch audit preceded job insert'; END IF;
        IF NOT EXISTS(SELECT 1 FROM deadline_dispatch_cursor WHERE bootstrap_after_deadline_id IS NOT NULL) THEN
            RAISE EXCEPTION 'dispatch audit preceded cursor update'; END IF;
        PERFORM nextval('dispatch_fault_reached');
        RAISE EXCEPTION 'injected dispatch audit failure after job and cursor';
        END; $$;
        CREATE TRIGGER reject_dispatch_audit_after_writes BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_dispatch_audit_after_writes()", db.role)).unwrap();
}

pub fn fault_reached(db: &mut dl::Fixture) -> bool {
    db.admin
        .query_one("SELECT is_called FROM dispatch_fault_reached", &[])
        .unwrap()
        .get(0)
}

pub fn allow_audit(db: &mut dl::Fixture) {
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_dispatch_audit_after_writes ON audit_events;
        DROP FUNCTION reject_dispatch_audit_after_writes(); DROP SEQUENCE dispatch_fault_reached",
        )
        .unwrap();
}

/// Force the UUID collision before the real row guard, without disabling it.
pub fn force_job_operation(db: &mut dl::Fixture, operation: Uuid) {
    db.admin.batch_execute(&format!("CREATE FUNCTION force_dispatch_operation_collision() RETURNS trigger
        LANGUAGE plpgsql AS $$ BEGIN NEW.operation_id:='{operation}'::uuid; RETURN NEW; END; $$;
        CREATE TRIGGER aaa_force_dispatch_operation_collision BEFORE INSERT ON deadline_reevaluation_jobs
        FOR EACH ROW EXECUTE FUNCTION force_dispatch_operation_collision()")).unwrap();
}

pub fn stop_forcing_operation(db: &mut dl::Fixture) {
    db.admin
        .batch_execute(
            "DROP TRIGGER aaa_force_dispatch_operation_collision ON deadline_reevaluation_jobs;
        DROP FUNCTION force_dispatch_operation_collision()",
        )
        .unwrap();
}

pub fn insert_bootstrap(
    db: &dl::Fixture,
    deadline: DeadlineId,
    operation: Uuid,
) -> Result<u64, Error> {
    db.runtime().execute(
        "INSERT INTO deadline_reevaluation_jobs(
        id,operation_id,deadline_id,case_id,event_sequence,bootstrap_policy_version,
        created_at_seconds,created_at_nanoseconds) VALUES($1,$2,$3,$4,NULL,1,$5,$6)",
        &[
            &Uuid::new_v4(),
            &operation,
            &deadline.as_uuid(),
            &db.case.as_uuid(),
            &db.at.unix_timestamp(),
            &i32::try_from(db.at.nanosecond()).unwrap(),
        ],
    )
}

pub fn assert_unique(error: &Error, constraint: &str) {
    let error = error
        .as_db_error()
        .expect("database uniqueness rejection expected");
    assert_eq!(error.code(), &postgres::error::SqlState::UNIQUE_VIOLATION);
    assert_eq!(error.constraint(), Some(constraint));
}
