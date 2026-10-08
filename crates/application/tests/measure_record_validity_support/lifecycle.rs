use crate::effect_support::{empty_history, substitution, LaterFixture};
use crate::measure_decision_fixtures::Fixture;
use crate::validity_support::*;

#[test]
fn mark_after_repeated_corrections_preserves_effective_values_and_original_judicial_evidence() {
    let first_fixture = RecordFixture::initial();
    let first = first_fixture.capture();
    let second_fixture = RecordFixture::next(&first, &first_fixture.history, 1);
    let second = second_fixture.capture();
    let marked_fixture = marking(RecordFixture::next(&second, &second_fixture.history, 2));
    let marked = capture(&marked_fixture);
    let prior = &second.review.result;
    let result = &marked.review.result;
    assert_eq!(result.revision.get(), 4);
    assert_eq!(result.previous, record_reference(&second.records[0]));
    assert_eq!(result.values, prior.values);
    assert_eq!(result.sources, prior.sources);
    assert_eq!(result.projection, prior.projection);
    assert_eq!(result.record_root, prior.record_root);
    assert_eq!(result.judicial_origin, prior.judicial_origin);
    assert_eq!(result.last_judicial, prior.last_judicial);
    assert_eq!(result.last_action, prior.last_action);
    assert_eq!(result.validity, MeasureCaptureValidity::EnteredInError);
    assert_eq!(marked.review.support, second.review.support);
    assert_eq!(marked.records[0].result, *result);
    measure_administrative_capture_with_history_matches(&Hasher, &marked, &marked_fixture.history)
        .unwrap();
}

#[test]
fn historical_resolution_returns_the_exact_entered_in_error_record_without_changing_earlier_reads()
{
    let first_fixture = RecordFixture::initial();
    let first = first_fixture.capture();
    let marked_fixture = marking(RecordFixture::next(&first, &first_fixture.history, 1));
    let marked = capture(&marked_fixture);
    let evidence = append_administrative(&marked_fixture.history, &marked);
    let checked = resolve_measure_records(
        &Hasher,
        marked_fixture.case_id,
        &[record_reference(&marked.records[0])],
        &evidence,
    )
    .unwrap();
    let record = &checked.targets()[0];
    assert_eq!(record.validity(), MeasureCaptureValidity::EnteredInError);
    assert_eq!(record.last_action(), MeasureCaptureAction::Impose);
    assert_eq!(record.values(), &first.review.result.values);
    assert_eq!(record.recorded_at(), marked.recorded_at);
    let earlier = resolve_measure_records(
        &Hasher,
        first_fixture.case_id,
        &[record_reference(&first.records[0])],
        &marked_fixture.history,
    )
    .unwrap();
    assert_eq!(
        earlier.targets()[0].validity(),
        MeasureCaptureValidity::Valid
    );
    assert_eq!(earlier.targets()[0].recorded_at(), first.recorded_at);
    measure_administrative_capture_with_history_matches(&Hasher, &first, &first_fixture.history)
        .unwrap();
    let origin =
        measure_administrative_origin_with_history(&Hasher, &marked, &marked_fixture.history)
            .unwrap();
    assert_eq!(origin.capture_digest, marked.capture_digest);
    assert_eq!(origin.review_digest, marked.review.review_digest);
}

#[test]
fn marking_terminal_judicial_records_retains_their_judicial_action_and_terms() {
    let initial = Fixture::single().capture();
    for kind in 0..3 {
        let mut later = if kind == 2 {
            substitution(&initial, &[90])
        } else {
            LaterFixture::confirm(&initial)
        };
        let previous = reference(&initial.measures[0]);
        if kind != 2 {
            later.effects(vec![if kind == 0 {
                MeasureEffect::Revoke { previous }
            } else {
                MeasureEffect::Cease { previous }
            }]);
        }
        let ancestors = later.evidence.clone();
        let group = later.capture();
        let fixture = marking(RecordFixture::from_first(CorrectionFixture::from_group(
            &group,
            &ancestors,
            id(70),
        )));
        let marked = capture(&fixture);
        let prior = &group.measures[0];
        assert_eq!(marked.review.result.last_action, prior.result.action);
        assert_eq!(marked.review.result.values, prior.result.values);
        assert_eq!(
            marked.review.result.last_judicial.reference,
            reference(prior)
        );
        assert_eq!(
            marked.review.result.validity,
            MeasureCaptureValidity::EnteredInError
        );
        assert!(marked.review.result.values.validity().end().is_none());
        measure_administrative_capture_with_history_matches(&Hasher, &marked, &fixture.history)
            .unwrap();
    }
}

#[test]
fn marking_substitution_successor_preserves_its_actual_origin_and_shared_relation() {
    let initial = Fixture::multiple().capture();
    let later = substitution(&initial, &[90, 100]);
    let ancestors = later.evidence.clone();
    let group = later.capture();
    let fixture = marking(RecordFixture::from_first(CorrectionFixture::from_group(
        &group,
        &ancestors,
        id(100),
    )));
    let marked = capture(&fixture);
    let prior = group
        .measures
        .iter()
        .find(|row| row.result.id == id(100))
        .unwrap();
    assert_eq!(
        marked.review.result.record_root,
        MeasureRecordRoot::Judicial(prior.result.origin)
    );
    assert_eq!(marked.review.result.judicial_origin, prior.result.origin);
    assert_eq!(
        marked.review.result.last_action,
        MeasureCaptureAction::SubstituteIn
    );
    assert_eq!(marked.review.result.values, prior.result.values);
    assert_eq!(marked.records.len(), 1);
    assert_eq!(
        fixture.history.judicial.groups.last().unwrap().capture,
        group
    );
}

#[test]
fn marking_one_member_does_not_mark_or_rewrite_its_siblings() {
    let group = Fixture::multiple().capture();
    let fixture = marking(RecordFixture::from_first(CorrectionFixture::from_group(
        &group,
        &empty_history(),
        id(80),
    )));
    let marked = capture(&fixture);
    let evidence = append_administrative(&fixture.history, &marked);
    let checked = resolve_measure_records(
        &Hasher,
        fixture.case_id,
        &[
            record_reference(&marked.records[0]),
            reference(&group.measures[0]),
        ],
        &evidence,
    )
    .unwrap();
    assert_eq!(
        checked.targets()[0].reference(),
        reference(&group.measures[0])
    );
    assert_eq!(
        checked.targets()[0].validity(),
        MeasureCaptureValidity::Valid
    );
    assert_eq!(
        checked.targets()[0].values(),
        &group.measures[0].result.values
    );
    assert_eq!(
        checked.targets()[1].reference(),
        record_reference(&marked.records[0])
    );
    assert_eq!(
        checked.targets()[1].validity(),
        MeasureCaptureValidity::EnteredInError
    );
    assert_eq!(marked.records.len(), 1);
    assert_eq!(evidence.judicial.groups[0].capture, group);
}

#[test]
fn generic_preparation_preserves_ordinary_correction_bytes() {
    let fixture = RecordFixture::initial();
    let corrected = fixture.capture();
    assert_eq!(capture(&fixture), corrected);
    let next_fixture = RecordFixture::next(&corrected, &fixture.history, 1);
    assert_eq!(capture(&next_fixture), next_fixture.capture());
}

#[test]
fn correction_only_wrappers_reject_mark_but_historical_wrappers_validate_its_receipt() {
    let fixture = marking(RecordFixture::initial());
    assert!(fixture.prepare().is_err());
    assert!(prepare_measure_record_correction(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command.clone(),
        fixture.context.clone(),
        &fixture.history.judicial,
    )
    .is_err());
    let marked = capture(&fixture);
    measure_administrative_capture_matches(&Hasher, &marked, &fixture.history.judicial).unwrap();
    assert_eq!(
        measure_administrative_origin(&Hasher, &marked, &fixture.history.judicial).unwrap(),
        measure_administrative_origin_with_history(&Hasher, &marked, &fixture.history).unwrap(),
    );
}
