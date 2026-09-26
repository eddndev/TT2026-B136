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
#[allow(dead_code)]
#[path = "../deadline_dispatch_family_support/mod.rs"]
mod deadline_dispatch_family_support;
#[allow(dead_code)]
#[path = "../deadline_dispatch_support/mod.rs"]
mod deadline_dispatch_support;
#[path = "../deadline_input_support/mod.rs"]
mod deadline_input_support;
#[path = "../deadline_observations_sql_support/mod.rs"]
mod deadline_observations_sql_support;
#[path = "../deadline_profile_database_support/mod.rs"]
mod deadline_profile_database_support;
#[path = "../deadline_schema_support/mod.rs"]
mod deadline_schema_support;
#[path = "../deadline_tracked_backend_support/mod.rs"]
mod deadline_tracked_backend_support;
#[path = "../deadline_tracking_sql.rs"]
mod deadline_tracking_sql;
#[path = "../deadline_tracking_sql_support/mod.rs"]
mod deadline_tracking_sql_support;
#[path = "../deadline_worker_backend.rs"]
mod deadline_worker_backend;
#[allow(dead_code)]
#[path = "../deadline_worker_backend_support/mod.rs"]
mod deadline_worker_backend_support;
#[path = "../deadline_worker_catalog.rs"]
mod deadline_worker_catalog;
#[allow(dead_code)]
#[path = "../deadline_worker_extra_support/mod.rs"]
mod deadline_worker_extra_support;
#[path = "../deadline_worker_families.rs"]
mod deadline_worker_families;
#[allow(dead_code)]
#[path = "../deadline_worker_guard_support/mod.rs"]
mod deadline_worker_guard_support;
#[path = "../deadline_worker_guards.rs"]
mod deadline_worker_guards;
#[path = "../deadline_worker_history.rs"]
mod deadline_worker_history;
#[path = "../deadline_worker_inventory.rs"]
mod deadline_worker_inventory;
#[path = "../deadline_worker_profile_failure.rs"]
mod deadline_worker_profile_failure;
#[path = "../deadline_worker_recovery.rs"]
mod deadline_worker_recovery;
#[path = "../deadline_worker_retries.rs"]
mod deadline_worker_retries;
#[allow(dead_code)]
#[path = "../deadline_worker_retry_support/mod.rs"]
mod deadline_worker_retry_support;
#[path = "../deadline_worker_sql_failures.rs"]
mod deadline_worker_sql_failures;
#[path = "../hearing_database_support/mod.rs"]
mod hearing_database_support;
#[path = "../hearing_result_database_support/mod.rs"]
mod hearing_result_database_support;
#[path = "../judicial_calendar_database_support/mod.rs"]
mod judicial_calendar_database_support;
#[path = "../procedural_fact_backend_support/mod.rs"]
mod procedural_fact_backend_support;
#[allow(dead_code)]
#[path = "../deadline_worker_extra_support/recovery.rs"]
mod recovery;
#[allow(dead_code)]
#[path = "../deadline_worker_extra_support/calendar.rs"]
mod worker_calendar;
