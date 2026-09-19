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
use deadline_tracked_backend_support as tracked;
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
fn followed_source_is_changed_before_dispatch_without_mutating_immutable_history() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (source, base) = worker::accepted(&mut db, 10);
    let store = dl::store(&db);
    let initial = store.current(db.owner, db.case, base.id).unwrap();
    assert_eq!(initial.operational().freshness(), Current);
    assert_eq!(
        initial.operational().due_at(),
        base.calculation.result.due_at()
    );
    let original = tracked::revision_row(&mut db, &base);
    dispatch::advance(&db, &source);
    assert!(worker::jobs(&mut db).is_empty());
    let before = state(&mut db);
    let read = store.current(db.owner, db.case, base.id).unwrap();
    assert_eq!(read.detail(), &base);
    assert_eq!(read.operational().freshness(), Changed);
    assert_eq!(read.operational().checked_at(), Some(db.at));
    assert_eq!(
        read.operational().changed_dependencies(),
        &[TrackingDependency::Source]
    );
    assert_eq!(read.operational().due_at(), None);
    assert_eq!(
        store
            .get(db.owner, db.case, base.id, Some(base.revision), db.at)
            .unwrap(),
        base
    );
    assert_eq!(state(&mut db), before);
    assert_eq!(tracked::revision_row(&mut db, &base), original);
    let audits: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='deadline.current_read'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(audits, 2);
}

#[test]
fn fixed_active_source_can_advance_but_retirement_removes_its_operational_date() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (profile, source) = worker::inputs(&mut db);
    let mut policies = dl::FOLLOW_RESOLUTION;
    policies.source = dl::TrackingPolicy::Fixed;
    let workflow = dl::service(&db, db.owner, Role::Owner);
    let base = dl::persist(
        &workflow,
        db.case,
        dl::human(dl::command(&db, &profile, &source), Some(policies)),
    );
    let changed = dispatch::advance(&db, &source);
    let current = workflow.current("session", db.case, base.id).unwrap();
    assert_eq!(current.detail(), &base);
    assert_eq!(current.operational().freshness(), Current);
    assert_eq!(
        current.operational().due_at(),
        base.calculation.result.due_at()
    );
    facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        facts::withdraw(&changed),
    );
    let read = workflow.current("session", db.case, base.id).unwrap();
    assert_eq!(read.operational().freshness(), Changed);
    assert_eq!(
        read.operational().changed_dependencies(),
        &[TrackingDependency::Source]
    );
    assert_eq!(read.operational().due_at(), None);
    assert_eq!(read.detail(), &base);
}

#[test]
fn a_pending_old_job_does_not_override_a_human_acceptance_of_the_current_source() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (source, base) = worker::accepted(&mut db, 20);
    let changed = dispatch::advance(&db, &source);
    dispatch::dispatch(&dispatch::open(&db), DeadlineDispatchStream::Events, 20);
    assert_eq!(worker::jobs(&mut db).len(), 1);
    let workflow = dl::service(&db, db.owner, Role::Owner);
    let mut command = dl::correct(&base);
    dl::definition_mut(&mut command).input.selection.source = FactDeclaration::Known(
        TriggerSourceRef::Resolution(facts::resolution_ref(&changed)),
    );
    let accepted = dl::persist(
        &workflow,
        db.case,
        dl::human(command, Some(dl::FOLLOW_RESOLUTION)),
    );
    let pending_job_read = workflow.current("session", db.case, base.id).unwrap();
    assert_eq!(pending_job_read.detail(), &accepted);
    assert_eq!(pending_job_read.operational().freshness(), Current);
    assert!(pending_job_read.operational().due_at().is_some());
    assert!(matches!(
        worker::open(&db).run_next().unwrap(),
        DeadlineWorkerRun::Completed(_)
    ));
    let completed_job_read = workflow.current("session", db.case, base.id).unwrap();
    assert_eq!(completed_job_read, pending_job_read);
}

#[test]
fn legacy_and_retired_reads_never_publish_a_historical_due() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (profile, source) = worker::inputs(&mut db);
    let legacy = dl::persist_legacy(&db, db.owner, dl::command(&db, &profile, &source));
    let workflow = dl::service(&db, db.owner, Role::Owner);
    let retired = dl::persist(&workflow, db.case, dl::human(dl::retire(&legacy), None));
    let second = dl::persist_legacy(&db, db.owner, dl::command(&db, &profile, &source));
    for detail in [retired, second] {
        let read = workflow.current("session", db.case, detail.id).unwrap();
        assert_eq!(read.detail(), &detail);
        assert_eq!(read.operational().freshness(), NotChecked);
        assert_eq!(read.operational().checked_at(), None);
        assert_eq!(read.operational().due_at(), None);
    }
}
