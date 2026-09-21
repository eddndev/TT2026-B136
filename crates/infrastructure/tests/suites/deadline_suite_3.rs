// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
#[path = "../deadline_backend_support/mod.rs"]
mod deadline_backend_support;
#[path = "../deadline_profile_backend.rs"]
mod deadline_profile_backend;
#[path = "../deadline_profile_database_support/mod.rs"]
mod deadline_profile_database_support;
#[path = "../deadline_profile_events.rs"]
mod deadline_profile_events;
#[path = "../deadline_profile_import.rs"]
mod deadline_profile_import;
#[path = "../deadline_profile_restore.rs"]
mod deadline_profile_restore;
#[path = "../deadline_responsibles_backend.rs"]
mod deadline_responsibles_backend;
#[path = "../deadline_responsibles_concurrency.rs"]
mod deadline_responsibles_concurrency;
#[path = "../deadline_responsibles_support/mod.rs"]
mod deadline_responsibles_support;
#[path = "../deadline_restore.rs"]
mod deadline_restore;
#[allow(dead_code)]
#[path = "../deadline_restore_support/mod.rs"]
mod deadline_restore_support;
#[path = "../deadline_schema.rs"]
mod deadline_schema;
#[path = "../deadline_schema_support/mod.rs"]
mod deadline_schema_support;
#[path = "../deadline_source_event_catalog.rs"]
mod deadline_source_event_catalog;
#[path = "../deadline_source_event_restore.rs"]
mod deadline_source_event_restore;
#[path = "../deadline_storage.rs"]
mod deadline_storage;
#[path = "../deadline_storage_support/mod.rs"]
mod deadline_storage_support;
#[path = "../hearing_database_support/mod.rs"]
mod hearing_database_support;
#[path = "../hearing_result_database_support/mod.rs"]
mod hearing_result_database_support;
#[path = "../judicial_calendar_database_support/mod.rs"]
mod judicial_calendar_database_support;
#[allow(dead_code)]
#[path = "../legacy_database_support/mod.rs"]
mod legacy_database_support;
#[path = "../procedural_fact_backend_support/mod.rs"]
mod procedural_fact_backend_support;
use deadline_profile_database_support::values as profile_values;
