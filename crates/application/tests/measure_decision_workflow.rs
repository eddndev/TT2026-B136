#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
#[path = "measure_decision_effect_support/mod.rs"]
mod effect_support;
#[allow(dead_code, unused_imports)]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
mod measure_source_support;
#[path = "document_format_support/mod.rs"]
mod observed_crypto;
#[allow(dead_code)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code)]
mod precautionary_receipt_support;
use measure_decision_fixtures as decision_support;
#[allow(dead_code)]
mod decision_anchor_support;
mod measure_decision_workflow_support;

use measure_decision_workflow_support::*;

#[test]
fn prepare_imposition_admits_the_exact_document_and_returns_the_complete_review() {
    let fixture = Fixture::single();
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
