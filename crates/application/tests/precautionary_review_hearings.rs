#[allow(dead_code, unused_imports)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[path = "review_hearing_support/continuity.rs"]
mod continuity_tests;
#[path = "review_hearing_support/identity_regressions.rs"]
mod identity_regressions;
#[path = "review_hearing_support/lifecycle.rs"]
mod lifecycle_tests;
#[allow(dead_code, unused_imports)]
#[path = "measure_decision_effect_support/mod.rs"]
mod measure_decision_effect_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_decision_fixtures/mod.rs"]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
#[path = "measure_source_support/mod.rs"]
mod measure_source_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_receipt_support/mod.rs"]
mod precautionary_receipt_support;
mod review_hearing_support;
#[path = "review_hearing_support/validation.rs"]
mod validation_tests;

use review_hearing_support::*;

#[test]
fn review_hearing_resolves_its_exact_measure_capture_from_the_actual_owning_group() {
    let group = MeasureFixture::single().capture();
    let fixture = ReviewFixture::schedule(&group);
    let evidence = fixture.measure_history.clone();
    let capture = fixture.capture(None, crate::precautionary_receipt_support::at());
    assert_eq!(
        capture.review.resolved_values.purpose(),
        PrecautionaryHearingPurpose::Review
    );
    assert_eq!(
        capture.review.resolved_values.review_targets(),
        &[crate::measure_decision_fixtures::reference(
            &group.measures[0]
        )]
    );
    precautionary_hearing_receipt_with_measure_history_matches(&Hasher, &capture, &evidence)
        .unwrap();
    let origin =
        precautionary_hearing_origin_with_measure_history(&Hasher, &capture, &evidence).unwrap();
    precautionary_hearing_history_with_measure_history_matches(
        &Hasher,
        std::slice::from_ref(&capture),
        &origin,
        &evidence,
    )
    .unwrap();
    assert!(precautionary_hearing_receipt_matches(&Hasher, &capture).is_err());
    assert!(precautionary_hearing_origin(&Hasher, &capture).is_err());
}
