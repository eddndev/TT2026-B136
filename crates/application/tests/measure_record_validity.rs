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
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

#[path = "measure_record_validity_support/lifecycle.rs"]
mod lifecycle_tests;
#[path = "measure_record_validity_support/recording.rs"]
mod recording_tests;
#[path = "measure_record_validity_support/mod.rs"]
mod validity_support;

#[path = "measure_record_validity_support/negative_tests.rs"]
mod negative_tests;
#[path = "measure_record_validity_support/vectors.rs"]
mod vectors;

use validity_support::*;

#[test]
fn mark_entered_in_error_preserves_the_record_and_appends_only_capture_invalidity() {
    let fixture = marking(RecordFixture::initial());
    let prior = &fixture.history.judicial.groups[0].capture.measures[0];
    let marked = capture(&fixture);
    let result = &marked.review.result;

    assert_eq!(result.id, prior.result.id);
    assert_eq!(result.revision.get(), 2);
    assert_eq!(result.previous, reference(prior));
    assert_eq!(result.validity, MeasureCaptureValidity::EnteredInError);
    assert_eq!(
        result.record_root,
        MeasureRecordRoot::Judicial(prior.result.origin)
    );
    assert_eq!(result.judicial_origin, prior.result.origin);
    assert_eq!(result.last_action, prior.result.action);
    assert_eq!(result.values, prior.result.values);
    assert_eq!(result.sources, prior.result.sources);
    assert_eq!(result.projection, prior.result.projection);
    assert_eq!(marked.records.len(), 1);
    assert_eq!(marked.records[0].result, *result);
    assert_eq!(marked.review.command.reason, fixture.command.reason);
    measure_administrative_capture_with_history_matches(&Hasher, &marked, &fixture.history)
        .unwrap();
}
