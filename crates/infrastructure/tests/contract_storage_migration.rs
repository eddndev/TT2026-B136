//! Contract storage on a fresh database and on one migrated from the state
//! before it, with signing bindings, a Client invitation and an individual
//! signature already stored: docs/adr/0086-bilateral-contract-signing.md.
#![allow(dead_code)]
mod case_administration_support;
mod contract_storage_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::document_signing::IndividualSigningStore as _;
use contract_storage_support::{opens, rejection, Harness};
use uuid::Uuid;

/// Every object the contract migrations add, removed in dependency order.
const BEFORE_CONTRACTS: &str = "
    DROP TABLE contract_notifications,contract_completions,contract_party_signatures,
        contract_events,contract_signature_requests,contracts;
    DROP TRIGGER contract_version_guard ON documents;
    DROP FUNCTION contract_lock(),contract_preserve(),contract_event_register(),
        contract_request_register(),contract_signature_register(),contract_completion_register(),
        contract_completion_pair(),contract_notification_register(),contract_notification_guard(),
        contract_version_guard(),
        contract_request_bytes(uuid,uuid,uuid,uuid,bigint,bytea,uuid,uuid,uuid,bytea,bytea,bigint,bigint),
        contract_party_bytes(uuid,bytea,bigint,uuid,bigint,bigint,uuid,bytea,uuid,uuid,bytea,bytea,
            smallint,bytea,bytea);";

#[test]
fn a_fresh_database_holds_empty_contract_storage_that_passes_every_startup_check() {
    let Some(mut test) = Harness::without_contract() else {
        return;
    };
    let tables: i64 = test
        .db()
        .admin
        .query_one(
            "SELECT count(*) FROM pg_class WHERE relname IN ('contracts','contract_signature_requests',
            'contract_events','contract_party_signatures','contract_completions','contract_notifications')
            AND relnamespace=current_schema()::regnamespace",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(tables, 6);
    assert!(opens(&test));
}

#[test]
fn a_database_with_prior_signing_history_migrates_and_keeps_signed_versions_out() {
    let Some(mut test) = Harness::without_contract() else {
        return;
    };
    let command = test.individual.command(Uuid::new_v4());
    test.individual.store().commit(command).unwrap();
    test.db().admin.batch_execute(BEFORE_CONTRACTS).unwrap();
    assert!(!opens(&test));
    test.db().migrate();
    assert!(opens(&test));
    test.create_contract();
    test.db().migrate();
    assert!(opens(&test));
    let request = test.prepare(test.now() + 60, 7);
    assert_eq!(
        rejection(test.insert_request(&request)),
        (
            "23514".into(),
            "contract document version is not available".into()
        )
    );
}
