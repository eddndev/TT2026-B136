// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../alert_backend_support/mod.rs"]
mod alert_backend_support;
#[path = "../alert_catalog.rs"]
mod alert_catalog;
#[path = "../alert_delivery.rs"]
mod alert_delivery;
#[allow(dead_code)]
#[path = "../alert_delivery_support/mod.rs"]
mod alert_delivery_support;
#[path = "../alert_inbox.rs"]
mod alert_inbox;
#[path = "../alert_inventory.rs"]
mod alert_inventory;
#[path = "../alert_preferences.rs"]
mod alert_preferences;
#[path = "../alert_review_episode.rs"]
mod alert_review_episode;
#[path = "../alert_scheduler.rs"]
mod alert_scheduler;
#[path = "../alert_scheduler_edges.rs"]
mod alert_scheduler_edges;
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
#[path = "../deadline_profile_database_support/mod.rs"]
mod deadline_profile_database_support;
#[allow(dead_code)]
#[path = "../deadline_schema_support/mod.rs"]
mod deadline_schema_support;
#[allow(dead_code)]
#[path = "../hearing_database_support/mod.rs"]
mod hearing_database_support;
#[allow(dead_code)]
#[path = "../procedural_fact_backend_support/mod.rs"]
mod procedural_fact_backend_support;
