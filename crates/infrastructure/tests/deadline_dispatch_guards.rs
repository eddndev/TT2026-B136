mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_dispatch_guard_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
mod deadline_profile_database_support;
mod procedural_fact_backend_support;

use application::deadline_dispatch::DeadlineDispatchStream;
use deadline_backend_support as dl;
use deadline_dispatch_guard_support as guards;
use deadline_dispatch_support as dispatch;
use postgres::error::SqlState;

#[test]
fn both_streams_reject_missing_middle_and_final_jobs_before_advancing() {
    for bootstrap in [false, true] {
        let Some((mut db, changed, _)) = guards::setup(&[20, 40, 60, 80]) else {
            return;
        };
        let sequence = i64::try_from(dispatch::event_sequence(&mut db, &changed)).unwrap();
        let event = (!bootstrap).then_some(sequence);
        let before = guards::snapshot(&mut db);
        let mut runtime = db.runtime();
        let mut tx = runtime.transaction().unwrap();
        guards::insert_job(&mut tx, &db, 20, event);
        guards::insert_job(&mut tx, &db, 60, event);
        let partial = if bootstrap {
            format!(
                "UPDATE deadline_dispatch_cursor SET bootstrap_after_deadline_id='{}'",
                dispatch::id(60)
            )
        } else {
            format!("UPDATE deadline_dispatch_cursor SET active_event_sequence={sequence},after_deadline_id='{}'", dispatch::id(60))
        };
        guards::rejects_in_savepoint(&mut tx, &partial);
        // The same transition becomes legal only after its missing middle job exists.
        guards::insert_job(&mut tx, &db, 40, event);
        assert_eq!(tx.execute(&partial, &[]).unwrap(), 1);
        let complete = if bootstrap {
            "UPDATE deadline_dispatch_cursor SET bootstrap_after_deadline_id=NULL".into()
        } else {
            format!("UPDATE deadline_dispatch_cursor SET completed_event_sequence={sequence},active_event_sequence=NULL,after_deadline_id=NULL")
        };
        guards::rejects_in_savepoint(&mut tx, &complete);
        guards::insert_job(&mut tx, &db, 80, event);
        assert_eq!(tx.execute(&complete, &[]).unwrap(), 1);
        // Direct SQL fixtures never commit jobs without the adapter's audit entry.
        tx.rollback().unwrap();
        assert_eq!(guards::snapshot(&mut db), before);
    }
}

#[test]
fn cursor_rejects_skipped_events_regressions_unassigned_positions_and_mixed_streams() {
    let Some((mut db, changed, _)) = guards::setup(&[20, 40, 60]) else {
        return;
    };
    let sequence = dispatch::event_sequence(&mut db, &changed);
    let later_source = dl::source(&db);
    let later = dispatch::event_sequence(&mut db, &later_source);
    assert!(later > sequence);
    let store = dispatch::open(&db);
    let page = dispatch::dispatch(&store, DeadlineDispatchStream::Events, 1);
    assert_eq!(page.progress.event.active_sequence, Some(sequence));
    assert_eq!(
        page.progress.event.after_deadline_id,
        Some(dispatch::id(20))
    );
    drop(store);
    let before = guards::snapshot(&mut db);
    let mut runtime = db.runtime();
    for sql in [
        format!("UPDATE deadline_dispatch_cursor SET completed_event_sequence={later},active_event_sequence=NULL,after_deadline_id=NULL"),
        "UPDATE deadline_dispatch_cursor SET completed_event_sequence=NULL,active_event_sequence=NULL,after_deadline_id=NULL".into(),
        format!("UPDATE deadline_dispatch_cursor SET after_deadline_id='{}'", dispatch::id(0)),
        format!("UPDATE deadline_dispatch_cursor SET after_deadline_id='{}'", dispatch::id(30)),
        format!("UPDATE deadline_dispatch_cursor SET after_deadline_id='{}',bootstrap_after_deadline_id='{}'", dispatch::id(40), dispatch::id(20)),
        format!("UPDATE deadline_dispatch_cursor SET completed_event_sequence={sequence},active_event_sequence=NULL,after_deadline_id=NULL"),
    ] {
        let error = runtime.batch_execute(&sql).unwrap_err();
        assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION), "{sql}: {error:?}");
        assert_eq!(guards::snapshot(&mut db), before, "{sql}");
    }
    let reopened = dispatch::open(&db);
    let last = dispatch::dispatch(&reopened, DeadlineDispatchStream::Events, 100);
    assert!(last.completed_scan);
    assert_eq!(last.event.unwrap().sequence, sequence);
    assert_eq!(
        dispatch::event_jobs(&mut db, sequence),
        vec![dispatch::id(20), dispatch::id(40), dispatch::id(60)]
    );
}

#[test]
fn job_history_and_cursor_identity_remain_protected_for_administrative_connections() {
    let Some((mut db, _, _)) = guards::setup(&[20, 40]) else {
        return;
    };
    let store = dispatch::open(&db);
    dispatch::dispatch(&store, DeadlineDispatchStream::Events, 1);
    drop(store);
    assert!(!guards::jobs(&mut db).as_array().unwrap().is_empty());
    let before = guards::snapshot(&mut db);
    for sql in [
        "UPDATE deadline_reevaluation_jobs SET operation_id=operation_id",
        "UPDATE deadline_reevaluation_jobs SET case_id=case_id",
        "DELETE FROM deadline_reevaluation_jobs",
        "TRUNCATE deadline_reevaluation_jobs",
        "INSERT INTO deadline_dispatch_cursor(singleton) VALUES(TRUE)",
        "UPDATE deadline_dispatch_cursor SET singleton=FALSE",
        "DELETE FROM deadline_dispatch_cursor",
        "TRUNCATE deadline_dispatch_cursor",
    ] {
        let error = db.admin.batch_execute(sql).unwrap_err();
        assert_eq!(
            error.code(),
            Some(&SqlState::CHECK_VIOLATION),
            "{sql}: {error:?}"
        );
        assert_eq!(guards::snapshot(&mut db), before, "{sql}");
    }
}
