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
#[path = "../deadline_backend_support/mod.rs"]
mod deadline_backend_support;
#[path = "../deadline_profile_backend.rs"]
mod deadline_profile_backend;
#[allow(dead_code)]
#[path = "../deadline_profile_database_support/mod.rs"]
mod deadline_profile_database_support;
#[path = "../deadline_profile_events.rs"]
mod deadline_profile_events;
#[path = "../deadline_profile_import.rs"]
mod deadline_profile_import;
#[path = "../deadline_profile_restore.rs"]
mod deadline_profile_restore;
#[path = "../deadline_profile_schema.rs"]
mod deadline_profile_schema;
#[allow(dead_code)]
#[path = "../deadline_profile_schema_support/mod.rs"]
mod deadline_profile_schema_support;
#[path = "../deadline_responsibles_backend.rs"]
mod deadline_responsibles_backend;
#[path = "../deadline_responsibles_concurrency.rs"]
mod deadline_responsibles_concurrency;
#[allow(dead_code)]
#[path = "../deadline_responsibles_support/mod.rs"]
mod deadline_responsibles_support;
#[path = "../deadline_restore.rs"]
mod deadline_restore;
#[allow(dead_code)]
#[path = "../deadline_restore_support/mod.rs"]
mod deadline_restore_support;
#[path = "../deadline_schema.rs"]
mod deadline_schema;
#[allow(dead_code)]
#[path = "../deadline_schema_support/mod.rs"]
mod deadline_schema_support;
#[path = "../deadline_source_event_catalog.rs"]
mod deadline_source_event_catalog;
#[path = "../deadline_source_event_restore.rs"]
mod deadline_source_event_restore;
#[path = "../deadline_storage.rs"]
mod deadline_storage;
#[allow(dead_code)]
#[path = "../deadline_storage_support/mod.rs"]
mod deadline_storage_support;
#[allow(dead_code)]
#[path = "../hearing_database_support/mod.rs"]
mod hearing_database_support;
#[allow(dead_code)]
#[path = "../hearing_result_database_support/mod.rs"]
mod hearing_result_database_support;
#[allow(dead_code)]
#[path = "../judicial_calendar_database_support/mod.rs"]
mod judicial_calendar_database_support;
#[allow(dead_code)]
#[path = "../legacy_database_support/mod.rs"]
mod legacy_database_support;
#[allow(dead_code)]
#[path = "../procedural_fact_backend_support/mod.rs"]
mod procedural_fact_backend_support;
#[allow(dead_code)]
#[path = "../../../application/tests/deadline_profile_catalog_support/values.rs"]
mod profile_values;
