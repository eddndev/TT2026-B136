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
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

#[path = "precautionary_record_review_support/lifecycle.rs"]
mod lifecycle_tests;
#[path = "precautionary_record_review_support/mod.rs"]
mod record_review_support;
#[path = "precautionary_record_review_support/selection.rs"]
mod selection_tests;

#[path = "precautionary_record_review_support/negative_tests.rs"]
mod negative_tests;

use record_review_support::*;

#[test]
fn review_hearing_selects_the_exact_administrative_record_with_unchanged_hearing_frames() {
    let correction = RecordFixture::initial();
    let record = correction.capture();
    let selected = record_reference(&record.records[0]);
    let evidence = append_administrative(&correction.history, &record);
    let fixture = RecordReviewFixture::schedule(vec![selected], evidence.clone());
    let capture = fixture.capture(None, record.recorded_at);
    assert_eq!(capture.review.resolved_values.review_targets(), &[selected]);
    assert_eq!(
        capture.review.resolved_values.purpose(),
        PrecautionaryHearingPurpose::Review
    );
    assert_eq!(capture.recorded_at, record.recorded_at);
    precautionary_hearing_receipt_with_record_history_matches(&Hasher, &capture, &evidence)
        .unwrap();
    let origin =
        precautionary_hearing_origin_with_record_history(&Hasher, &capture, &evidence).unwrap();
    precautionary_hearing_history_with_record_history_matches(
        &Hasher,
        std::slice::from_ref(&capture),
        &origin,
        &evidence,
    )
    .unwrap();
}
