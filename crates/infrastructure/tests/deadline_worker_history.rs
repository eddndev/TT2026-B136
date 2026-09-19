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
mod deadline_worker_extra_support;
#[allow(dead_code)]
mod deadline_worker_retry_support;
mod procedural_fact_backend_support;

use application::{
    deadline_reevaluation::{ObservationRole, TechnicalCause},
    deadline_technical::DeadlineReevaluationNoChange,
    deadline_worker::*,
};
use deadline_backend_support as dl;
use deadline_dispatch_support as dispatch;
use deadline_worker_backend_support as worker;
use deadline_worker_extra_support as extra;
use deadline_worker_retry_support as retry;
use procedural_fact_backend_support as facts;

#[test]
fn already_observed_records_later_checked_heads_and_replays_after_a_human_successor() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (source, first) = worker::accepted(&mut db, 820);
    let second = dispatch::advance(&db, &source);
    let (job, event) = extra::event_job(
        &mut db,
        second.snapshot.metadata().receipt.operation_id.as_uuid(),
    );
    let [corrected, base] = retry::correct_and_attend(&db, &first, &second, db.owner);
    let third = dispatch::advance(&db, &second);
    let before = dispatch::deadline_history(&mut db);
    let runner = worker::open(&db);
    let result = retry::completed(runner.run_next().unwrap());
    assert_eq!(
        result.command.cause,
        TechnicalCause::SourceEvent {
            job_id: job.id,
            event
        }
    );
    let DeadlineWorkerOutcome::NoChange { reason, checked } = &result.outcome else {
        panic!("an already observed event must not create a revision")
    };
    assert_eq!(*reason, DeadlineReevaluationNoChange::AlreadyObserved);
    let checked = checked.as_ref().unwrap();
    let observed = checked
        .observations
        .entries
        .iter()
        .find(|e| e.role == ObservationRole::Source)
        .unwrap();
    assert_eq!(observed.revision, 3);
    assert_eq!(
        observed.submission_digest,
        third.snapshot.metadata().receipt.submission_digest
    );
    assert_eq!(extra::observed(&base, ObservationRole::Source).revision, 2);
    assert_ne!(
        checked.observations,
        base.tracking.as_ref().unwrap().observations
    );
    assert_eq!(dispatch::deadline_history(&mut db), before);
    worker::assert_result(&mut db, &job, &base, None, "already_observed");
    let saved_row = worker::result_row(&mut db, &job);
    drop(runner);
    let [later, last] = retry::correct_and_attend(&db, &base, &third, db.owner);
    dispatch::advance(&db, &third);
    extra::history(&mut db, &[first, corrected, base, later, last], &[*result]);
    assert_eq!(worker::result_row(&mut db, &job), saved_row);
    retry::assert_counts(&mut db, 0, 1, 0);
}

#[test]
fn deselected_event_authenticates_old_cause_and_captures_the_new_sources_later_head() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (old_source, first) = worker::accepted(&mut db, 821);
    let chosen = dl::source(&db);
    extra::clear_events(&mut db);
    let old_changed = dispatch::advance(&db, &old_source);
    let (job, event) = extra::event_job(
        &mut db,
        old_changed
            .snapshot
            .metadata()
            .receipt
            .operation_id
            .as_uuid(),
    );
    let [corrected, base] = retry::correct_and_attend(&db, &first, &chosen, db.owner);
    let chosen_head = dispatch::advance(&db, &chosen);
    let history = dispatch::deadline_history(&mut db);
    let runner = worker::open(&db);
    let result = retry::completed(runner.run_next().unwrap());
    let DeadlineWorkerOutcome::NoChange { reason, checked } = &result.outcome else {
        panic!("deselected dependency must not create a revision")
    };
    assert_eq!(*reason, DeadlineReevaluationNoChange::DependencyNotSelected);
    assert_eq!(
        result.command.cause,
        TechnicalCause::SourceEvent {
            job_id: job.id,
            event
        }
    );
    let observed = checked
        .as_ref()
        .unwrap()
        .observations
        .entries
        .iter()
        .find(|e| e.role == ObservationRole::Source)
        .unwrap();
    assert_eq!(observed.id, facts::resolution_ref(&chosen).id.as_uuid());
    assert_eq!(
        observed.revision,
        chosen_head.snapshot.metadata().revision.get()
    );
    assert_eq!(
        observed.submission_digest,
        chosen_head.snapshot.metadata().receipt.submission_digest
    );
    assert_ne!(observed.id, event.source_id);
    assert_eq!(dispatch::deadline_history(&mut db), history);
    worker::assert_result(&mut db, &job, &base, None, "dependency_not_selected");
    let saved = worker::result_row(&mut db, &job);
    drop(runner);
    dispatch::advance(&db, &old_changed);
    dispatch::advance(&db, &chosen_head);
    extra::history(&mut db, &[first, corrected, base], &[*result]);
    assert_eq!(worker::result_row(&mut db, &job), saved);
    retry::assert_counts(&mut db, 0, 1, 0);
}
