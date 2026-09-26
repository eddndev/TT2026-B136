// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
#[path = "../declaration_fixture/mod.rs"]
mod declaration_fixture;
#[path = "../typed_participant_permissions.rs"]
mod typed_participant_permissions;
#[path = "../typed_participant_restore.rs"]
mod typed_participant_restore;
#[path = "../typed_participant_schema.rs"]
mod typed_participant_schema;
#[path = "../typed_participant_service_support/mod.rs"]
mod typed_participant_service_support;
#[path = "../typed_participant_signed.rs"]
mod typed_participant_signed;
#[path = "../typed_participant_transactions.rs"]
mod typed_participant_transactions;
#[path = "../typed_participant_trust_races.rs"]
mod typed_participant_trust_races;
#[allow(dead_code)]
#[path = "../typed_participant_trust_races_support/mod.rs"]
mod typed_participant_trust_races_support;
