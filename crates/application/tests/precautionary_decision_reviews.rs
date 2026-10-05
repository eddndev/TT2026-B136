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
#[path = "precautionary_record_review_support/mod.rs"]
mod record_review_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

#[path = "precautionary_decision_review_support/compatibility.rs"]
mod compatibility_tests;
#[path = "precautionary_decision_review_support/mod.rs"]
mod decision_review_support;
#[path = "precautionary_decision_review_support/lifecycle.rs"]
mod lifecycle_tests;
#[path = "precautionary_decision_review_support/selection.rs"]
mod selection_tests;

use decision_review_support::*;

#[test]
fn review_hearing_selects_a_genuine_m2_with_its_complete_owner_history() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let history = append_v2(&judicial.history, &group);
    let selected = reference_v2(&group.measures[0]);
    let fixture = DecisionReviewFixture::schedule(vec![selected], history.clone());
    let capture = fixture.capture(None, group.recorded_at);

    assert_eq!(capture.review.resolved_values.review_targets(), &[selected]);
    assert_eq!(capture.recorded_at, group.recorded_at);
    precautionary_hearing_receipt_with_decision_history_matches(&Hasher, &capture, &history)
        .unwrap();
    let origin =
        precautionary_hearing_origin_with_decision_history(&Hasher, &capture, &history).unwrap();
    precautionary_hearing_history_with_decision_history_matches(
        &Hasher,
        std::slice::from_ref(&capture),
        &origin,
        &history,
    )
    .unwrap();
}

#[path = "precautionary_decision_review_support/negative_tests.rs"]
mod negative_tests;
