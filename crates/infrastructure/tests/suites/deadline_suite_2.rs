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
#[allow(dead_code)]
#[path = "../deadline_dispatch_family_support/mod.rs"]
mod deadline_dispatch_family_support;
#[allow(dead_code)]
#[path = "../deadline_dispatch_guard_support/mod.rs"]
mod deadline_dispatch_guard_support;
#[path = "../deadline_dispatch_guards.rs"]
mod deadline_dispatch_guards;
#[path = "../deadline_dispatch_inventory.rs"]
mod deadline_dispatch_inventory;
#[path = "../deadline_dispatch_reconnect.rs"]
mod deadline_dispatch_reconnect;
#[path = "../deadline_dispatch_restore.rs"]
mod deadline_dispatch_restore;
#[allow(dead_code)]
#[path = "../deadline_dispatch_support/mod.rs"]
mod deadline_dispatch_support;
#[allow(dead_code)]
#[path = "../deadline_dispatch_timeout_support/mod.rs"]
mod deadline_dispatch_timeout_support;
#[path = "../deadline_dispatch_timeouts.rs"]
mod deadline_dispatch_timeouts;
#[path = "../deadline_import.rs"]
mod deadline_import;
#[path = "../deadline_input_history.rs"]
mod deadline_input_history;
#[allow(dead_code)]
#[path = "../deadline_input_history_support/mod.rs"]
mod deadline_input_history_support;
#[path = "../deadline_input_permissions.rs"]
mod deadline_input_permissions;
#[path = "../deadline_input_projection.rs"]
mod deadline_input_projection;
#[allow(dead_code)]
#[path = "../deadline_input_projection_support/mod.rs"]
mod deadline_input_projection_support;
#[allow(dead_code)]
#[path = "../deadline_input_support/mod.rs"]
mod deadline_input_support;
#[path = "../deadline_legacy_parent_guard.rs"]
mod deadline_legacy_parent_guard;
#[path = "../deadline_observations_sql.rs"]
mod deadline_observations_sql;
#[allow(dead_code)]
#[path = "../deadline_observations_sql_support/mod.rs"]
mod deadline_observations_sql_support;
#[path = "../deadline_preparation_parent.rs"]
mod deadline_preparation_parent;
#[allow(dead_code)]
#[path = "../deadline_profile_database_support/mod.rs"]
mod deadline_profile_database_support;
#[allow(dead_code)]
#[path = "../deadline_schema_support/mod.rs"]
mod deadline_schema_support;
#[allow(dead_code)]
#[path = "../deadline_tracked_backend_support/mod.rs"]
mod deadline_tracked_backend_support;
#[allow(dead_code)]
#[path = "../deadline_tracked_notification_support/mod.rs"]
mod deadline_tracked_notification_support;
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
