pub mod sql;

use crate::{
    case_stage_database_support::FixedClock, deadline_backend_support as dl,
    deadline_dispatch_support as dispatch, deadline_tracked_backend_support as tracked,
    deadline_worker_backend_support as worker, procedural_fact_backend_support as facts,
};
use application::{
    deadline_dispatch::DeadlineDispatchStream,
    deadline_inputs::DeadlineSourceDetail,
    deadline_reevaluation::{DependencyFamily, SourceEventReference, TechnicalCause},
    deadline_technical::*,
    deadline_worker::{DeadlineWorkerAttempt, DeadlineWorkerRun, DeadlineWorkerStore},
    deadlines::*,
};
use infrastructure::{PostgresDeadlineWorkerStore, RingSha256Hasher};
use postgres::{error::SqlState, Error};
use serde_json::Value;
use std::sync::Arc;
use time::OffsetDateTime;

pub struct Scenario {
    pub base: DeadlineDetail,
    pub expected: DeadlineDetail,
    pub job: worker::Job,
    pub command: DeadlineReevaluationCommand,
    pub inputs: DeadlineReevaluationInputs,
    pub later_sequence: u64,
}

pub fn setup(db: &mut dl::Fixture) -> Scenario {
    let (source, base) = worker::accepted(db, 60);
    let changed = dispatch::advance(db, &source);
    let sequence = dispatch::event_sequence(db, &changed);
    let batch = dispatch::dispatch(&dispatch::open(db), DeadlineDispatchStream::Events, 20);
    assert_eq!((batch.selected, batch.inserted), (1, 1));
    assert_eq!(batch.event.unwrap().sequence, sequence);
    let mut jobs = worker::jobs(db);
    assert_eq!(jobs.len(), 1);
    let job = jobs.pop().unwrap();
    let head = dispatch::advance(db, &changed);
    let later_sequence = dispatch::event_sequence(db, &head);
    assert_ne!(sequence, later_sequence);
    let reference = facts::resolution_ref(&changed);
    let command = DeadlineReevaluationCommand {
        operation_id: job.operation_id,
        cause: TechnicalCause::SourceEvent {
            job_id: job.id,
            event: SourceEventReference {
                sequence,
                family: DependencyFamily::Resolution,
                source_id: reference.id.as_uuid(),
                revision: reference.revision.get(),
                case_id: Some(db.case),
                hearing_id: None,
                operation_id: changed.snapshot.metadata().receipt.operation_id.as_uuid(),
            },
        },
    };
    let mut material = base.calculation.material.clone();
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(head)));
    let inputs = DeadlineReevaluationInputs {
        profile_head: base.calculation.profile.clone(),
        material,
        notification_parent_head: None,
    };
    let expected = prepare_value(db, &base, command.clone(), inputs.clone());
    Scenario {
        base,
        expected,
        job,
        command,
        inputs,
        later_sequence,
    }
}

pub fn prepare(
    db: &dl::Fixture,
    scenario: &Scenario,
    command: DeadlineReevaluationCommand,
) -> DeadlineDetail {
    prepare_value(db, &scenario.base, command, scenario.inputs.clone())
}

fn prepare_value(
    db: &dl::Fixture,
    base: &DeadlineDetail,
    command: DeadlineReevaluationCommand,
    inputs: DeadlineReevaluationInputs,
) -> DeadlineDetail {
    let DeadlineReevaluationOutcome::Revision(prepared) =
        prepare_technical_deadline_change(&RingSha256Hasher, base, command, inputs).unwrap()
    else {
        panic!("a followed changed source must require a revision")
    };
    let value = prepared.record(db.at);
    deadline_receipt_matches(&RingSha256Hasher, &value).unwrap();
    deadline_successor_matches(&RingSha256Hasher, base, &value).unwrap();
    value
}

pub fn complete_and_reopen(db: &mut dl::Fixture, scenario: &Scenario) {
    let runner = worker::open(db);
    assert!(matches!(
        runner.run_next().unwrap(),
        DeadlineWorkerRun::Completed(_)
    ));
    let actual = worker::current(db, scenario.base.id);
    assert_eq!(actual, scenario.expected);
    worker::assert_result(db, &scenario.job, &scenario.base, Some(&actual), "revision");
    worker::assert_technical(
        &scenario.base,
        &actual,
        &scenario.job,
        scenario.command.cause,
    );
    drop(runner);
    // Full startup must allow only the authentic completed operation reservation.
    // Validate stored inventory without applying another migration.
    drop(db.store());
    let reopened = worker::open(db);
    assert!(reopened.result(scenario.job.id).unwrap().is_some());
    tracked::assert_readback(db, dl::store(db).as_ref(), &[scenario.base.clone(), actual]);
    worker::assert_idle(db, &reopened);
}

pub fn assert_check(error: &Error) {
    assert_eq!(
        error.as_db_error().expect("SQL guard rejection").code(),
        &SqlState::CHECK_VIOLATION
    );
}

pub fn data_snapshot(db: &mut dl::Fixture) -> Value {
    let mut value = worker::snapshot(db);
    value.as_object_mut().unwrap().remove("audit");
    value
}

pub fn audit_count(db: &mut dl::Fixture) -> i64 {
    db.admin
        .query_one("SELECT count(*) FROM audit_events", &[])
        .unwrap()
        .get(0)
}

pub fn open_at(db: &dl::Fixture, at: OffsetDateTime) -> PostgresDeadlineWorkerStore {
    PostgresDeadlineWorkerStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(at)),
    )
    .unwrap()
}

pub fn reject_completion_audit(db: &mut dl::Fixture) {
    db.admin
        .batch_execute(&format!(
            "CREATE SEQUENCE worker_fault_reached;
        GRANT USAGE ON SEQUENCE worker_fault_reached TO {};
        CREATE FUNCTION reject_worker_completion_audit() RETURNS trigger
        LANGUAGE plpgsql AS $$ BEGIN
        IF NEW.action <> 'deadline.reevaluated' THEN RETURN NEW; END IF;
        IF NOT EXISTS(SELECT 1 FROM case_deadline_revisions r
            JOIN deadline_reevaluation_jobs j ON j.operation_id=r.operation_id
            JOIN deadline_reevaluation_results v ON v.job_id=j.id
            WHERE r.action='reevaluate' AND v.outcome='revision'
                AND v.result_revision=r.revision
                AND v.result_submission_digest=r.submission_digest
                AND v.result_capture_digest=r.capture_digest) THEN
            RAISE EXCEPTION 'completion audit preceded matching revision and result';
        END IF;
        PERFORM nextval('worker_fault_reached');
        RAISE EXCEPTION 'injected completion audit failure';
        END; $$;
        CREATE TRIGGER reject_worker_completion_audit BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_worker_completion_audit()",
            db.role
        ))
        .unwrap();
}

pub fn fault_reached(db: &mut dl::Fixture) -> bool {
    db.admin
        .query_one("SELECT is_called FROM worker_fault_reached", &[])
        .unwrap()
        .get(0)
}

pub fn allow_completion_audit(db: &mut dl::Fixture) {
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_worker_completion_audit ON audit_events;
        DROP FUNCTION reject_worker_completion_audit(); DROP SEQUENCE worker_fault_reached",
        )
        .unwrap();
}

pub fn assert_attempt(
    db: &mut dl::Fixture,
    scenario: &Scenario,
    attempt: &DeadlineWorkerAttempt,
    previous_audits: i64,
) {
    let rows = db
        .admin
        .query(
            "SELECT attempt_id,attempt_number FROM deadline_reevaluation_attempts
        WHERE job_id=$1",
            &[&scenario.job.id],
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, uuid::Uuid>(0), attempt.attempt_id);
    assert_eq!(rows[0].get::<_, i64>(1), 1);
    let base = attempt
        .checked_base
        .as_ref()
        .expect("the failed audit followed verified writes");
    assert_eq!(base.revision, scenario.base.revision);
    assert_eq!(
        base.receipt.submission_digest,
        scenario.base.receipt.submission_digest
    );
    assert_eq!(
        base.receipt.capture_digest,
        scenario.base.receipt.capture_digest
    );
    assert_eq!(audit_count(db), previous_audits + 1);
    let row = db
        .admin
        .query_one(
            "SELECT actor,resource FROM audit_events
        WHERE action='deadline.reevaluation_deferred'",
            &[],
        )
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "deadline_reevaluator");
    let resource: String = row.get(1);
    for expected in [
        scenario.job.id.to_string(),
        attempt.attempt_id.to_string(),
        scenario.job.operation_id.to_string(),
        scenario.base.receipt.submission_digest.to_hex(),
        scenario.base.receipt.capture_digest.to_hex(),
    ] {
        assert!(
            resource.contains(&expected),
            "attempt audit omits {expected}"
        );
    }
    let completions: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='deadline.reevaluated'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(completions, 0);
}
