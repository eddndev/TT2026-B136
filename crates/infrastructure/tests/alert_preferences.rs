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
use std::sync::Arc;

#[test]
fn personal_preferences_preserve_exact_replay_and_reject_stale_changes() {
    let Some(mut db) = fixture() else { return };
    let actor = db.user("litigator", true);
    let other = db.user("owner", false);
    let client = db.user("client", true);
    let clock = Arc::new(MutableClock::new(db.at));
    let store = store(&db, clock);
    let initial = store.preferences(actor).unwrap();
    assert_eq!(initial.user_id, actor);
    assert_eq!(initial.revision, 0);
    assert_eq!(initial.email_transport, AlertEmailTransport::Ready);
    assert_eq!(
        initial
            .values
            .hearing_upcoming
            .anticipations
            .hours()
            .iter()
            .map(|lead| lead.get())
            .collect::<Vec<_>>(),
        vec![48, 24]
    );
    assert_eq!(
        initial.values.hearing_upcoming.channels,
        AlertChannels {
            internal: true,
            email: true
        }
    );
    let mut command = AlertPreferenceCommand {
        operation_id: operation(),
        expected_revision: 0,
        values: initial.values.clone(),
    };
    command.values.review_required.email = false;
    let saved = store.save_preferences(actor, command.clone()).unwrap();
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.updated_at, Some(db.at));
    assert_eq!(saved.values, command.values);
    let mut next = command.clone();
    next.operation_id = operation();
    next.expected_revision = 1;
    next.values.overdue_unattended.internal = false;
    assert_eq!(store.save_preferences(actor, next).unwrap().revision, 2);
    assert_eq!(
        store.save_preferences(actor, command.clone()).unwrap(),
        saved
    );
    let mut reused = command.clone();
    reused.values.review_required.email = true;
    assert!(matches!(
        store.save_preferences(actor, reused),
        Err(ApplicationError::Alert(AlertError::OperationConflict))
    ));
    command.operation_id = operation();
    assert!(matches!(
        store.save_preferences(actor, command),
        Err(ApplicationError::Alert(AlertError::RevisionConflict))
    ));
    assert_eq!(store.preferences(other).unwrap().revision, 0);
    assert!(matches!(
        store.preferences(client),
        Err(ApplicationError::PermissionDenied)
    ));
    db.admin
        .execute(
            "UPDATE users SET active=false WHERE id=$1",
            &[&actor.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        store.preferences(actor),
        Err(ApplicationError::InvalidSession)
    ));
}
