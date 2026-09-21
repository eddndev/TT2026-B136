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
#[path = "../judicial_calendar_backend.rs"]
mod judicial_calendar_backend;
#[path = "../judicial_calendar_catalog.rs"]
mod judicial_calendar_catalog;
#[path = "../judicial_calendar_clock.rs"]
mod judicial_calendar_clock;
#[path = "../judicial_calendar_concurrency.rs"]
mod judicial_calendar_concurrency;
#[allow(dead_code)]
#[path = "../judicial_calendar_database_support/mod.rs"]
mod judicial_calendar_database_support;
#[path = "../judicial_calendar_import.rs"]
mod judicial_calendar_import;
#[allow(dead_code)]
#[path = "../judicial_calendar_interleaved_support/mod.rs"]
mod judicial_calendar_interleaved_support;
#[path = "../judicial_calendar_inventory.rs"]
mod judicial_calendar_inventory;
#[allow(dead_code)]
#[path = "../judicial_calendar_inventory_support/mod.rs"]
mod judicial_calendar_inventory_support;
#[path = "../judicial_calendar_queries.rs"]
mod judicial_calendar_queries;
#[path = "../judicial_calendar_restore.rs"]
mod judicial_calendar_restore;
#[path = "../judicial_calendar_revalidation.rs"]
mod judicial_calendar_revalidation;
#[path = "../judicial_calendar_schema.rs"]
mod judicial_calendar_schema;
#[path = "../judicial_calendar_vectors.rs"]
mod judicial_calendar_vectors;
#[allow(dead_code)]
#[path = "../legacy_database_support/mod.rs"]
mod legacy_database_support;
