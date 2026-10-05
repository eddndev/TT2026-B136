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
#[path = "measure_record_decision_support/mod.rs"]
mod record_decision_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_history_support/mod.rs"]
mod record_support;
#[path = "measure_administrative_replacement_support/mod.rs"]
mod replacement_support;

#[path = "measure_administrative_replacement_support/bounds.rs"]
mod bounds;
#[path = "measure_administrative_replacement_support/history.rs"]
mod history;
#[path = "measure_administrative_replacement_support/reconstruction.rs"]
mod reconstruction;
#[path = "measure_administrative_replacement_support/validation.rs"]
mod validation;
#[path = "measure_administrative_replacement_support/vector.rs"]
mod vector;

use replacement_support::*;

#[test]
fn replacement_marks_the_old_record_and_owns_the_correct_identity_atomically() {
    let fixture = ReplacementFixture::initial();
    let old = fixture.previous();
    let original_group = fixture.history.records.judicial.groups[0].capture.clone();
    let capture = fixture.capture();
    let marked = &capture.review.result;
    let replacement = capture.review.replacement.as_ref().unwrap();
    assert_eq!(marked.id, old.result.id);
    assert_eq!(marked.revision, old.result.revision.next().unwrap());
    assert_eq!(marked.previous, reference(&old));
    assert_eq!(marked.validity, MeasureCaptureValidity::EnteredInError);
    assert_eq!(marked.values, old.result.values);
    assert_eq!(marked.sources, old.result.sources);
    assert_eq!(marked.projection, old.result.projection);
    assert_eq!(
        marked.record_root,
        MeasureRecordRoot::Judicial(old.result.origin)
    );
    assert_eq!(replacement.id, fixture.replacement_id());
    assert_eq!(replacement.revision, MeasureRevision::new(1).unwrap());
    assert_eq!(replacement.previous, reference(&old));
    assert_eq!(replacement.validity, MeasureCaptureValidity::Valid);
    assert_eq!(
        replacement.record_root,
        MeasureRecordRoot::Administrative {
            operation_id: fixture.command.operation_id,
            measure_id: fixture.replacement_id(),
        }
    );
    assert_eq!(replacement.judicial_origin, old.result.origin);
    assert_eq!(replacement.last_judicial, marked.last_judicial);
    assert_eq!(replacement.last_action, old.result.action);
    assert_eq!(
        replacement.values,
        expected_values(&old.result.values, &fixture.subject)
    );
    assert_eq!(replacement.sources.subject, fixture.subject);
    assert_eq!(
        replacement.sources.supervisor,
        old.result.sources.supervisor
    );
    assert_eq!(replacement.projection.subject.id, fixture.subject.id);
    assert_eq!(
        capture.review.support,
        original_group.review.material.support
    );
    assert_eq!(capture.records.len(), 2);
    assert!(capture
        .records
        .windows(2)
        .all(|rows| rows[0].result.id.as_uuid() < rows[1].result.id.as_uuid()));
    let link = capture.replacement_link.as_ref().unwrap();
    assert_eq!(
        link.entered_in_error,
        record_reference(row(&capture, marked.id))
    );
    assert_eq!(
        link.replacement,
        record_reference(row(&capture, replacement.id))
    );
    assert_eq!(
        fixture.history.records.judicial.groups[0].capture,
        original_group
    );
    measure_administrative_capture_with_decision_history_matches(
        &Hasher,
        &capture,
        &fixture.history,
    )
    .unwrap();
}
