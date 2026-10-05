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
mod measure_correction_vector_support;
#[allow(dead_code, unused_imports)]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
mod measure_decision_vector_support;
#[allow(dead_code, unused_imports)]
mod measure_source_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code, unused_imports)]
mod precautionary_receipt_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_record_review_support/mod.rs"]
mod record_review_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

#[path = "measure_record_decision_support/anchors.rs"]
mod anchor_tests;
#[path = "measure_record_decision_support/continuity.rs"]
mod continuity_tests;
#[path = "measure_record_decision_support/effects.rs"]
mod effect_tests;
#[path = "measure_record_decision_support/inventory.rs"]
mod inventory_tests;
#[path = "measure_record_decision_support/negative_tests.rs"]
mod negative_tests;
#[path = "measure_record_decision_support/mod.rs"]
mod record_decision_support;
#[path = "measure_record_decision_support/vectors.rs"]
mod vector_tests;

use record_decision_support::*;

#[test]
fn judicial_confirmation_uses_the_exact_corrected_record() {
    let correction = RecordFixture::initial();
    let administrative = correction.capture();
    let fixture = FixtureV2::confirm(&administrative, &correction.history);
    let group = fixture.capture();
    let result = &group.measures[0].result;

    assert_eq!(result.revision.get(), 3);
    assert_eq!(
        result.previous,
        Some(record_reference(&administrative.records[0]))
    );
    assert_eq!(result.values, administrative.review.result.values);
    assert_eq!(result.sources, administrative.review.result.sources);
    assert_eq!(result.projection, administrative.review.result.projection);
    assert_eq!(result.record_root, administrative.review.result.record_root);
    assert_eq!(
        result.judicial_origin,
        administrative.review.result.judicial_origin
    );
    assert_eq!(result.action, MeasureCaptureAction::Confirm);
    measure_decision_group_v2_matches(&Hasher, &group, &fixture.history).unwrap();
}
