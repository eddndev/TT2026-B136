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
mod procedural_fact_backend_support;

use application::{
    deadline_currentness::DeadlineFreshness::{Changed, Current, NotChecked},
    deadline_dispatch::DeadlineDispatchStream,
    deadline_tracking::TrackingDependency,
    deadline_worker::{DeadlineWorkerRun, DeadlineWorkerStore},
    deadlines::*,
};
use deadline_backend_support as dl;
use deadline_dispatch_support as dispatch;
use deadline_worker_backend_support as worker;
use domain::{
    deadline_triggers::TriggerSourceRef, identity::Role, procedural_facts::FactDeclaration,
};
use procedural_fact_backend_support as facts;

fn state(db: &mut dl::Fixture) -> serde_json::Value {
    let mut value = worker::snapshot(db);
    value.as_object_mut().unwrap().remove("audit");
    value
}

#[test]
fn mixed_history_list_checks_heads_before_dispatch_and_retains_each_historical_calculation() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (profile, source) = worker::inputs(&mut db);
    let workflow = dl::service(&db, db.owner, Role::Owner);
    let legacy = dispatch::legacy(&db, &profile, &source, 10);
    let mut fixed_policies = dl::FOLLOW_RESOLUTION;
    fixed_policies.source = dl::TrackingPolicy::Fixed;
    let fixed = dl::persist(
        &workflow,
        db.case,
        dl::human(
            dispatch::command(&db, &profile, &source, 20),
            Some(fixed_policies),
        ),
    );
    let followed = dl::persist(
        &workflow,
        db.case,
        dl::human(
            dispatch::command(&db, &profile, &source, 30),
            Some(dl::FOLLOW_RESOLUTION),
        ),
    );
    dispatch::advance(&db, &source);
    assert!(worker::jobs(&mut db).is_empty());
    let before = state(&mut db);
    let page = workflow
        .list("session", db.case, dl::query(20, None))
        .unwrap();
    assert_eq!(page.deadlines.len(), 3);
    for ((row, base), freshness) in page
        .deadlines
        .iter()
        .zip([&legacy, &fixed, &followed])
        .zip([NotChecked, Current, Changed])
    {
        assert_eq!(row.id, base.id);
        assert_eq!(row.revision, base.revision);
        assert_eq!(row.calculation_due_at, base.calculation.result.due_at());
        assert!(!row.calculation_blocked);
        assert_eq!(
            row.receipt_kind,
            if base.id == legacy.id {
                DeadlineReceiptKind::Legacy
            } else {
                DeadlineReceiptKind::Tracked
            }
        );
        assert_eq!(row.operational.freshness(), freshness);
        assert_eq!(
            row.operational.checked_at(),
            if freshness == NotChecked {
                None
            } else {
                Some(db.at)
            }
        );
        assert!(row.operational.matches_overview(row));
    }
    assert_eq!(page.deadlines[0].operational.due_at(), None);
    assert_eq!(
        page.deadlines[1].operational.due_at(),
        fixed.calculation.result.due_at()
    );
    assert_eq!(page.deadlines[2].operational.due_at(), None);
    assert_eq!(
        page.deadlines[2].operational.changed_dependencies(),
        &[TrackingDependency::Source]
    );
    assert_eq!(state(&mut db), before);
    let audits: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='deadline.list_read'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(audits, 1);
}

#[test]
fn list_ignores_old_pending_work_and_rechecks_heads_after_a_no_change_completion() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (source, initial) = worker::accepted(&mut db, 10);
    let second = dispatch::advance(&db, &source);
    dispatch::dispatch(&dispatch::open(&db), DeadlineDispatchStream::Events, 20);
    assert_eq!(worker::jobs(&mut db).len(), 1);
    let workflow = dl::service(&db, db.owner, Role::Owner);
    let mut command = dl::correct(&initial);
    dl::definition_mut(&mut command).input.selection.source =
        FactDeclaration::Known(TriggerSourceRef::Resolution(facts::resolution_ref(&second)));
    let accepted = dl::persist(
        &workflow,
        db.case,
        dl::human(command, Some(dl::FOLLOW_RESOLUTION)),
    );
    let page = workflow
        .list("session", db.case, dl::query(20, None))
        .unwrap();
    assert_eq!(page.deadlines[0].revision, accepted.revision);
    assert_eq!(page.deadlines[0].operational.freshness(), Current);
    assert_eq!(
        page.deadlines[0].operational.due_at(),
        accepted.calculation.result.due_at()
    );
    dispatch::advance(&db, &second);
    assert!(matches!(
        worker::open(&db).run_next().unwrap(),
        DeadlineWorkerRun::Completed(_)
    ));
    let before = state(&mut db);
    let page = workflow
        .list("session", db.case, dl::query(20, None))
        .unwrap();
    assert_eq!(page.deadlines[0].revision, accepted.revision);
    assert_eq!(page.deadlines[0].operational.freshness(), Changed);
    assert_eq!(page.deadlines[0].operational.due_at(), None);
    assert_eq!(state(&mut db), before);
}

#[test]
fn list_paginates_before_loading_current_dependencies_and_rejects_corruption_on_the_page() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (profile, first_source) = worker::inputs(&mut db);
    let second_source = dl::source(&db);
    let workflow = dl::service(&db, db.owner, Role::Owner);
    for (id, source) in [(10, &first_source), (20, &second_source)] {
        dl::persist(
            &workflow,
            db.case,
            dl::human(
                dispatch::command(&db, &profile, source, id),
                Some(dl::FOLLOW_RESOLUTION),
            ),
        );
    }
    let changed = dispatch::advance(&db, &second_source);
    let store = dl::store(&db);
    let reference = facts::resolution_ref(&changed);
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute(
        "ALTER TABLE case_procedural_fact_revisions DISABLE TRIGGER procedural_fact_immutable",
    )
    .unwrap();
    assert_eq!(tx.execute(
        "UPDATE case_procedural_fact_revisions SET recorded_administration_reference='Substituted current evidence'
         WHERE family='resolution' AND id=$1 AND revision=$2",
        &[&reference.id.as_uuid(), &i64::from(reference.revision.get())],
    ).unwrap(), 1);
    tx.batch_execute(
        "ALTER TABLE case_procedural_fact_revisions ENABLE TRIGGER procedural_fact_immutable",
    )
    .unwrap();
    tx.commit().unwrap();
    let before = state(&mut db);
    let first = store
        .list(db.owner, db.case, dl::query(1, None), db.at)
        .unwrap();
    assert_eq!(first.deadlines.len(), 1);
    assert_eq!(first.deadlines[0].id, dispatch::id(10));
    assert_eq!(first.deadlines[0].operational.freshness(), Current);
    assert!(first.has_more);
    assert_eq!(first.next_after_id, Some(dispatch::id(10)));
    let empty = store
        .list(
            db.owner,
            db.case,
            dl::query(1, Some(dispatch::id(20))),
            db.at,
        )
        .unwrap();
    assert!(empty.deadlines.is_empty());
    let retired = DeadlineQuery::new(1, None, DeadlineStatusFilter::Retired).unwrap();
    assert!(store
        .list(db.owner, db.case, retired, db.at)
        .unwrap()
        .deadlines
        .is_empty());
    assert!(store
        .list(
            db.owner,
            db.case,
            dl::query(1, Some(dispatch::id(10))),
            db.at
        )
        .is_err());
    assert_eq!(state(&mut db), before);
}

#[test]
fn a_list_audit_failure_discloses_no_projection_and_leaves_no_partial_changes() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    worker::accepted(&mut db, 10);
    let store = dl::store(&db);
    db.admin.batch_execute("CREATE FUNCTION reject_current_list_audit() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.action='deadline.list_read' THEN RAISE EXCEPTION 'injected audit failure'; END IF; RETURN NEW; END; $$;
        CREATE TRIGGER reject_current_list_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_current_list_audit()").unwrap();
    let before = worker::snapshot(&mut db);
    assert!(store
        .list(db.owner, db.case, dl::query(20, None), db.at)
        .is_err());
    assert_eq!(worker::snapshot(&mut db), before);
}
