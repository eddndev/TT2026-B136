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

#[path = "measure_record_history_support/boundaries.rs"]
mod boundary_tests;
#[path = "measure_record_history_support/mod.rs"]
mod record_support;
#[path = "measure_record_history_support/selection.rs"]
mod selection_tests;
#[path = "measure_record_history_support/sequential.rs"]
mod sequential_tests;

#[path = "measure_record_history_support/negative_tests.rs"]
mod negative_tests;

use record_support::*;

#[test]
fn exact_administrative_record_resolves_with_its_actual_judicial_antecedent() {
    let fixture = RecordFixture::initial();
    let capture = fixture.capture();
    let evidence = append_administrative(&fixture.history, &capture);
    let selected = record_reference(&capture.records[0]);
    let checked =
        resolve_measure_records(&Hasher, fixture.case_id, &[selected], &evidence).unwrap();
    let result = &checked.targets()[0];

    assert_eq!(result.reference(), selected);
    assert_eq!(result.values(), &capture.review.result.values);
    assert_eq!(result.context(), &capture.review.context);
    assert_eq!(result.recorded_at(), capture.recorded_at);
    assert_eq!(
        result.last_judicial(),
        &owned(&fixture.history.judicial.groups[0].capture)
    );
    assert_eq!(result.last_action(), MeasureCaptureAction::Impose);
    assert_eq!(result.validity(), MeasureCaptureValidity::Valid);
    assert!(matches!(
        result.record(),
        OwnedMeasureRecord::Administrative { .. }
    ));
}
