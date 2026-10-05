use application::cases::CaseRevision;
use application::precautionary_hearings::PrecautionaryContext;
use time::Duration;

use crate::effect_support::{substitution, LaterFixture};
use crate::measure_decision_fixtures::Fixture;
use crate::record_support::*;

#[test]
fn three_corrections_advance_exact_records_while_retaining_the_actual_judicial_evidence() {
    let first_fixture = RecordFixture::initial();
    let judicial = owned(&first_fixture.history.judicial.groups[0].capture);
    let first = first_fixture.capture();
    let second_fixture = RecordFixture::next(&first, &first_fixture.history, 1);
    let second = second_fixture.capture();
    let third_fixture = RecordFixture::next(&second, &second_fixture.history, 2);
    let third = third_fixture.capture();

    for (capture, fixture, revision, previous) in [
        (&first, &first_fixture, 2, reference(&judicial.capture)),
        (
            &second,
            &second_fixture,
            3,
            record_reference(&first.records[0]),
        ),
        (
            &third,
            &third_fixture,
            4,
            record_reference(&second.records[0]),
        ),
    ] {
        let evidence = append_administrative(&fixture.history, capture);
        let selected = record_reference(&capture.records[0]);
        let checked =
            resolve_measure_records(&Hasher, fixture.case_id, &[selected], &evidence).unwrap();
        let record = &checked.targets()[0];
        assert_eq!(record.reference(), selected);
        assert_eq!(record.reference().revision().get(), revision);
        assert_eq!(record.last_judicial(), &judicial);
        assert_eq!(record.judicial_origin(), judicial.capture.result.origin);
        assert_eq!(
            record.record_root(),
            MeasureRecordRoot::Judicial(judicial.capture.result.origin)
        );
        assert_eq!(record.last_action(), MeasureCaptureAction::Impose);
        assert_eq!(record.validity(), MeasureCaptureValidity::Valid);
        assert_eq!(record.values(), &capture.review.result.values);
        assert_eq!(record.sources(), &judicial.capture.result.sources);
        assert_eq!(record.projection(), &judicial.capture.result.projection);
        assert_eq!(record.support(), &first.review.support);
        let OwnedMeasureRecord::Administrative {
            owner,
            capture: row,
        } = record.record()
        else {
            panic!("administrative record expected")
        };
        assert_eq!(owner.operation_id, capture.review.command.operation_id);
        assert_eq!(owner.capture_digest, capture.capture_digest);
        assert_eq!(row.as_ref(), &capture.records[0]);
        assert_eq!(row.result.previous, previous);
        measure_administrative_capture_with_history_matches(&Hasher, capture, &fixture.history)
            .unwrap();
    }
    assert_ne!(first.review.result.values, second.review.result.values);
    assert_ne!(second.review.result.values, third.review.result.values);
}

#[test]
fn effective_context_and_clock_advance_without_rewriting_the_last_judicial_capture() {
    let mut first_fixture = RecordFixture::initial();
    let judicial_group = first_fixture.history.judicial.groups[0].capture.clone();
    let judicial = owned(&judicial_group);
    let mut context = first_fixture.context.material().clone();
    context.administration.revision = CaseRevision::new(2).unwrap();
    context.administration.changed_at = first_fixture.recorded_at;
    first_fixture.context = PrecautionaryContext::new(&Hasher, context).unwrap();
    first_fixture.command.context = expectation(&first_fixture.context);
    let first = first_fixture.capture();
    let mut next_fixture = RecordFixture::next(&first, &first_fixture.history, 1);
    let mut context = next_fixture.context.material().clone();
    context.administration.revision = CaseRevision::new(3).unwrap();
    context.administration.changed_at = next_fixture.recorded_at;
    next_fixture.context = PrecautionaryContext::new(&Hasher, context).unwrap();
    next_fixture.command.context = expectation(&next_fixture.context);
    let second = next_fixture.capture();
    let evidence = append_administrative(&next_fixture.history, &second);
    let selected = record_reference(&second.records[0]);
    let checked =
        resolve_measure_records(&Hasher, next_fixture.case_id, &[selected], &evidence).unwrap();
    let record = &checked.targets()[0];
    assert_eq!(record.context(), &next_fixture.context);
    assert_eq!(record.recorded_at(), next_fixture.recorded_at);
    assert_ne!(record.context(), &judicial_group.review.material.context);
    assert!(record.recorded_at() > record.last_judicial().capture.recorded_at);
    assert_eq!(record.last_judicial(), &judicial);
    assert_eq!(judicial_group, evidence.judicial.groups[0].capture);
}

#[test]
fn first_correction_wrappers_keep_identical_captures_and_repeated_corrections_require_mixed_history(
) {
    let legacy = CorrectionFixture::initial();
    let expected = legacy.capture();
    let first_fixture = RecordFixture::from_first(legacy.clone());
    let first = first_fixture.capture();
    assert_eq!(first, expected);
    assert_eq!(
        measure_administrative_origin(&Hasher, &first, &legacy.history).unwrap(),
        measure_administrative_origin_with_history(&Hasher, &first, &first_fixture.history)
            .unwrap(),
    );
    let second_fixture = RecordFixture::next(&first, &first_fixture.history, 1);
    let second = second_fixture.capture();
    assert!(prepare_measure_record_correction(
        &Hasher,
        &second_fixture.actor,
        second_fixture.case_id,
        second_fixture.command.clone(),
        second_fixture.context.clone(),
        &legacy.history,
    )
    .is_err());
    assert!(measure_administrative_capture_matches(&Hasher, &second, &legacy.history).is_err());
    assert!(measure_administrative_origin(&Hasher, &second, &legacy.history).is_err());
}

#[test]
fn repeated_corrections_of_terminal_judicial_records_remain_terminal() {
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
        let first_fixture =
            RecordFixture::from_first(CorrectionFixture::from_group(&group, &ancestors, id(70)));
        let first = first_fixture.capture();
        let second_fixture = RecordFixture::next(&first, &first_fixture.history, 1);
        let second = second_fixture.capture();
        let evidence = append_administrative(&second_fixture.history, &second);
        let checked = resolve_measure_records(
            &Hasher,
            second_fixture.case_id,
            &[record_reference(&second.records[0])],
            &evidence,
        )
        .unwrap();
        let record = &checked.targets()[0];
        let expected = match kind {
            0 => MeasureCaptureAction::Revoke,
            1 => MeasureCaptureAction::Cease,
            _ => MeasureCaptureAction::SubstituteOut,
        };
        assert_eq!(record.last_action(), expected);
        assert_eq!(record.validity(), MeasureCaptureValidity::Valid);
        assert!(record.values().validity().end().is_none());
        assert_eq!(record.last_judicial(), &owned(&group));
    }
}

#[test]
fn repeated_capture_floor_includes_previous_administrative_time() {
    let first_fixture = RecordFixture::initial();
    let first = first_fixture.capture();
    let second_fixture = RecordFixture::next(&first, &first_fixture.history, 1);
    let checked = second_fixture.prepare().unwrap();
    assert!(checked
        .into_capture(&Hasher, first.recorded_at - Duration::nanoseconds(1))
        .is_err());
    let capture = second_fixture
        .prepare()
        .unwrap()
        .into_capture(&Hasher, first.recorded_at)
        .unwrap();
    assert_eq!(capture.recorded_at, first.recorded_at);
    measure_administrative_capture_with_history_matches(&Hasher, &capture, &second_fixture.history)
        .unwrap();
}
