#[allow(dead_code)]
mod case_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_correction_capture_support/mod.rs"]
mod correction_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_decision_effect_support/mod.rs"]
mod effect_support;
#[allow(dead_code, unused_imports)]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
mod measure_source_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code, unused_imports)]
mod precautionary_receipt_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_decision_support/mod.rs"]
mod record_decision_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

mod measure_administrative_read_support;
use measure_administrative_read_support::*;

#[test]
fn authorized_operation_read_returns_the_original_administrative_receipt_and_closure() {
    let original = operation(10);
    let actor = reader(Role::Paralegal);
    let service = service(
        successful_store(&actor, &original, original.clone(), ReadKind::Operation),
        identity(&actor),
    );
    let result = service
        .get_operation(
            "session",
            original.capture.review.case_id,
            original.capture.review.command.operation_id,
        )
        .unwrap();
    assert_eq!(result, original);
    assert_ne!(result.capture.review.actor.id, actor.id);
}
