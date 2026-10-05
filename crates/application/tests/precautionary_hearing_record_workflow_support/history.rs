use super::*;
use crate::record_support::{append_administrative, record_reference, RecordFixture};
use domain::hearings::HearingStatus;

#[test]
fn replacement_and_cancellation_keep_exact_corrected_revisions_and_the_original_hearing_origin() {
    let first = RecordFixture::initial();
    let correction = first.capture();
    let second = RecordFixture::next(&correction, &first.history, 1);
    let corrected_again = second.capture();
    let initial = Fixture::schedule().operation(at());
    let mut replacement = Fixture::replace(&initial);
    replacement.targets(vec![record_reference(&corrected_again.records[0])]);
    replacement.material.record_history.records =
        append_administrative(&second.history, &corrected_again);
    let expected = replacement.full_operation(now());
    let replaced = submit(replacement);
    assert_eq!(replaced, expected);
    assert_eq!(replaced.history.origin, initial.history.origin);
    assert_eq!(replaced.history.captures[0], initial.capture);
    assert_eq!(
        replaced.history.record_history.records.administrative.len(),
        2
    );
    assert_eq!(
        replaced.capture.review.resolved_values.review_targets(),
        &[record_reference(&corrected_again.records[0])]
    );

    let cancellation = Fixture::cancel(&replaced);
    let cancelled = submit(cancellation);
    assert_eq!(cancelled.capture.review.status, HearingStatus::Cancelled);
    assert_eq!(
        cancelled.capture.review.resolved_values,
        replaced.capture.review.resolved_values
    );
    assert_eq!(
        cancelled.capture.review.sources,
        replaced.capture.review.sources
    );
    assert_eq!(
        cancelled.capture.review.scheduling_context,
        replaced.capture.review.scheduling_context
    );
    assert_eq!(cancelled.history.origin, initial.history.origin);
    assert_eq!(cancelled.history.captures.len(), 3);
    assert_eq!(
        cancelled.history.record_history,
        replaced.history.record_history
    );
}

#[test]
fn historical_valid_record_remains_replayable_and_cancellable_after_a_later_mark() {
    use application::measure_corrections::{
        prepare_measure_administrative_record_with_history, MeasureAdministrativeAction,
        MeasureCaptureValidity,
    };
    let first = RecordFixture::initial();
    let prior = first.capture();
    let mut later = RecordFixture::next(&prior, &first.history, 1);
    later.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = prepare_measure_administrative_record_with_history(
        &Hasher,
        &later.actor,
        later.case_id,
        later.command,
        later.context,
        &later.history,
    )
    .unwrap()
    .into_capture(&Hasher, later.recorded_at)
    .unwrap();
    assert_eq!(
        marked.records[0].result.validity,
        MeasureCaptureValidity::EnteredInError
    );
    let fixture = Fixture::schedule();
    let original = fixture.operation(at());
    let h = harness(
        fixture.replay_store(original.clone()),
        identity(fixture.actor),
    );
    let replayed = h
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&original.capture.review),
        )
        .unwrap();
    assert_eq!(replayed, original);
    assert_eq!(h.validator.calls(), 0);
    assert!(h.events().is_empty());
    let cancelled = submit(Fixture::cancel(&original));
    assert_eq!(
        cancelled.capture.review.resolved_values,
        original.capture.review.resolved_values
    );
    assert_eq!(
        cancelled.history.record_history,
        original.history.record_history
    );
}

#[test]
fn legacy_receipt_lifts_into_the_mixed_envelope_without_reencoding() {
    use application::precautionary_measures::MeasureHistoryEvidence;
    let mut fixture = Fixture::schedule();
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut fixture.command.change else {
        unreachable!()
    };
    let mut input = values_input(values);
    input.purpose = PrecautionaryHearingPurpose::Imposition;
    input.review_targets.clear();
    *values = PrecautionaryHearingValues::new(input).unwrap();
    fixture.material.record_history.records = crate::record_support::empty();
    let mixed = fixture.operation(at());
    precautionary_hearing_receipt_with_measure_history_matches(
        &Hasher,
        &mixed.capture,
        &MeasureHistoryEvidence { groups: vec![] },
    )
    .unwrap();
    let h = harness(fixture.replay_store(mixed.clone()), identity(fixture.actor));
    assert_eq!(
        h.service
            .submit(
                "session",
                fixture.case_id,
                fixture.command,
                confirmation(&mixed.capture.review)
            )
            .unwrap(),
        mixed
    );
    assert!(h.events().is_empty());
}
