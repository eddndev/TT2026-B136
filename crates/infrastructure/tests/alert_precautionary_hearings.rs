#[allow(dead_code, unused_imports)]
#[path = "measure_administrative_backend_support/fixture.rs"]
mod administrative_fixture;
mod alert_backend_support;
mod alert_precautionary_support;
#[allow(unused_imports)]
mod alert_resource_hearing_support;
mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod deadline_schema_support;
mod hearing_database_support;
#[path = "precautionary_hearing_backend_support/fixture.rs"]
mod hearing_fixture;
#[path = "measure_decision_backend_support/fixture.rs"]
mod measure_fixture;
mod procedural_fact_backend_support;
mod procedural_resource_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_hearing_record_backend_support/fixture.rs"]
mod record_fixture;
mod resource_activity_support;
mod resource_hearing_database_support;
#[allow(unused_imports)]
mod typed_participant_service_support;
use alert_precautionary_support::*;
#[path = "alert_precautionary_atomicity.rs"]
mod atomicity;
#[path = "alert_precautionary_lifecycle.rs"]
mod lifecycle;

#[path = "alert_precautionary_access.rs"]
mod access;
