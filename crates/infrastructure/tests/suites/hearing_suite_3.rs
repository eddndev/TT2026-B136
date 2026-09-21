// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
#[path = "../hearing_database_support/mod.rs"]
mod hearing_database_support;
#[path = "../hearing_retention.rs"]
mod hearing_retention;
#[path = "../hearing_revalidation.rs"]
mod hearing_revalidation;
#[path = "../hearing_revalidation_support/mod.rs"]
mod hearing_revalidation_support;
#[path = "../hearing_schema.rs"]
mod hearing_schema;
#[path = "../hearing_sql_guards.rs"]
mod hearing_sql_guards;
#[path = "../hearing_sql_support/mod.rs"]
mod hearing_sql_support;
#[path = "../hearing_support_integrity.rs"]
mod hearing_support_integrity;
#[path = "../typed_participant_service_support/mod.rs"]
mod typed_participant_service_support;
