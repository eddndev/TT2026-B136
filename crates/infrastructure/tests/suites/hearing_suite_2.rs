// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[allow(dead_code)]
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
#[path = "../hearing_database_support/mod.rs"]
mod hearing_database_support;
#[path = "../hearing_result_catalog.rs"]
mod hearing_result_catalog;
#[path = "../hearing_result_clock.rs"]
mod hearing_result_clock;
#[path = "../hearing_result_codec.rs"]
mod hearing_result_codec;
#[path = "../hearing_result_concurrency.rs"]
mod hearing_result_concurrency;
#[allow(dead_code)]
#[path = "../hearing_result_database_support/mod.rs"]
mod hearing_result_database_support;
#[path = "../hearing_result_import.rs"]
mod hearing_result_import;
#[path = "../hearing_result_inventory.rs"]
mod hearing_result_inventory;
#[allow(dead_code)]
#[path = "../hearing_result_inventory_support/mod.rs"]
mod hearing_result_inventory_support;
#[path = "../hearing_result_queries.rs"]
mod hearing_result_queries;
#[path = "../hearing_result_receipt_sql.rs"]
mod hearing_result_receipt_sql;
#[path = "../hearing_result_restore.rs"]
mod hearing_result_restore;
#[allow(dead_code)]
#[path = "../hearing_result_restore_support/mod.rs"]
mod hearing_result_restore_support;
#[allow(dead_code)]
#[path = "../hearing_result_revalidation_support/mod.rs"]
mod hearing_result_revalidation_support;
#[path = "../hearing_result_schema.rs"]
mod hearing_result_schema;
#[path = "../hearing_result_source_integrity.rs"]
mod hearing_result_source_integrity;
#[path = "../hearing_result_sql_guards.rs"]
mod hearing_result_sql_guards;
#[allow(dead_code)]
#[path = "../hearing_result_sql_support/mod.rs"]
mod hearing_result_sql_support;
#[allow(dead_code)]
#[path = "../legacy_database_support/mod.rs"]
mod legacy_database_support;
