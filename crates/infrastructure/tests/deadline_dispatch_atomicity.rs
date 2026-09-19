mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_dispatch_atomic_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
mod deadline_tracked_guard_support;
mod procedural_fact_backend_support;

use application::{deadline_dispatch::*, deadlines::*, ApplicationError};
use deadline_backend_support as dl;
use deadline_dispatch_atomic_support as atomic;
use deadline_dispatch_support as dispatch;
use deadline_tracked_backend_support as tracked;
use deadline_tracked_guard_support as guards;
use infrastructure::RingSha256Hasher;
use std::sync::{Arc, Barrier};
use uuid::Uuid;

#[test]
fn two_real_dispatchers_expand_each_event_once_without_losing_cursor_progress() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (_, source, _) = atomic::seed(&db, &[10, 20, 30, 40, 50]);
    let initial = dispatch::open(&db);
    dispatch::drain_existing(&mut db, &initial);
    drop(initial);
    let second = dispatch::advance(&db, &source);
    let third = dispatch::advance(&db, &second);
    let sequences = [
        dispatch::event_sequence(&mut db, &second),
        dispatch::event_sequence(&mut db, &third),
    ];
    let stores = [dispatch::open(&db), dispatch::open(&db)];
    let barrier = Arc::new(Barrier::new(3));
    let audit_before = atomic::dispatch_audit_count(&mut db);
    let history_before = dispatch::deadline_history(&mut db);
    let mut workers = Vec::new();
    for store in stores {
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            (0..6)
                .map(|_| store.dispatch(atomic::request(DeadlineDispatchStream::Events, 1)))
                .collect::<Vec<_>>()
        }));
    }
    barrier.wait();
    let mut selected = 0;
    let mut inserted = 0;
    for worker in workers {
        for result in worker.join().unwrap() {
            let batch =
                result.expect("concurrent pages must serialize without stale cursor errors");
            assert!(batch.selected <= 1);
            if let Some(event) = batch.event {
                assert!(sequences.contains(&event.sequence));
            }
            selected += batch.selected;
            inserted += batch.inserted;
        }
    }
    assert_eq!((selected, inserted), (10, 10));
    for sequence in sequences {
        assert_eq!(
            dispatch::event_jobs(&mut db, sequence),
            [10, 20, 30, 40, 50]
                .into_iter()
                .map(dispatch::id)
                .collect::<Vec<_>>()
        );
    }
    let sequence_values = sequences
        .map(|value| i64::try_from(value).unwrap())
        .to_vec();
    let counts = db
        .admin
        .query_one(
            "SELECT count(*),count(DISTINCT operation_id),count(DISTINCT id)
         FROM deadline_reevaluation_jobs WHERE event_sequence=ANY($1)",
            &[&sequence_values],
        )
        .unwrap();
    assert_eq!(
        (
            counts.get::<_, i64>(0),
            counts.get::<_, i64>(1),
            counts.get::<_, i64>(2)
        ),
        (10, 10, 10)
    );
    assert_eq!(atomic::dispatch_audit_count(&mut db) - audit_before, 10);
    assert_eq!(dispatch::deadline_history(&mut db), history_before);
    let reopened = dispatch::open(&db);
    let before = atomic::snapshot(&mut db);
    let idle = dispatch::dispatch(&reopened, DeadlineDispatchStream::Events, 1);
    assert!(idle.event.is_none());
    assert_eq!(idle.progress.event.completed_sequence, Some(sequences[1]));
    assert!(idle.progress.event.active_sequence.is_none());
    assert!(idle.progress.event.after_deadline_id.is_none());
    assert_eq!(atomic::snapshot(&mut db), before);
}

#[test]
fn audit_failure_after_job_and_cursor_writes_rolls_back_the_complete_batch() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    atomic::seed(&db, &[10, 20, 30]);
    let store = dispatch::open(&db);
    let before = atomic::snapshot(&mut db);
    atomic::reject_audit_after_writes(&mut db);
    let result = store.dispatch(atomic::request(DeadlineDispatchStream::LegacyBootstrap, 1));
    assert!(matches!(result, Err(ApplicationError::Port(_))));
    assert!(
        atomic::fault_reached(&mut db),
        "audit must see both earlier writes before failing"
    );
    assert_eq!(atomic::snapshot(&mut db), before);
    atomic::allow_audit(&mut db);
    let retry = dispatch::dispatch(&store, DeadlineDispatchStream::LegacyBootstrap, 1);
    assert_eq!((retry.selected, retry.inserted), (1, 1));
    assert_eq!(
        retry.progress.bootstrap_after_deadline_id,
        Some(dispatch::id(10))
    );
    assert_eq!(dispatch::bootstrap_jobs(&mut db), vec![dispatch::id(10)]);
}

#[test]
fn a_job_reserved_operation_rejects_human_commit_and_direct_sql_with_exact_constraint() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (profile, source, _) = atomic::seed(&db, &[10]);
    let repository = dl::store(&db);
    let mut command = dispatch::command(&db, &profile, &source, 20);
    let inputs = repository.prepare(db.owner, db.case, &command).unwrap();
    let store = dispatch::open(&db);
    dispatch::dispatch(&store, DeadlineDispatchStream::LegacyBootstrap, 100);
    let reserved: Uuid = db
        .admin
        .query_one(
            "SELECT operation_id FROM deadline_reevaluation_jobs WHERE deadline_id=$1",
            &[&dispatch::id(10).as_uuid()],
        )
        .unwrap()
        .get(0);
    command.operation_id = DeadlineOperationId::from_uuid(reserved);
    let prepared = prepare_tracked_deadline_change(
        &RingSha256Hasher,
        DeadlineActorSnapshot::User {
            id: db.owner,
            email: "owner@example.test".into(),
        },
        db.case,
        command.clone(),
        inputs,
        Some(tracked::policies()),
        None,
    )
    .unwrap();
    let detail = tracked::prepared_detail(&db, &prepared);
    deadline_receipt_matches(&RingSha256Hasher, &detail).unwrap();
    let before = atomic::snapshot(&mut db);
    assert!(matches!(
        repository.commit(db.owner, prepared),
        Err(ApplicationError::Deadline(DeadlineError::OperationConflict))
    ));
    assert_eq!(atomic::snapshot(&mut db), before);
    let error = guards::insert(&db, &detail).unwrap_err();
    atomic::assert_unique(&error, "deadline_job_operation");
    assert_eq!(atomic::snapshot(&mut db), before);
    command.operation_id = DeadlineOperationId::new();
    let fresh = tracked::tracked_prepared(
        &db,
        repository.as_ref(),
        &command,
        Some(tracked::policies()),
    );
    assert_eq!(
        repository.commit(db.owner, fresh).unwrap().id,
        dispatch::id(20)
    );
}

#[test]
fn a_human_operation_rejects_dispatcher_and_direct_job_insert_without_moving_cursor() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (_, _, rows) = atomic::seed(&db, &[10]);
    let used = rows[0].receipt.operation_id.as_uuid();
    let store = dispatch::open(&db);
    let before = atomic::snapshot(&mut db);
    atomic::force_job_operation(&mut db, used);
    assert!(matches!(
        store.dispatch(atomic::request(
            DeadlineDispatchStream::LegacyBootstrap,
            100
        )),
        Err(ApplicationError::Deadline(DeadlineError::OperationConflict))
    ));
    assert_eq!(atomic::snapshot(&mut db), before);
    let error = atomic::insert_bootstrap(&db, dispatch::id(10), used).unwrap_err();
    atomic::assert_unique(&error, "deadline_operation_unique");
    assert_eq!(atomic::snapshot(&mut db), before);
    atomic::stop_forcing_operation(&mut db);
    let retry = dispatch::dispatch(&store, DeadlineDispatchStream::LegacyBootstrap, 100);
    assert_eq!((retry.selected, retry.inserted), (1, 1));
    let operation: Uuid = db
        .admin
        .query_one(
            "SELECT operation_id FROM deadline_reevaluation_jobs WHERE deadline_id=$1",
            &[&dispatch::id(10).as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_ne!(operation, used);
}

#[test]
fn reopen_after_a_discarded_committed_batch_keeps_job_identity_and_continues_once() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    atomic::seed(&db, &[10, 20, 30]);
    let store = dispatch::open(&db);
    // Deliberately discard the committed response before reopening the adapter.
    store
        .dispatch(atomic::request(DeadlineDispatchStream::LegacyBootstrap, 2))
        .unwrap();
    let committed = atomic::snapshot(&mut db);
    let first_jobs = atomic::jobs(&mut db);
    assert_eq!(first_jobs.as_array().unwrap().len(), 2);
    drop(store);
    let reopened = dispatch::open(&db);
    assert_eq!(atomic::snapshot(&mut db), committed);
    let next = dispatch::dispatch(&reopened, DeadlineDispatchStream::LegacyBootstrap, 2);
    assert_eq!((next.selected, next.inserted), (1, 1));
    assert!(next.completed_scan);
    assert!(next.progress.bootstrap_after_deadline_id.is_none());
    let all_jobs = atomic::jobs(&mut db);
    assert_eq!(all_jobs.as_array().unwrap().len(), 3);
    assert_eq!(
        &all_jobs.as_array().unwrap()[..2],
        first_jobs.as_array().unwrap().as_slice()
    );
    let before = atomic::snapshot(&mut db);
    drop(reopened);
    let final_store = dispatch::open(&db);
    let idle = dispatch::dispatch(&final_store, DeadlineDispatchStream::LegacyBootstrap, 100);
    assert_eq!((idle.selected, idle.inserted), (0, 0));
    assert_eq!(atomic::snapshot(&mut db), before);
}
