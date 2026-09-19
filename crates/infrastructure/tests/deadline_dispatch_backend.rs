mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_dispatch_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
mod procedural_fact_backend_support;

use application::{
    deadline_dispatch::*, deadline_reevaluation::DependencyFamily,
    deadline_tracking::DeadlineReviewState, deadlines::*, procedural_facts::FactDeclaration,
};
use deadline_backend_support as dl;
use deadline_dispatch_support as dispatch;
use deadline_tracked_backend_support as tracked;
use domain::deadline_triggers::TriggerSourceRef;
use procedural_fact_backend_support as facts;
use uuid::Uuid;

#[test]
fn dispatch_limits_reject_zero_and_values_above_one_hundred() {
    for invalid in [0, 101, u32::MAX] {
        assert!(DeadlineDispatchLimit::new(invalid).is_err());
    }
    for valid in [1, 20, 100] {
        assert!(DeadlineDispatchLimit::new(valid).is_ok());
    }
}

#[test]
fn event_pages_use_exclusive_uuid_and_current_heads_across_reopen() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let profile = dl::profile(&db);
    let source = dl::source(&db);
    let other = dl::source(&db);
    for value in [0, 20, 40, 60, 80] {
        dispatch::legacy(&db, &profile, &source, value);
    }
    dispatch::legacy(&db, &profile, &other, 10);
    let old_match = dispatch::legacy(&db, &profile, &source, 30);
    let mut correction = dl::correct(&old_match);
    dl::definition_mut(&mut correction).input.selection.source =
        FactDeclaration::Known(TriggerSourceRef::Resolution(facts::resolution_ref(&other)));
    dl::persist_legacy(&db, db.owner, correction);
    let initial = dispatch::open(&db);
    dispatch::drain_existing(&mut db, &initial);
    drop(initial);
    let head = dispatch::advance(&db, &source);
    let sequence = dispatch::event_sequence(&mut db, &head);
    let history_before = dispatch::deadline_history(&mut db);
    let mut seen = Vec::new();
    for (count, after) in [(2, Some(20)), (2, Some(60)), (1, None)] {
        let store = dispatch::open(&db);
        let batch = dispatch::dispatch(&store, DeadlineDispatchStream::Events, 2);
        let event = batch.event.unwrap();
        assert_eq!(event.sequence, sequence);
        assert_eq!(event.family, DependencyFamily::Resolution);
        assert_eq!(event.source_id, facts::resolution_ref(&source).id.as_uuid());
        assert_eq!(event.revision, 2);
        assert_eq!(
            event.operation_id,
            head.snapshot.metadata().receipt.operation_id.as_uuid()
        );
        assert_eq!(batch.selected, count);
        assert_eq!(batch.inserted, count);
        assert_eq!(batch.completed_scan, after.is_none());
        assert_eq!(
            batch.progress.event.active_sequence,
            after.map(|_| sequence)
        );
        assert_eq!(
            batch.progress.event.after_deadline_id,
            after.map(dispatch::id)
        );
        if after.is_none() {
            assert_eq!(batch.progress.event.completed_sequence, Some(sequence));
        } else {
            assert!(batch
                .progress
                .event
                .completed_sequence
                .is_none_or(|old| old < sequence));
        }
        seen.extend(match after {
            Some(20) => vec![dispatch::id(0), dispatch::id(20)],
            Some(60) => vec![dispatch::id(40), dispatch::id(60)],
            None => vec![dispatch::id(80)],
            _ => unreachable!(),
        });
        assert_eq!(dispatch::event_jobs(&mut db, sequence), seen);
        let before = dispatch::snapshot(&mut db);
        drop(store);
        db.migrate();
        let reopened = dispatch::open(&db);
        assert_eq!(dispatch::snapshot(&mut db), before);
        drop(reopened);
    }
    let rows = db
        .admin
        .query(
            "SELECT id,operation_id,case_id,bootstrap_policy_version,
                created_at_seconds,created_at_nanoseconds
         FROM deadline_reevaluation_jobs WHERE event_sequence=$1 ORDER BY deadline_id",
            &[&i64::try_from(sequence).unwrap()],
        )
        .unwrap();
    assert_eq!(rows.len(), 5);
    let mut operations = std::collections::BTreeSet::new();
    let mut jobs = std::collections::BTreeSet::new();
    for row in rows {
        jobs.insert(row.get::<_, Uuid>(0));
        operations.insert(row.get::<_, Uuid>(1));
        assert_eq!(row.get::<_, Uuid>(2), db.case.as_uuid());
        assert_eq!(row.get::<_, Option<i16>>(3), None);
        assert_eq!(row.get::<_, i64>(4), db.at.unix_timestamp());
        assert_eq!(
            row.get::<_, i32>(5),
            i32::try_from(db.at.nanosecond()).unwrap()
        );
    }
    assert_eq!(jobs.len(), 5);
    assert_eq!(operations.len(), 5);
    assert_eq!(dispatch::deadline_history(&mut db), history_before);
    let store = dispatch::open(&db);
    let before = dispatch::snapshot(&mut db);
    let idle = dispatch::dispatch(&store, DeadlineDispatchStream::Events, 2);
    assert!(idle.event.is_none());
    assert_eq!((idle.selected, idle.inserted), (0, 0));
    assert_eq!(dispatch::snapshot(&mut db), before);
}

#[test]
fn rollback_sequence_gaps_and_empty_events_do_not_stall_or_skip_dispatch() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let profile = dl::profile(&db);
    let source = dl::source(&db);
    for value in [20, 40, 60] {
        dispatch::legacy(&db, &profile, &source, value);
    }
    let store = dispatch::open(&db);
    dispatch::drain_existing(&mut db, &store);
    let unrelated = dl::source(&db);
    let empty_sequence = dispatch::event_sequence(&mut db, &unrelated);
    let gap = dispatch::rolled_back_source_event(&mut db, &source);
    let changed = dispatch::advance(&db, &source);
    let later_sequence = dispatch::event_sequence(&mut db, &changed);
    assert!(empty_sequence < u64::try_from(gap).unwrap());
    assert!(later_sequence > u64::try_from(gap).unwrap());
    let empty = dispatch::dispatch(&store, DeadlineDispatchStream::Events, 100);
    assert_eq!(empty.event.unwrap().sequence, empty_sequence);
    assert_eq!((empty.selected, empty.inserted), (0, 0));
    assert!(empty.completed_scan);
    assert_eq!(
        empty.progress.event.completed_sequence,
        Some(empty_sequence)
    );
    assert!(empty.progress.event.active_sequence.is_none());
    assert!(empty.progress.event.after_deadline_id.is_none());
    assert!(dispatch::event_jobs(&mut db, empty_sequence).is_empty());
    let later = dispatch::dispatch(&store, DeadlineDispatchStream::Events, 100);
    assert_eq!(later.event.unwrap().sequence, later_sequence);
    assert_eq!((later.selected, later.inserted), (3, 3));
    assert!(later.completed_scan);
    assert_eq!(
        later.progress.event.completed_sequence,
        Some(later_sequence)
    );
    assert_eq!(
        dispatch::event_jobs(&mut db, later_sequence),
        vec![dispatch::id(20), dispatch::id(40), dispatch::id(60),]
    );
    let before = dispatch::snapshot(&mut db);
    assert!(
        dispatch::dispatch(&store, DeadlineDispatchStream::Events, 100)
            .event
            .is_none()
    );
    assert_eq!(dispatch::snapshot(&mut db), before);
}

#[test]
fn bootstrap_repeats_without_duplicate_jobs_and_finds_late_lower_uuids() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let profile = dl::profile(&db);
    let source = dl::source(&db);
    dispatch::legacy(&db, &profile, &source, 20);
    let legacy = dispatch::legacy(&db, &profile, &source, 40);
    dispatch::legacy(&db, &profile, &source, 60);
    let retired = dispatch::legacy(&db, &profile, &source, 50);
    dl::persist_legacy(&db, db.owner, dl::retire(&retired));
    let deadlines = dl::store(&db);
    let prepared =
        tracked::tracked_prepared(&db, deadlines.as_ref(), &dl::attention(&legacy), None);
    let undeclared = deadlines.commit(db.owner, prepared).unwrap();
    assert_eq!(
        undeclared.review_state(),
        DeadlineReviewState::LegacyUndeclared
    );
    assert!(matches!(
        undeclared.receipt.version,
        DeadlineReceiptVersion::Tracked(_)
    ));
    let accepted_command = dispatch::command(&db, &profile, &source, 30);
    let prepared = tracked::tracked_prepared(
        &db,
        deadlines.as_ref(),
        &accepted_command,
        Some(tracked::policies()),
    );
    let accepted = deadlines.commit(db.owner, prepared).unwrap();
    assert_eq!(accepted.review_state(), DeadlineReviewState::Accepted);
    let first_store = dispatch::open(&db);
    let first = dispatch::dispatch(&first_store, DeadlineDispatchStream::LegacyBootstrap, 1);
    assert!(first.event.is_none());
    assert_eq!((first.selected, first.inserted), (1, 1));
    assert!(!first.completed_scan);
    assert_eq!(
        first.progress.bootstrap_after_deadline_id,
        Some(dispatch::id(20))
    );
    assert_eq!(dispatch::bootstrap_jobs(&mut db), vec![dispatch::id(20)]);
    dispatch::legacy(&db, &profile, &source, 10);
    drop(first_store);
    for after in [Some(40), None] {
        let store = dispatch::open(&db);
        let batch = dispatch::dispatch(&store, DeadlineDispatchStream::LegacyBootstrap, 1);
        assert_eq!((batch.selected, batch.inserted), (1, 1));
        assert_eq!(batch.completed_scan, after.is_none());
        assert_eq!(
            batch.progress.bootstrap_after_deadline_id,
            after.map(dispatch::id)
        );
    }
    assert_eq!(
        dispatch::bootstrap_jobs(&mut db),
        vec![dispatch::id(20), dispatch::id(40), dispatch::id(60),]
    );
    let store = dispatch::open(&db);
    let next_sweep = dispatch::dispatch(&store, DeadlineDispatchStream::LegacyBootstrap, 1);
    assert_eq!((next_sweep.selected, next_sweep.inserted), (1, 1));
    assert!(next_sweep.completed_scan);
    assert!(next_sweep.progress.bootstrap_after_deadline_id.is_none());
    assert_eq!(
        dispatch::bootstrap_jobs(&mut db),
        vec![
            dispatch::id(10),
            dispatch::id(20),
            dispatch::id(40),
            dispatch::id(60),
        ]
    );
    let before = dispatch::snapshot(&mut db);
    drop(store);
    for _ in 0..2 {
        db.migrate();
        let reopened = dispatch::open(&db);
        let batch = dispatch::dispatch(&reopened, DeadlineDispatchStream::LegacyBootstrap, 100);
        assert_eq!((batch.selected, batch.inserted), (0, 0));
        assert!(batch.completed_scan);
        assert!(batch.progress.bootstrap_after_deadline_id.is_none());
        assert_eq!(dispatch::snapshot(&mut db), before);
        // Repeated empty sweeps do not append audit entries or replace job UUIDs.
        assert!(batch.progress.event.completed_sequence.is_none());
    }
}
