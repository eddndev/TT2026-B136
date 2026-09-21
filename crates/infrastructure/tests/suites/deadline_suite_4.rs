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
#[path = "../deadline_observations_sql_support/mod.rs"]
mod deadline_observations_sql_support;
#[allow(dead_code)]
#[path = "../deadline_profile_database_support/mod.rs"]
mod deadline_profile_database_support;
#[allow(dead_code)]
#[path = "../deadline_schema_support/mod.rs"]
mod deadline_schema_support;
#[path = "../deadline_tracked_backend.rs"]
mod deadline_tracked_backend;
#[allow(dead_code)]
#[path = "../deadline_tracked_backend_support/mod.rs"]
mod deadline_tracked_backend_support;
#[allow(dead_code)]
#[path = "../deadline_tracked_guard_support/mod.rs"]
mod deadline_tracked_guard_support;
#[path = "../deadline_tracked_human_commit.rs"]
mod deadline_tracked_human_commit;
#[path = "../deadline_tracked_human_guards.rs"]
mod deadline_tracked_human_guards;
#[path = "../deadline_tracked_integrity_guards.rs"]
mod deadline_tracked_integrity_guards;
#[allow(dead_code)]
#[path = "../deadline_tracked_notification_support/mod.rs"]
mod deadline_tracked_notification_support;
#[path = "../deadline_tracked_notifications.rs"]
mod deadline_tracked_notifications;
#[path = "../deadline_tracked_readback.rs"]
mod deadline_tracked_readback;
#[allow(dead_code)]
#[path = "../deadline_tracked_readback_support/mod.rs"]
mod deadline_tracked_readback_support;
#[path = "../deadline_tracking_consistency_sql.rs"]
mod deadline_tracking_consistency_sql;
#[path = "../deadline_tracking_functions.rs"]
mod deadline_tracking_functions;
#[path = "../deadline_tracking_receipt_rejections.rs"]
mod deadline_tracking_receipt_rejections;
#[allow(dead_code)]
#[path = "../deadline_tracking_receipt_support/mod.rs"]
mod deadline_tracking_receipt_support;
#[path = "../deadline_tracking_receipts.rs"]
mod deadline_tracking_receipts;
#[path = "../deadline_tracking_schema.rs"]
mod deadline_tracking_schema;
#[path = "../deadline_tracking_schema_upgrade.rs"]
mod deadline_tracking_schema_upgrade;
#[allow(dead_code)]
#[path = "../deadline_tracking_sql_support/mod.rs"]
mod deadline_tracking_sql_support;
#[allow(dead_code)]
#[path = "../deadline_tracking_upgrade_support/mod.rs"]
mod deadline_tracking_upgrade_support;
#[allow(dead_code)]
#[path = "../procedural_fact_backend_support/mod.rs"]
mod procedural_fact_backend_support;
