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
use application::alerts::*;
use infrastructure::{PostgresAlertStore, RingSha256Hasher};
use std::sync::Arc;

fn open(db: &Fixture) -> Result<PostgresAlertStore, application::ApplicationError> {
    PostgresAlertStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(MutableClock::new(db.at)),
        None,
    )
}

#[test]
fn reopening_verified_history_preserves_every_persisted_row() {
    let Some(mut fixture) = prepared() else {
        return;
    };
    fixture
        .store
        .save_preferences(
            fixture.db.owner,
            AlertPreferenceCommand {
                operation_id: operation(),
                expected_revision: 0,
                values: AlertPreferenceValues::default(),
            },
        )
        .unwrap();
    fixture
        .store
        .mark_read(
            fixture.db.owner,
            AlertReadCommand {
                operation_id: operation(),
                alert_id: fixture.alert,
            },
        )
        .unwrap();
    let claimed = fixture.store.claim_next().unwrap().expect("claim");
    fixture
        .store
        .complete_attempt(completion(&claimed, accepted()))
        .unwrap();
    let before = atomicity::snapshot(&mut fixture.db);
    let ledger_before = ledger(&mut fixture.db);
    drop(open(&fixture.db).unwrap());
    assert_eq!(atomicity::snapshot(&mut fixture.db), before);
    assert_eq!(ledger(&mut fixture.db), ledger_before);
}

#[test]
fn startup_rejects_preference_gaps_cursor_orphans_and_schedule_projection_changes() {
    for damage in 0..3 {
        let Some(mut fixture) = prepared() else {
            return;
        };
        for revision in 0..2 {
            fixture
                .store
                .save_preferences(
                    fixture.db.owner,
                    AlertPreferenceCommand {
                        operation_id: operation(),
                        expected_revision: revision,
                        values: AlertPreferenceValues::default(),
                    },
                )
                .unwrap();
        }
        let sql = match damage {
            0 => "DELETE FROM alert_preferences WHERE revision=1",
            1 => "UPDATE alert_scan_cursor SET active_kind=0,active_id='ffffffff-ffff-ffff-ffff-ffffffffffff',after_recipient=NULL",
            _ => "UPDATE alert_schedule SET trigger_nanos=(trigger_nanos+1)%1000000000",
        };
        fixture
            .db
            .admin
            .batch_execute("SET session_replication_role='replica'")
            .unwrap();
        fixture.db.admin.batch_execute(sql).unwrap();
        fixture
            .db
            .admin
            .batch_execute("SET session_replication_role='origin'")
            .unwrap();
        assert!(open(&fixture.db).is_err(), "damage {damage}");
    }
}

#[test]
fn startup_rejects_corrupt_notification_read_binding_and_delivery_history() {
    for damage in 0..3 {
        let Some(mut fixture) = prepared() else {
            return;
        };
        fixture
            .store
            .mark_read(
                fixture.db.owner,
                AlertReadCommand {
                    operation_id: operation(),
                    alert_id: fixture.alert,
                },
            )
            .unwrap();
        fixture.store.claim_next().unwrap().expect("claim");
        let sql = match damage {
            0 => "UPDATE alert_notifications SET payload=convert_to('{}','UTF8'),payload_digest=pg_catalog.sha256(convert_to('{}','UTF8'))",
            1 => "UPDATE alert_read_receipts SET read_nanos=(read_nanos+1)%1000000000",
            _ => "DELETE FROM alert_email_attempts WHERE sequence=0",
        };
        fixture
            .db
            .admin
            .batch_execute("SET session_replication_role='replica'")
            .unwrap();
        fixture.db.admin.batch_execute(sql).unwrap();
        fixture
            .db
            .admin
            .batch_execute("SET session_replication_role='origin'")
            .unwrap();
        assert!(open(&fixture.db).is_err(), "damage {damage}");
    }
}
