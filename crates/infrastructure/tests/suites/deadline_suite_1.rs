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
#[path = "../deadline_backend_concurrency.rs"]
mod deadline_backend_concurrency;
#[path = "../deadline_backend_families.rs"]
mod deadline_backend_families;
#[path = "../deadline_backend_revalidation.rs"]
mod deadline_backend_revalidation;
#[allow(dead_code)]
#[path = "../deadline_backend_support/mod.rs"]
mod deadline_backend_support;
#[path = "../deadline_current_backend.rs"]
mod deadline_current_backend;
#[path = "../deadline_current_guards.rs"]
mod deadline_current_guards;
#[path = "../deadline_current_list_backend.rs"]
mod deadline_current_list_backend;
#[path = "../deadline_current_read_concurrency.rs"]
mod deadline_current_read_concurrency;
#[allow(dead_code)]
#[path = "../deadline_dispatch_atomic_support/mod.rs"]
mod deadline_dispatch_atomic_support;
#[path = "../deadline_dispatch_atomicity.rs"]
mod deadline_dispatch_atomicity;
#[path = "../deadline_dispatch_backend.rs"]
mod deadline_dispatch_backend;
#[path = "../deadline_dispatch_candidates.rs"]
mod deadline_dispatch_candidates;
#[path = "../deadline_dispatch_catalog.rs"]
mod deadline_dispatch_catalog;
#[path = "../deadline_dispatch_families.rs"]
mod deadline_dispatch_families;
#[allow(dead_code)]
#[path = "../deadline_dispatch_family_support/mod.rs"]
mod deadline_dispatch_family_support;
#[allow(dead_code)]
#[path = "../deadline_dispatch_support/mod.rs"]
mod deadline_dispatch_support;
#[allow(dead_code)]
#[path = "../deadline_input_history_support/mod.rs"]
mod deadline_input_history_support;
#[allow(dead_code)]
#[path = "../deadline_input_support/mod.rs"]
mod deadline_input_support;
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
#[path = "../deadline_tracked_guard_support/mod.rs"]
mod deadline_tracked_guard_support;
#[allow(dead_code)]
#[path = "../deadline_worker_backend_support/mod.rs"]
mod deadline_worker_backend_support;
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
#[path = "../procedural_fact_backend_support/mod.rs"]
mod procedural_fact_backend_support;
#[allow(dead_code)]
#[path = "../procedural_fact_concurrency_support/mod.rs"]
mod procedural_fact_concurrency_support;
#[allow(dead_code)]
#[path = "../deadline_profile_interleaved_support/rendezvous.rs"]
mod rendezvous;
