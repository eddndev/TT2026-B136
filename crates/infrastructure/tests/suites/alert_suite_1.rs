// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../alert_backend_support/mod.rs"]
mod alert_backend_support;
#[path = "../alert_catalog.rs"]
mod alert_catalog;
#[path = "../alert_delivery.rs"]
mod alert_delivery;
#[path = "../alert_delivery_support/mod.rs"]
mod alert_delivery_support;
#[path = "../alert_inbox.rs"]
mod alert_inbox;
#[path = "../alert_inventory.rs"]
mod alert_inventory;
#[path = "../alert_preferences.rs"]
mod alert_preferences;
#[path = "../alert_reschedule_inventory.rs"]
mod alert_reschedule_inventory;
#[path = "../alert_resource_hearing_inventory.rs"]
mod alert_resource_hearing_inventory;
#[path = "../alert_resource_hearing_schema.rs"]
mod alert_resource_hearing_schema;
#[path = "../alert_resource_hearing_support/mod.rs"]
mod alert_resource_hearing_support;
#[path = "../alert_resource_hearings.rs"]
mod alert_resource_hearings;
#[path = "../alert_review_episode.rs"]
mod alert_review_episode;
#[path = "../alert_scheduler.rs"]
mod alert_scheduler;
#[path = "../alert_scheduler_edges.rs"]
mod alert_scheduler_edges;
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
#[path = "../deadline_backend_support/mod.rs"]
mod deadline_backend_support;
#[path = "../deadline_profile_database_support/mod.rs"]
mod deadline_profile_database_support;
#[path = "../deadline_schema_support/mod.rs"]
mod deadline_schema_support;
#[path = "../hearing_database_support/mod.rs"]
mod hearing_database_support;
#[path = "../procedural_fact_backend_support/mod.rs"]
mod procedural_fact_backend_support;
#[path = "../procedural_resource_support/mod.rs"]
mod procedural_resource_support;
#[path = "../resource_activity_support/mod.rs"]
mod resource_activity_support;
#[path = "../resource_hearing_database_support/mod.rs"]
mod resource_hearing_database_support;
