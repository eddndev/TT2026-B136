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
#[path = "precautionary_decision_review_support/mod.rs"]
mod decision_review_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_decision_effect_support/mod.rs"]
mod effect_support;
#[allow(dead_code, unused_imports)]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
mod measure_dependency_support;
#[allow(dead_code, unused_imports)]
mod measure_source_support;
#[path = "document_format_support/mod.rs"]
mod observed_crypto;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code, unused_imports)]
mod precautionary_receipt_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_decision_support/mod.rs"]
mod record_decision_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_record_review_support/mod.rs"]
mod record_review_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

mod measure_administrative_workflow_support;
use measure_administrative_workflow_support::*;

#[test]
fn prepare_correction_admits_the_retained_support_and_returns_the_exact_review() {
    let fixture = Fixture::single();
    let expected = fixture.review();
    let harness = harness(fixture.store(), identity(fixture.actor.clone()));
    let actual = harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .unwrap();

    assert_eq!(actual, expected);
    assert_eq!(harness.validator.calls(), 1);
    assert!(harness.events().contains(&"open"));
    assert!(harness.events().contains(&"hash"));
}

#[path = "measure_administrative_workflow_support/negative_tests.rs"]
mod negative_tests;
