#[allow(dead_code, unused_imports)]
#[path = "measure_administrative_backend_support/fixture.rs"]
mod administrative_fixture;
mod agenda_precautionary_support;
mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
#[path = "precautionary_hearing_backend_support/fixture.rs"]
mod hearing_fixture;
#[path = "measure_decision_backend_support/fixture.rs"]
mod measure_fixture;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_hearing_record_backend_support/fixture.rs"]
mod record_fixture;
#[allow(unused_imports)]
mod typed_participant_service_support;
use agenda_precautionary_support::*;

#[path = "agenda_precautionary_lifecycle.rs"]
mod lifecycle;
#[path = "agenda_precautionary_proof.rs"]
mod proof;
