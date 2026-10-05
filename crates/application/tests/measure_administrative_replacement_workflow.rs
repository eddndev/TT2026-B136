#[allow(dead_code)]
mod case_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_correction_capture_support/mod.rs"]
mod correction_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code, unused_imports)]
#[path = "measure_decision_effect_support/mod.rs"]
mod effect_support;
#[allow(dead_code, unused_imports)]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
mod measure_source_support;
#[path = "document_format_support/mod.rs"]
mod observed_crypto;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_decision_support/mod.rs"]
mod record_decision_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_history_support/mod.rs"]
mod record_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_administrative_replacement_support/mod.rs"]
mod replacement_support;

#[path = "measure_administrative_replacement_workflow_support/lifecycle.rs"]
mod lifecycle;
#[path = "measure_administrative_replacement_workflow_support/validation.rs"]
mod validation;
#[path = "measure_administrative_replacement_workflow_support/mod.rs"]
mod workflow_support;
