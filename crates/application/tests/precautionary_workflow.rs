#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[path = "document_format_support/mod.rs"]
mod observed_crypto;
#[allow(dead_code)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
mod precautionary_workflow_support;
#[allow(dead_code)]
#[path = "precautionary_receipt_support/mod.rs"]
mod receipt_support;

use precautionary_workflow_support::*;

#[test]
fn schedule_prepare_admits_exact_support_and_returns_full_review_without_commit() {
    let fixture = Fixture::schedule();
    let expected = fixture.review();
    let harness = harness(fixture.store(), identity(fixture.actor.clone()));
    let review = harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .unwrap();
    assert_eq!(review, expected);
    assert_eq!(harness.validator.calls(), 1);
    assert!(harness.events().contains(&"open"));
    assert!(harness.events().contains(&"hash"));
}

#[allow(dead_code, unused_imports)]
#[path = "measure_decision_fixtures/mod.rs"]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
#[path = "measure_source_support/mod.rs"]
mod measure_source_support;
