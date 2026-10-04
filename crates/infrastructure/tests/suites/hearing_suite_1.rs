// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
#[path = "../hearing_backend.rs"]
mod hearing_backend;
#[path = "../hearing_canonical_sql.rs"]
mod hearing_canonical_sql;
#[path = "../hearing_clock.rs"]
mod hearing_clock;
#[path = "../hearing_codec.rs"]
mod hearing_codec;
#[path = "../hearing_concurrency.rs"]
mod hearing_concurrency;
#[path = "../hearing_database_support/mod.rs"]
mod hearing_database_support;
#[path = "../hearing_import.rs"]
mod hearing_import;
#[path = "../hearing_inventory.rs"]
mod hearing_inventory;
#[path = "../hearing_queries.rs"]
mod hearing_queries;
#[path = "../hearing_receipt_sql.rs"]
mod hearing_receipt_sql;
#[path = "../hearing_restore.rs"]
mod hearing_restore;
#[path = "../hearing_result_backend.rs"]
mod hearing_result_backend;
#[path = "../hearing_result_canonical_sql.rs"]
mod hearing_result_canonical_sql;
#[path = "../hearing_result_database_support/mod.rs"]
mod hearing_result_database_support;
#[path = "../hearing_revalidation_support/mod.rs"]
mod hearing_revalidation_support;
#[path = "../hearing_sql_support/mod.rs"]
mod hearing_sql_support;
#[allow(dead_code)]
#[path = "../legacy_database_support/mod.rs"]
mod legacy_database_support;

#[path = "../deadline_profile_database_support/mod.rs"]
mod deadline_profile_database_support;
#[path = "../hearing_derived_deadline_schema.rs"]
mod hearing_derived_deadline_schema;
#[path = "../hearing_derived_deadline_storage_support.rs"]
mod hearing_derived_deadline_storage_support;

#[path = "../hearing_derived_deadline_catalog.rs"]
mod hearing_derived_deadline_catalog;

#[path = "../hearing_derived_deadline_guards.rs"]
mod hearing_derived_deadline_guards;
