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
use application::{alerts::*, deadlines::*};
use deadline_backend_support as deadlines;
use domain::identity::Role;
use std::sync::Arc;
use time::Duration;

#[test]
fn attention_recorded_then_reopened_between_scans_starts_a_new_overdue_occurrence() {
    let Some(mut db) = fixture() else { return };
    let (_, deadline) = accepted_deadline(&db, db.owner);
    let due = deadline.calculation.result.due_at().unwrap();
    let now = due + Duration::hours(1);
    let clock = Arc::new(MutableClock::new(now));
    let alerts = store(&db, clock.clone());
    drive(&alerts);
    let first = alerts.list(db.owner, query(20)).unwrap();
    assert_eq!(first.alerts.len(), 1);
    let first = first.alerts[0].clone();
    assert_eq!(first.kind, AlertKind::OverdueUnattended { due_at: due });
    assert_eq!(first.state, AlertState::Active);

    db.at = now + Duration::seconds(1);
    clock.set(db.at);
    let attended = deadlines::persist(
        &deadlines::service(&db, db.owner, Role::Owner),
        db.case,
        deadlines::human(deadlines::attention(&deadline), None),
    );
    db.at += Duration::seconds(1);
    clock.set(db.at);
    let mut reopen = deadlines::attention(&attended);
    let DeadlineChange::SetAttention { attention, .. } = &mut reopen.change else {
        unreachable!()
    };
    *attention = DeadlineAttention::Pending;
    let reopened_deadline = deadlines::persist(
        &deadlines::service(&db, db.owner, Role::Owner),
        db.case,
        deadlines::human(reopen, None),
    );
    // Neither the inbox nor the scheduler observes the intermediate attention state.
    let resolved = alerts.get(db.owner, first.id).unwrap().alert;
    assert!(matches!(
        resolved.state,
        AlertState::Resolved {
            reason: AlertResolutionReason::AttentionRecorded,
            ..
        }
    ));
    drop(alerts);
    let restarted = store(&db, clock.clone());
    drive(&restarted);
    let page = restarted.list(db.owner, query(20)).unwrap();
    assert_eq!(page.alerts.len(), 2);
    let current = page
        .alerts
        .iter()
        .find(|alert| alert.id != first.id)
        .unwrap();
    assert_eq!(current.kind, AlertKind::OverdueUnattended { due_at: due });
    assert_eq!(current.state, AlertState::Active);
    assert_eq!(current.origin.revision, reopened_deadline.revision.get());
    assert_ne!(current.occurrence_id, first.occurrence_id);
    assert_eq!(restarted.get(db.owner, first.id).unwrap().alert, resolved);
    clock.set(db.at + Duration::seconds(2));
    drive(&restarted);
    assert_eq!(restarted.list(db.owner, query(20)).unwrap().alerts.len(), 2);
}

#[test]
fn reenabled_upcoming_channels_activate_a_never_emitted_occurrence_once() {
    let Some(db) = fixture() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::hours(72);
    hearing(&db, due);
    let clock = Arc::new(MutableClock::new(db.at));
    let alerts = store(&db, clock.clone());
    drive(&alerts);
    assert!(alerts.list(db.owner, query(20)).unwrap().alerts.is_empty());
    let initial = alerts.preferences(db.owner).unwrap();
    let mut disabled = initial.values.clone();
    disabled.hearing_upcoming.channels = AlertChannels {
        internal: false,
        email: false,
    };
    let saved = alerts
        .save_preferences(
            db.owner,
            AlertPreferenceCommand {
                operation_id: operation(),
                expected_revision: initial.revision,
                values: disabled,
            },
        )
        .unwrap();
    clock.set(due - Duration::hours(36));
    drive(&alerts);
    assert!(alerts.list(db.owner, query(20)).unwrap().alerts.is_empty());
    assert!(matches!(
        alerts.run_next().unwrap(),
        AlertSchedulerRun::Idle
    ));
    alerts
        .save_preferences(
            db.owner,
            AlertPreferenceCommand {
                operation_id: operation(),
                expected_revision: saved.revision,
                values: initial.values,
            },
        )
        .unwrap();
    drive(&alerts);
    let page = alerts.list(db.owner, query(20)).unwrap();
    assert_eq!(page.alerts.len(), 1);
    let first = page.alerts[0].clone();
    assert!(
        matches!(first.kind, AlertKind::Upcoming { lead_hours, activity_at }
        if lead_hours.get() == 48 && activity_at == due)
    );
    assert_eq!(first.state, AlertState::Active);
    assert_eq!(first.email, AlertEmailStatus::Pending);
    drop(alerts);
    clock.set(due - Duration::hours(35));
    let restarted = store(&db, clock);
    drive(&restarted);
    let after = restarted.list(db.owner, query(20)).unwrap();
    assert_eq!(after.alerts.len(), 1);
    assert_eq!(after.alerts[0].id, first.id);
    assert_eq!(after.alerts[0].occurrence_id, first.occurrence_id);
}
