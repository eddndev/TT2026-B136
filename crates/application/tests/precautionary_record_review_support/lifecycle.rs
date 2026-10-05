use application::cases::CaseRevision;
use domain::hearings::HearingStatus;
use time::Duration;

use crate::precautionary_receipt_support::at;
use crate::record_review_support::*;

#[test]
fn replacement_and_cancellation_preserve_union_of_exact_administrative_revisions() {
    let first_fixture = RecordFixture::initial();
    let first = first_fixture.capture();
    let first_history = append_administrative(&first_fixture.history, &first);
    let scheduled = RecordReviewFixture::schedule(
        vec![record_reference(&first.records[0])],
        first_history.clone(),
    )
    .capture(None, first.recorded_at);
    let origin =
        precautionary_hearing_origin_with_record_history(&Hasher, &scheduled, &first_history)
            .unwrap();
    let second_fixture = RecordFixture::next(&first, &first_fixture.history, 1);
    let second = second_fixture.capture();
    let history = append_administrative(&second_fixture.history, &second);
    let replaced = RecordReviewFixture::replace(
        &scheduled,
        vec![record_reference(&second.records[0])],
        history.clone(),
    )
    .capture(Some(&scheduled), second.recorded_at + Duration::seconds(1));
    precautionary_hearing_transition_with_record_history_matches(
        &Hasher, &scheduled, &replaced, &history,
    )
    .unwrap();
    let cancelled = RecordReviewFixture::cancel(&replaced, history.clone())
        .capture(Some(&replaced), second.recorded_at + Duration::seconds(2));
    assert_eq!(cancelled.review.status, HearingStatus::Cancelled);
    assert_eq!(
        cancelled.review.resolved_values,
        replaced.review.resolved_values
    );
    assert_eq!(cancelled.review.sources, replaced.review.sources);
    assert_eq!(
        cancelled.review.scheduling_context,
        replaced.review.scheduling_context
    );
    assert_eq!(
        scheduled.review.resolved_values.review_targets()[0]
            .revision()
            .get(),
        2
    );
    assert_eq!(
        cancelled.review.resolved_values.review_targets()[0]
            .revision()
            .get(),
        3
    );
    precautionary_hearing_receipt_with_record_history_matches(&Hasher, &cancelled, &history)
        .unwrap();
    precautionary_hearing_transition_with_record_history_matches(
        &Hasher, &replaced, &cancelled, &history,
    )
    .unwrap();
    precautionary_hearing_history_with_record_history_matches(
        &Hasher,
        &[scheduled, replaced, cancelled],
        &origin,
        &history,
    )
    .unwrap();
}

#[test]
fn record_context_and_capture_time_can_advance_beyond_the_retained_judicial_evidence() {
    let mut correction = RecordFixture::initial();
    let original_group = correction.history.judicial.groups[0].capture.clone();
    let mut material = correction.context.material().clone();
    material.administration.revision = CaseRevision::new(2).unwrap();
    material.administration.changed_at = correction.recorded_at;
    correction.context = PrecautionaryContext::new(&Hasher, material).unwrap();
    correction.command.context = expectation(&correction.context);
    let record = correction.capture();
    let history = append_administrative(&correction.history, &record);
    let mut fixture =
        RecordReviewFixture::schedule(vec![record_reference(&record.records[0])], history.clone());
    set_context(&mut fixture.hearing, correction.context.clone());
    let hearing = fixture.capture(None, record.recorded_at);
    assert_eq!(hearing.review.scheduling_context, record.review.context);
    assert_eq!(hearing.recorded_at, record.recorded_at);
    assert!(hearing.recorded_at > original_group.recorded_at);
    assert_ne!(
        hearing.review.scheduling_context,
        original_group.review.material.context
    );
    assert_eq!(history.judicial.groups[0].capture, original_group);
    precautionary_hearing_receipt_with_record_history_matches(&Hasher, &hearing, &history).unwrap();
}

#[test]
fn an_older_valid_target_remains_historical_and_cancellable_after_a_later_mark() {
    let correction = RecordFixture::initial();
    let record = correction.capture();
    let history = append_administrative(&correction.history, &record);
    let selected = record_reference(&record.records[0]);
    let hearing = RecordReviewFixture::schedule(vec![selected], history.clone())
        .capture(None, record.recorded_at);
    let origin =
        precautionary_hearing_origin_with_record_history(&Hasher, &hearing, &history).unwrap();
    let mut mark = RecordFixture::next(&record, &correction.history, 1);
    mark.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = prepare_measure_administrative_record_with_history(
        &Hasher,
        &mark.actor,
        mark.case_id,
        mark.command.clone(),
        mark.context.clone(),
        &mark.history,
    )
    .unwrap()
    .into_capture(&Hasher, mark.recorded_at)
    .unwrap();
    let marked_history = append_administrative(&mark.history, &marked);
    let marked_target = resolve_measure_records(
        &Hasher,
        mark.case_id,
        &[record_reference(&marked.records[0])],
        &marked_history,
    )
    .unwrap();
    assert_eq!(
        marked_target.targets()[0].validity(),
        MeasureCaptureValidity::EnteredInError
    );

    precautionary_hearing_receipt_with_record_history_matches(&Hasher, &hearing, &history).unwrap();
    let cancelled = RecordReviewFixture::cancel(&hearing, history.clone())
        .capture(Some(&hearing), marked.recorded_at + Duration::seconds(1));
    assert_eq!(
        cancelled.review.resolved_values.review_targets(),
        &[selected]
    );
    precautionary_hearing_history_with_record_history_matches(
        &Hasher,
        &[hearing, cancelled],
        &origin,
        &history,
    )
    .unwrap();
}

#[test]
fn judicial_only_record_history_preserves_the_existing_hearing_capture_bytes() {
    let group = crate::measure_decision_fixtures::Fixture::single().capture();
    let history = MeasureRecordHistoryEvidence {
        judicial: crate::effect_support::append_history(
            &crate::effect_support::empty_history(),
            &group,
        ),
        administrative: vec![],
    };
    let fixture = RecordReviewFixture::schedule(vec![reference(&group.measures[0])], history);
    let expected = prepare_precautionary_hearing_with_history(
        &Hasher,
        &fixture.hearing.actor,
        fixture.hearing.case_id,
        fixture.hearing.command.clone(),
        PrecautionaryHearingPreparationMaterial {
            observed_context: fixture.hearing.context.clone(),
            sources: fixture.hearing.sources.clone(),
            predecessor: None,
            measure_history: &fixture.record_history.judicial,
        },
    )
    .unwrap()
    .into_capture(&Hasher, group.recorded_at)
    .unwrap();
    let actual = fixture.capture(None, group.recorded_at);
    assert_eq!(actual, expected);
    assert_eq!(
        precautionary_hearing_capture_bytes(&actual).unwrap(),
        precautionary_hearing_capture_bytes(&expected).unwrap()
    );
}

#[test]
fn imposition_with_empty_record_history_preserves_its_existing_capture() {
    let hearing = HearingFixture::schedule();
    let expected = hearing.clone().capture(None, at());
    let fixture = RecordReviewFixture {
        hearing,
        record_history: empty(),
    };
    assert_eq!(fixture.capture(None, at()), expected);
}
