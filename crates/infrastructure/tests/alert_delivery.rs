mod alert_backend_support;
mod alert_delivery_support;
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
use alert_delivery_support::*;
use application::{alerts::*, ApplicationError};
use domain::identity::Role;
use std::sync::Arc;
use time::Duration;

#[test]
fn expired_claim_is_fenced_and_known_completion_is_idempotent() {
    let Some(mut fixture) = prepared() else {
        return;
    };
    let first = fixture
        .store
        .claim_next()
        .unwrap()
        .expect("first durable claim");
    assert_eq!(first.alert_id, fixture.alert);
    assert_eq!(first.attempt, 1);
    assert_eq!(first.lease_until, first.claimed_at + Duration::seconds(120));
    assert!(fixture.store.claim_next().unwrap().is_none());
    fixture
        .clock
        .set(first.lease_until + Duration::nanoseconds(1));
    let second = fixture
        .store
        .claim_next()
        .unwrap()
        .expect("expired lease may be retried");
    assert_eq!(second.delivery_id, first.delivery_id);
    assert_eq!(second.attempt, 2);
    assert_ne!(second.claim_id, first.claim_id);
    assert_eq!(second.message, first.message);
    assert!(matches!(
        fixture
            .store
            .complete_attempt(completion(&first, accepted())),
        Err(ApplicationError::Alert(AlertError::OperationConflict))
    ));
    let result = completion(&second, accepted());
    fixture.store.complete_attempt(result.clone()).unwrap();
    let before = ledger(&mut fixture.db);
    fixture.store.complete_attempt(result).unwrap();
    assert_eq!(ledger(&mut fixture.db), before);
    assert!(matches!(
        fixture
            .store
            .complete_attempt(completion(&second, unknown())),
        Err(ApplicationError::Alert(AlertError::OperationConflict))
    ));
    assert!(fixture.store.claim_next().unwrap().is_none());
    assert!(matches!(
        fixture
            .store
            .get(fixture.db.owner, fixture.alert)
            .unwrap()
            .alert
            .email,
        AlertEmailStatus::Accepted { .. }
    ));
}

#[test]
fn uncertain_retry_preserves_all_provider_input_across_reopen_and_configuration_changes() {
    let Some(fixture) = prepared() else { return };
    let first = fixture.store.claim_next().unwrap().expect("first claim");
    fixture
        .store
        .complete_attempt(completion(&first, unknown()))
        .unwrap();
    assert!(fixture.store.claim_next().unwrap().is_none());
    fixture.clock.set(first.claimed_at + Duration::seconds(30));
    let reopened = reopened(&fixture);
    let retry = reopened
        .claim_next()
        .unwrap()
        .expect("retry at backoff boundary");
    assert_eq!(retry.message, first.message);
    assert_eq!(retry.first_attempt_at, first.first_attempt_at);
    assert_eq!(retry.attempt, 2);
    reopened
        .complete_attempt(completion(&retry, accepted()))
        .unwrap();
    let email = reopened
        .get(fixture.db.owner, fixture.alert)
        .unwrap()
        .alert
        .email;
    assert!(matches!(email, AlertEmailStatus::Accepted { .. }));
}

#[test]
fn unresolved_delivery_past_the_provider_window_never_receives_a_new_key() {
    let Some(mut fixture) = prepared() else {
        return;
    };
    let first = fixture.store.claim_next().unwrap().expect("first claim");
    fixture
        .store
        .complete_attempt(completion(&first, unknown()))
        .unwrap();
    fixture
        .clock
        .set(first.first_attempt_at + Duration::hours(24) + Duration::seconds(1));
    assert!(fixture.store.claim_next().unwrap().is_none());
    assert_eq!(
        fixture
            .store
            .get(fixture.db.owner, fixture.alert)
            .unwrap()
            .alert
            .email,
        AlertEmailStatus::Unknown
    );
    let count: i64 = fixture
        .db
        .admin
        .query_one("SELECT count(*) FROM alert_email_outbox", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
}

#[test]
fn claim_rechecks_preferences_membership_and_recipient_address_before_provider_handoff() {
    for change in 0..3 {
        let Some(mut fixture) = prepared() else {
            return;
        };
        let first = if change == 2 {
            let first = fixture.store.claim_next().unwrap().expect("first claim");
            fixture
                .store
                .complete_attempt(completion(&first, unknown()))
                .unwrap();
            Some(first)
        } else {
            None
        };
        match change {
            0 => {
                let mut values = AlertPreferenceValues::default();
                values.hearing_upcoming.channels.email = false;
                fixture
                    .store
                    .save_preferences(
                        fixture.db.owner,
                        AlertPreferenceCommand {
                            operation_id: operation(),
                            expected_revision: 0,
                            values,
                        },
                    )
                    .unwrap();
            }
            1 => {
                fixture
                    .db
                    .admin
                    .execute(
                        "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                        &[&fixture.db.case.as_uuid(), &fixture.db.owner.as_uuid()],
                    )
                    .unwrap();
            }
            _ => {
                fixture
                    .db
                    .admin
                    .execute(
                        "UPDATE users SET email='changed@example.test' WHERE id=$1",
                        &[&fixture.db.owner.as_uuid()],
                    )
                    .unwrap();
                fixture
                    .clock
                    .set(first.unwrap().claimed_at + Duration::seconds(30));
            }
        }
        assert!(
            fixture.store.claim_next().unwrap().is_none(),
            "change {change}"
        );
    }
}

#[test]
fn source_change_blocks_email_before_the_reevaluation_worker() {
    let Some(db) = fixture() else { return };
    let (source, deadline) = accepted_deadline(&db, db.owner);
    let clock = Arc::new(MutableClock::new(
        deadline.calculation.result.due_at().unwrap() - Duration::hours(12),
    ));
    let store = store(&db, clock);
    drive(&store);
    let alert = store.list(db.owner, query(20)).unwrap().alerts[0].id;
    procedural_fact_backend_support::persist(
        &procedural_fact_backend_support::service(&db, db.owner, Role::Owner),
        db.case,
        procedural_fact_backend_support::correct(&source),
    );
    assert!(store.claim_next().unwrap().is_none());
    assert_eq!(
        store.get(db.owner, alert).unwrap().alert.email,
        AlertEmailStatus::Cancelled
    );
}

#[test]
fn audit_failure_rolls_back_claim_and_attempt_history_before_retry() {
    let Some(mut fixture) = prepared() else {
        return;
    };
    let before = ledger(&mut fixture.db);
    fixture.db.admin.batch_execute("CREATE FUNCTION reject_delivery_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected delivery audit failure'; END $$;
        CREATE TRIGGER reject_delivery_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_delivery_audit()").unwrap();
    assert!(matches!(
        fixture.store.claim_next(),
        Err(ApplicationError::Port(_))
    ));
    assert_eq!(ledger(&mut fixture.db), before);
    fixture.db.admin.batch_execute("DROP TRIGGER reject_delivery_audit ON audit_events; DROP FUNCTION reject_delivery_audit()").unwrap();
    assert_eq!(
        fixture
            .store
            .claim_next()
            .unwrap()
            .expect("claim after rollback")
            .attempt,
        1
    );
}
