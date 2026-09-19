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
use application::{alerts::*, cases::CaseRepository, ApplicationError};
use domain::clock::Clock;
use std::sync::Arc;
use time::Duration;

#[test]
fn revoked_case_and_other_recipients_are_filtered_before_the_inbox_limit() {
    let Some(mut db) = fixture() else { return };
    let actor = db.user("litigator", true);
    let global_owner = db.user("owner", false);
    let first_case = db.case;
    let at = db.at.replace_nanosecond(0).unwrap() + Duration::hours(24);
    hearing(&db, at);
    let clock = Arc::new(MutableClock::new(db.at));
    let store = store(&db, clock.clone());
    drive(&store);
    let first = store.list(actor, query(20)).unwrap();
    assert_eq!(first.alerts.len(), 1);
    let first_id = first.alerts[0].id;
    assert!(store
        .list(global_owner, query(20))
        .unwrap()
        .alerts
        .is_empty());
    assert!(matches!(
        store.get(global_owner, first_id),
        Err(ApplicationError::Alert(AlertError::NotFound))
    ));
    hearing_database_support::complete(&mut db);
    db.store()
        .add_member(db.case, actor, db.owner, db.at)
        .unwrap();
    hearing(&db, at);
    clock.set(db.at + Duration::seconds(1));
    drive(&store);
    let newest = store.list(actor, query(1)).unwrap();
    assert_eq!(newest.alerts[0].subject.case_id(), db.case);
    assert!(newest.has_more);
    let revoked_id = newest.alerts[0].id;
    db.store()
        .remove_member(db.case, actor, db.owner, clock.now())
        .unwrap();
    let allowed = store.list(actor, query(1)).unwrap();
    assert_eq!(allowed.alerts.len(), 1);
    assert_eq!(allowed.alerts[0].id, first_id);
    assert_eq!(allowed.alerts[0].subject.case_id(), first_case);
    assert!(!allowed.has_more);
    assert!(allowed.next_cursor.is_none());
    assert!(matches!(
        store.get(actor, revoked_id),
        Err(ApplicationError::Alert(AlertError::NotFound))
    ));
    assert!(matches!(
        store.mark_read(
            actor,
            AlertReadCommand {
                operation_id: operation(),
                alert_id: revoked_id,
            }
        ),
        Err(ApplicationError::Alert(AlertError::NotFound))
    ));
}

#[test]
fn catchup_and_restart_keep_one_occurrence_and_read_does_not_record_attention() {
    let Some(mut db) = fixture() else { return };
    let owner = db.owner;
    let (_, deadline) = accepted_deadline(&db, owner);
    let due = deadline.calculation.result.due_at().unwrap();
    let now = due - Duration::hours(12) + Duration::nanoseconds(123);
    let clock = Arc::new(MutableClock::new(now));
    let first_store = store(&db, clock.clone());
    let before = resources(&mut db);
    drive(&first_store);
    let page = first_store.list(owner, query(20)).unwrap();
    assert_eq!(page.alerts.len(), 1);
    assert_eq!(page.checked_at, now);
    let alert = &page.alerts[0];
    assert!(
        matches!(alert.kind, AlertKind::Upcoming { lead_hours, activity_at }
        if lead_hours.get() == 24 && activity_at == due)
    );
    assert_eq!(alert.email, AlertEmailStatus::Pending);
    let command = AlertReadCommand {
        operation_id: operation(),
        alert_id: alert.id,
    };
    let read = first_store.mark_read(owner, command).unwrap();
    assert_eq!(read.alert.read_at, Some(now));
    clock.set(now + Duration::seconds(1));
    assert_eq!(
        first_store.mark_read(owner, command).unwrap().alert.read_at,
        Some(now)
    );
    drop(first_store);
    let reopened = store(&db, clock);
    drive(&reopened);
    let after = reopened.list(owner, query(20)).unwrap();
    assert_eq!(after.alerts.len(), 1);
    assert_eq!(after.alerts[0].id, alert.id);
    assert_eq!(after.alerts[0].occurrence_id, alert.occurrence_id);
    assert_eq!(resources(&mut db), before);
}

#[test]
fn corrupted_captured_hearing_context_fails_closed_on_get_and_list() {
    let Some(mut db) = fixture() else { return };
    let at = db.at.replace_nanosecond(0).unwrap() + Duration::hours(24);
    let hearing = hearing(&db, at);
    let store = store(&db, Arc::new(MutableClock::new(db.at)));
    drive(&store);
    let rows = store.list(db.owner, query(20)).unwrap();
    assert_eq!(rows.alerts.len(), 1);
    let id = rows.alerts[0].id;
    assert_eq!(store.get(db.owner, id).unwrap().alert.id, id);
    corrupt_hearing_context(&mut db, hearing.snapshot.id);
    assert!(store.get(db.owner, id).is_err());
    assert!(store.list(db.owner, query(20)).is_err());
}
