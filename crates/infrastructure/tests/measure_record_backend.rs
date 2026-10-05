#[allow(dead_code, unused_imports)]
#[path = "measure_administrative_backend_support/fixture.rs"]
mod administrative_fixture;
#[path = "case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
#[path = "measure_decision_backend_support/fixture.rs"]
mod measure_fixture;
#[allow(unused_imports)]
#[path = "typed_participant_service_support/mod.rs"]
mod typed_participant_service_support;

mod measure_record_backend_support;
