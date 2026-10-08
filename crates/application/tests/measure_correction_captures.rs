#[allow(dead_code, unused_imports)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
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

#[path = "measure_correction_capture_support/binding.rs"]
mod binding_tests;
#[path = "measure_correction_capture_support/mod.rs"]
mod correction_support;
#[path = "measure_correction_capture_support/retention.rs"]
mod retention_tests;
#[path = "measure_correction_capture_support/validation.rs"]
mod validation_tests;

#[path = "measure_correction_capture_support/evidence_tests.rs"]
mod evidence_tests;

use correction_support::*;

#[test]
fn correction_appends_one_administrative_row_from_the_exact_judicial_capture() {
    let fixture = CorrectionFixture::initial();
    let prior = fixture.previous();
    let capture = fixture.capture();
    let result = &capture.review.result;

    assert_eq!(result.id, prior.result.id);
    assert_eq!(result.revision, MeasureRevision::new(2).unwrap());
    assert_eq!(result.previous, reference(&prior));
    assert_eq!(
        result.record_root,
        MeasureRecordRoot::Judicial(prior.result.origin)
    );
    assert_eq!(result.judicial_origin, prior.result.origin);
    assert_eq!(result.last_judicial.reference, reference(&prior));
    assert_eq!(
        result.last_judicial.owner,
        owned(&fixture.previous_group()).owner
    );
    assert_eq!(result.last_action, MeasureCaptureAction::Impose);
    assert_eq!(result.validity, MeasureCaptureValidity::Valid);
    assert_eq!(result.sources, prior.result.sources);
    assert_eq!(result.projection, prior.result.projection);
    assert_eq!(result.values.subject(), prior.result.values.subject());
    assert_eq!(result.values.kind(), prior.result.values.kind());
    assert_eq!(
        result.values.conditions().as_str(),
        "Corrected recorded conditions"
    );
    assert_eq!(capture.records.len(), 1);
    assert_eq!(capture.records[0].result, *result);
    assert_eq!(
        capture.records[0].review_digest,
        capture.review.review_digest
    );
    assert_eq!(
        capture.records[0].operation_id,
        fixture.command.operation_id
    );
    assert_eq!(capture.records[0].recorded_at, capture.recorded_at);
    measure_administrative_capture_matches(&Hasher, &capture, &fixture.history).unwrap();
}
