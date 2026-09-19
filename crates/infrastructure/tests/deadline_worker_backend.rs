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
mod deadline_worker_backend_support;
mod procedural_fact_backend_support;

use application::{
    deadline_dispatch::DeadlineDispatchStream,
    deadline_reevaluation::{
        DependencyFamily, ObservationRole, SourceEventReference, TechnicalCause,
    },
    deadline_tracking::{
        DeadlineReviewState, TrackingDependency, TrackingPolicy, TrackingReviewReason,
    },
    deadline_worker::{DeadlineWorkerRun, DeadlineWorkerStore},
    deadlines::*,
};
use deadline_backend_support as dl;
use deadline_dispatch_support as dispatch;
use deadline_tracked_backend_support as tracked;
use deadline_worker_backend_support as worker;
use domain::identity::Role;
use procedural_fact_backend_support as facts;

#[test]
fn worker_bootstraps_v1_without_replacing_calculation_attention_or_responsible() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let responsible = db.user("paralegal", true);
    let (profile, source) = worker::inputs(&mut db);
    let mut command = dispatch::command(&db, &profile, &source, 10);
    dl::definition_mut(&mut command).responsible = responsible;
    let workflow = dl::service(&db, db.owner, Role::Owner);
    let first = dl::persist(&workflow, db.case, command);
    let base = dl::persist(&workflow, db.case, dl::attention(&first));
    assert_eq!(base.receipt.version, DeadlineReceiptVersion::Legacy);
    assert!(matches!(base.attention, DeadlineAttention::Recorded { .. }));
    let old_rows = [
        tracked::revision_row(&mut db, &first),
        tracked::revision_row(&mut db, &base),
    ];
    let users = tracked::users(&mut db);
    let batch = dispatch::dispatch(
        &dispatch::open(&db),
        DeadlineDispatchStream::LegacyBootstrap,
        20,
    );
    assert_eq!((batch.selected, batch.inserted), (1, 1));
    let jobs = worker::jobs(&mut db);
    assert_eq!(jobs.len(), 1);
    let job = &jobs[0];
    let runner = worker::open(&db);
    assert!(runner.result(job.id).unwrap().is_none());
    assert!(matches!(
        runner.run_next().unwrap(),
        DeadlineWorkerRun::Completed(_)
    ));
    let actual = worker::current(&db, base.id);
    worker::assert_preserved(&base, &actual);
    worker::assert_technical(
        &base,
        &actual,
        job,
        TechnicalCause::LegacyBootstrap {
            job_id: job.id,
            policy_version: 1,
        },
    );
    assert_eq!(actual.review_state(), DeadlineReviewState::Pending);
    assert!(actual.operational_due_at().is_none());
    let capture = actual.tracking.as_ref().unwrap();
    assert_eq!(
        [
            capture.policies.profile,
            capture.policies.source,
            capture.policies.calendar
        ],
        [TrackingPolicy::Undetermined; 3]
    );
    assert_eq!(
        capture
            .review
            .reasons()
            .iter()
            .map(|r| (r.dependency, r.reason))
            .collect::<Vec<_>>(),
        vec![
            (
                TrackingDependency::Profile,
                TrackingReviewReason::PolicyUndetermined
            ),
            (
                TrackingDependency::Source,
                TrackingReviewReason::PolicyUndetermined
            )
        ]
    );
    assert_eq!(actual.responsible.id, responsible);
    assert_eq!(tracked::users(&mut db), users);
    assert_eq!(tracked::revision_row(&mut db, &first), old_rows[0]);
    assert_eq!(tracked::revision_row(&mut db, &base), old_rows[1]);
    worker::assert_result(&mut db, job, &base, Some(&actual), "revision");
    assert!(runner.result(job.id).unwrap().is_some());
    tracked::assert_readback(&db, dl::store(&db).as_ref(), &[first, base, actual]);
    worker::assert_idle(&mut db, &runner);
}

#[test]
fn worker_persists_exact_source_event_cause_and_predecessor_receipts() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (source, base) = worker::accepted(&mut db, 20);
    let original = tracked::revision_row(&mut db, &base);
    let changed = dispatch::advance(&db, &source);
    let sequence = dispatch::event_sequence(&mut db, &changed);
    let batch = dispatch::dispatch(&dispatch::open(&db), DeadlineDispatchStream::Events, 20);
    assert_eq!((batch.selected, batch.inserted), (1, 1));
    assert_eq!(batch.event.unwrap().sequence, sequence);
    let jobs = worker::jobs(&mut db);
    assert_eq!(jobs.len(), 1);
    let job = &jobs[0];
    let runner = worker::open(&db);
    assert!(matches!(
        runner.run_next().unwrap(),
        DeadlineWorkerRun::Completed(_)
    ));
    let actual = worker::current(&db, base.id);
    let reference = facts::resolution_ref(&changed);
    let event = SourceEventReference {
        sequence,
        family: DependencyFamily::Resolution,
        source_id: reference.id.as_uuid(),
        revision: reference.revision.get(),
        case_id: Some(db.case),
        hearing_id: None,
        operation_id: changed.snapshot.metadata().receipt.operation_id.as_uuid(),
    };
    worker::assert_technical(
        &base,
        &actual,
        job,
        TechnicalCause::SourceEvent {
            job_id: job.id,
            event,
        },
    );
    worker::assert_preserved(&base, &actual);
    assert_eq!(actual.review_state(), DeadlineReviewState::Pending);
    assert!(actual.operational_due_at().is_none());
    let tracking = actual.tracking.as_ref().unwrap();
    assert_eq!(tracking.policies, tracked::policies());
    assert_eq!(
        tracking
            .review
            .reasons()
            .iter()
            .map(|r| (r.dependency, r.reason))
            .collect::<Vec<_>>(),
        vec![(
            TrackingDependency::Source,
            TrackingReviewReason::SourceChanged
        )]
    );
    let observed = tracking
        .observations
        .entries
        .iter()
        .find(|e| e.role == ObservationRole::Source)
        .unwrap();
    assert_eq!(observed.revision, reference.revision.get());
    assert_eq!(
        observed.submission_digest,
        changed.snapshot.metadata().receipt.submission_digest
    );
    worker::assert_result(&mut db, job, &base, Some(&actual), "revision");
    assert_eq!(tracked::revision_row(&mut db, &base), original);
    let exact_revision_row = tracked::revision_row(&mut db, &actual);
    dispatch::advance(&db, &changed);
    tracked::assert_readback(&db, dl::store(&db).as_ref(), &[base, actual.clone()]);
    assert_eq!(tracked::revision_row(&mut db, &actual), exact_revision_row);
    assert!(runner.result(job.id).unwrap().is_some());
    worker::assert_idle(&mut db, &runner);
}

#[test]
fn retired_after_dispatch_records_no_change_without_another_revision() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (source, first) = worker::accepted(&mut db, 30);
    dispatch::advance(&db, &source);
    let batch = dispatch::dispatch(&dispatch::open(&db), DeadlineDispatchStream::Events, 20);
    assert_eq!((batch.selected, batch.inserted), (1, 1));
    let store = dl::store(&db);
    let prepared = tracked::tracked_prepared(&db, store.as_ref(), &dl::retire(&first), None);
    let retired = store.commit(db.owner, prepared).unwrap();
    assert_eq!(retired.status, DeadlineStatus::Retired);
    let before = dispatch::deadline_history(&mut db);
    let jobs = worker::jobs(&mut db);
    assert_eq!(jobs.len(), 1);
    let job = &jobs[0];
    let runner = worker::open(&db);
    assert!(matches!(
        runner.run_next().unwrap(),
        DeadlineWorkerRun::Completed(_)
    ));
    assert_eq!(worker::current(&db, first.id), retired);
    assert_eq!(dispatch::deadline_history(&mut db), before);
    worker::assert_result(&mut db, job, &retired, None, "retired");
    let used: bool = db
        .admin
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_deadline_revisions WHERE operation_id=$1)",
            &[&job.operation_id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert!(
        !used,
        "a no-change result must not invent a technical revision"
    );
    let checked: bool = db.admin.query_one(
        "SELECT checked_observations_canonical IS NULL AND checked_administration_revision IS NULL
            AND checked_administration_evidence_digest IS NULL
         FROM deadline_reevaluation_results WHERE job_id=$1", &[&job.id],
    ).unwrap().get(0);
    assert!(
        checked,
        "retired outcome must not fabricate examined dependency heads"
    );
    assert!(runner.result(job.id).unwrap().is_some());
    tracked::assert_readback(&db, store.as_ref(), &[first, retired]);
    worker::assert_idle(&mut db, &runner);
}

#[test]
fn worker_reopens_between_jobs_and_replays_committed_results_without_duplicates() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (profile, source) = worker::inputs(&mut db);
    let bases = [
        dispatch::legacy(&db, &profile, &source, 40),
        dispatch::legacy(&db, &profile, &source, 50),
    ];
    let batch = dispatch::dispatch(
        &dispatch::open(&db),
        DeadlineDispatchStream::LegacyBootstrap,
        20,
    );
    assert_eq!((batch.selected, batch.inserted), (2, 2));
    let jobs = worker::jobs(&mut db);
    assert_eq!(jobs.len(), 2);
    let initial = worker::open(&db);
    assert!(matches!(
        initial.run_next().unwrap(),
        DeadlineWorkerRun::Completed(_)
    ));
    drop(initial);
    assert_eq!(worker::result_count(&mut db), 1);
    let finished: uuid::Uuid = db
        .admin
        .query_one("SELECT job_id FROM deadline_reevaluation_results", &[])
        .unwrap()
        .get(0);
    let first_job = jobs.iter().find(|job| job.id == finished).unwrap();
    let first_result = worker::result_row(&mut db, first_job);
    let before_reopen = worker::snapshot(&mut db);
    db.migrate();
    let reopened = worker::open(&db);
    assert!(reopened.result(first_job.id).unwrap().is_some());
    assert_eq!(worker::snapshot(&mut db), before_reopen);
    assert!(matches!(
        reopened.run_next().unwrap(),
        DeadlineWorkerRun::Completed(_)
    ));
    assert_eq!(worker::result_count(&mut db), 2);
    assert_eq!(worker::result_row(&mut db, first_job), first_result);
    let counts = db.admin.query_one(
        "SELECT count(*),count(DISTINCT operation_id) FROM case_deadline_revisions WHERE action='reevaluate'", &[],
    ).unwrap();
    assert_eq!((counts.get::<_, i64>(0), counts.get::<_, i64>(1)), (2, 2));
    for base in bases {
        let job = jobs.iter().find(|job| job.deadline_id == base.id).unwrap();
        let actual = worker::current(&db, base.id);
        worker::assert_preserved(&base, &actual);
        worker::assert_technical(
            &base,
            &actual,
            job,
            TechnicalCause::LegacyBootstrap {
                job_id: job.id,
                policy_version: 1,
            },
        );
        worker::assert_result(&mut db, job, &base, Some(&actual), "revision");
        assert!(reopened.result(job.id).unwrap().is_some());
        tracked::assert_readback(&db, dl::store(&db).as_ref(), &[base, actual]);
    }
    worker::assert_idle(&mut db, &reopened);
    let final_snapshot = worker::snapshot(&mut db);
    drop(reopened);
    let last = worker::open(&db);
    for job in jobs {
        assert!(last.result(job.id).unwrap().is_some());
    }
    worker::assert_idle(&mut db, &last);
    assert_eq!(worker::snapshot(&mut db), final_snapshot);
}
