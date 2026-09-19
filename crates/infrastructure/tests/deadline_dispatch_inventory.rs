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
mod deadline_schema_support;
mod procedural_fact_backend_support;

use application::{deadline_dispatch::DeadlineDispatchStream, procedural_facts::FactDeclaration};
use deadline_backend_support as dl;
use deadline_dispatch_guard_support as guards;
use deadline_dispatch_support as dispatch;
use deadline_schema_support::open;
use domain::deadline_triggers::TriggerSourceRef;
use procedural_fact_backend_support as facts;

#[test]
fn migration_does_not_reseed_a_missing_singleton_in_an_existing_cursor_table() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    open(&db).unwrap();
    let deadlines_before = dl::snapshot(&mut db);
    let jobs_before = guards::jobs(&mut db);
    // Model an administrative corruption, then reinstall every real guard.
    db.admin
        .batch_execute(
            "DROP TRIGGER deadline_dispatch_protected ON deadline_dispatch_cursor;
        DELETE FROM deadline_dispatch_cursor",
        )
        .unwrap();
    db.migrate();
    let row = db
        .admin
        .query_one(
            "SELECT (SELECT count(*) FROM deadline_dispatch_cursor),
            EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='deadline_dispatch_cursor'::regclass
                AND tgname='deadline_dispatch_protected' AND tgenabled IN ('O','A'))",
            &[],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), 0);
    assert!(
        row.get::<_, bool>(1),
        "migration must restore the guard without fabricating progress"
    );
    assert!(
        open(&db).is_err(),
        "startup accepted a lost dispatch checkpoint"
    );
    assert_eq!(guards::jobs(&mut db), jobs_before);
    assert_eq!(dl::snapshot(&mut db), deadlines_before);
    db.migrate();
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM deadline_dispatch_cursor", &[])
            .unwrap()
            .get::<_, i64>(0),
        0
    );
    assert!(open(&db).is_err());
}

#[test]
fn historic_jobs_and_cursor_anchors_survive_dependency_changes_and_retirement() {
    let Some((mut db, changed, deadlines)) = guards::setup(&[20, 40, 60]) else {
        return;
    };
    let sequence = dispatch::event_sequence(&mut db, &changed);
    let store = dispatch::open(&db);
    let first = dispatch::dispatch(&store, DeadlineDispatchStream::Events, 1);
    assert_eq!(
        first.progress.event.after_deadline_id,
        Some(dispatch::id(20))
    );
    drop(store);
    let jobs_before = guards::jobs(&mut db);
    let other = dl::source(&db);
    let mut command = dl::correct(&deadlines[0]);
    dl::definition_mut(&mut command).input.selection.source =
        FactDeclaration::Known(TriggerSourceRef::Resolution(facts::resolution_ref(&other)));
    let corrected = dl::persist_legacy(&db, db.owner, command);
    let after_correction = guards::snapshot(&mut db);
    open(&db).unwrap();
    assert_eq!(guards::snapshot(&mut db), after_correction);
    assert_eq!(guards::jobs(&mut db), jobs_before);
    dl::persist_legacy(&db, db.owner, dl::retire(&corrected));
    let before_reopen = guards::snapshot(&mut db);
    db.migrate();
    open(&db).unwrap();
    let reopened = dispatch::open(&db);
    assert_eq!(guards::snapshot(&mut db), before_reopen);
    assert_eq!(guards::jobs(&mut db), jobs_before);
    let last = dispatch::dispatch(&reopened, DeadlineDispatchStream::Events, 100);
    assert_eq!(last.event.unwrap().sequence, sequence);
    assert_eq!((last.selected, last.inserted), (2, 2));
    assert!(last.completed_scan);
    assert_eq!(
        dispatch::event_jobs(&mut db, sequence),
        vec![dispatch::id(20), dispatch::id(40), dispatch::id(60)]
    );
}
