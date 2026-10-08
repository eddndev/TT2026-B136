#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code)]
#[path = "measure_decision_effect_support/mod.rs"]
mod effect_support;
#[allow(dead_code, unused_imports)]
mod measure_decision_fixtures;
mod measure_decision_read_support;
#[allow(dead_code, unused_imports)]
mod measure_source_support;
#[allow(dead_code)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code)]
mod precautionary_receipt_support;

use measure_decision_read_support::*;

#[test]
fn authorized_get_returns_the_original_complete_decision_and_its_owning_evidence() {
    let original = operation(10);
    let actor = reader(Role::Paralegal);
    let service = service(
        successful_store(&actor, &original, original.clone(), ReadKind::Get),
        identity(&actor),
    );
    let result = service
        .get(
            "session",
            original.group.review.case_id,
            original.group.decision.decision_id,
        )
        .unwrap();
    assert_eq!(result, original);
    assert_ne!(result.group.review.actor.id, actor.id);
}

use measure_decision_fixtures as decision_support;
#[allow(dead_code)]
mod decision_anchor_support;
