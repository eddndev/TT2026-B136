mod alert_backend_support;
mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod hearing_database_support;
mod procedural_fact_backend_support;

use alert_backend_support::*;
use application::{alerts::*, ApplicationError};
use domain::identity::Role;
use std::sync::Arc;
use time::Duration;

#[test]
fn scanner_audit_failure_rolls_back_plan_and_cursor_then_retry_succeeds() {
    let Some(mut db) = fixture() else { return };
    let at = db.at.replace_nanosecond(0).unwrap() + Duration::hours(24);
    hearing(&db, at);
    let store = store(&db, Arc::new(MutableClock::new(db.at)));
    store.preferences(db.owner).unwrap();
    let before = atomicity::snapshot(&mut db);
    atomicity::reject_scan_audit_after_writes(&mut db);
    assert!(matches!(store.run_next(), Err(ApplicationError::Port(_))));
    assert!(atomicity::scan_fault_reached(&mut db));
    assert_eq!(atomicity::snapshot(&mut db), before);
    atomicity::allow_scan_audit(&mut db);
    assert!(matches!(
        store.run_next().unwrap(),
        AlertSchedulerRun::Reconciled { .. }
    ));
    drive(&store);
    assert_eq!(store.list(db.owner, query(20)).unwrap().alerts.len(), 1);
}

#[test]
fn activation_audit_failure_rolls_back_inbox_outbox_and_plan_together() {
    let Some(mut db) = fixture() else { return };
    let at = db.at.replace_nanosecond(0).unwrap() + Duration::hours(24);
    hearing(&db, at);
    let store = store(&db, Arc::new(MutableClock::new(db.at)));
    assert!(matches!(
        store.run_next().unwrap(),
        AlertSchedulerRun::Reconciled { .. }
    ));
    let before = atomicity::snapshot(&mut db);
    atomicity::reject_activation_audit_after_writes(&mut db);
    assert!(matches!(store.run_next(), Err(ApplicationError::Port(_))));
    assert!(atomicity::fault_reached(&mut db));
    assert_eq!(atomicity::snapshot(&mut db), before);
    atomicity::allow_activation_audit(&mut db);
    drive(&store);
    assert_eq!(store.list(db.owner, query(20)).unwrap().alerts.len(), 1);
    let count: i64 = db
        .admin
        .query_one("SELECT count(*) FROM alert_email_outbox", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
}

#[test]
fn changed_follow_source_invalidates_planned_due_before_any_reevaluation_worker() {
    let Some(db) = fixture() else { return };
    let (source, deadline) = accepted_deadline(&db, db.owner);
    let due = deadline.calculation.result.due_at().unwrap();
    let store = store(&db, Arc::new(MutableClock::new(due - Duration::hours(12))));
    assert!(matches!(
        store.run_next().unwrap(),
        AlertSchedulerRun::Reconciled { .. }
    ));
    procedural_fact_backend_support::persist(
        &procedural_fact_backend_support::service(&db, db.owner, Role::Owner),
        db.case,
        procedural_fact_backend_support::correct(&source),
    );
    drive(&store);
    let page = store.list(db.owner, query(20)).unwrap();
    assert!(!page
        .alerts
        .iter()
        .any(|row| matches!(row.kind, AlertKind::Upcoming { .. })));
    assert!(page
        .alerts
        .iter()
        .any(|row| row.kind == AlertKind::ReviewRequired));
    assert!(deadline.calculation.result.due_at().is_some());
}
