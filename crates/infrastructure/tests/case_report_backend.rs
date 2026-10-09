#[path = "case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "case_report_support/atomicity_cases.rs"]
mod case_report_atomicity;
#[path = "case_report_support/leases.rs"]
mod case_report_leases;
#[path = "case_report_support/notices.rs"]
mod case_report_notices;
#[path = "case_report_support/requests.rs"]
mod case_report_requests;
#[path = "case_report_support/restore.rs"]
mod case_report_restore;
mod case_report_support;

#[path = "case_report_support/recovery.rs"]
mod case_report_recovery;

#[path = "case_report_support/lease_safety.rs"]
mod case_report_lease_safety;

#[path = "case_report_support/litigators.rs"]
mod case_report_litigators;

#[allow(dead_code, unused_imports)]
#[path = "measure_administrative_backend_support/fixture.rs"]
mod activity_administrative_fixture;
#[path = "case_report_support/activity_resources.rs"]
mod activity_resources;
#[path = "case_report_support/activity_sources.rs"]
mod activity_sources;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod hearing_database_support;
mod hearing_result_database_support;
#[path = "measure_decision_backend_support/fixture.rs"]
mod measure_fixture;
mod procedural_fact_backend_support;
mod procedural_resource_support;
#[allow(unused_imports)]
mod typed_participant_service_support;
