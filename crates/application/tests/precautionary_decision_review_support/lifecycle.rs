use crate::decision_review_support::*;
use application::cases::CaseRevision;
use domain::hearings::HearingStatus;
use time::Duration;
use uuid::Uuid;

#[test]
fn replacement_and_cancel_bind_the_union_of_m1_c_m2_and_post_m2_c_revisions() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let first_group = &judicial.history.records.judicial.groups[0].capture;
    let first_correction = &judicial.history.records.administrative[0].capture;
    let first_history = MeasureDecisionRecordHistoryEvidence {
        records: MeasureRecordHistoryEvidence {
            judicial: judicial.history.records.judicial.clone(),
            administrative: vec![],
        },
        decisions: vec![],
    };
    let scheduled = DecisionReviewFixture::schedule(
        vec![reference(&first_group.measures[0])],
        first_history.clone(),
    )
    .capture(None, first_group.recorded_at);
    let origin =
        precautionary_hearing_origin_with_decision_history(&Hasher, &scheduled, &first_history)
            .unwrap();
    let administrative = AdministrativeFixture::after(&group, &judicial.history, 1);
    let corrected = administrative.capture();
    let full_history = append_administrative_decision_history(&administrative.history, &corrected);
    let targets = [
        (
            record_reference(&first_correction.records[0]),
            judicial.history.clone(),
            first_correction.recorded_at,
        ),
        (
            reference_v2(&group.measures[0]),
            administrative.history.clone(),
            group.recorded_at,
        ),
        (
            record_reference(&corrected.records[0]),
            full_history.clone(),
            corrected.recorded_at,
        ),
    ];
    let mut captures = vec![scheduled];
    for (index, (selected, history, at)) in targets.into_iter().enumerate() {
        let previous = captures.last().unwrap();
        let mut fixture = DecisionReviewFixture::replace(previous, vec![selected], history.clone());
        fixture.hearing.command.operation_id =
            PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(4000 + index as u128));
        let next = fixture.capture(Some(previous), at);
        precautionary_hearing_transition_with_decision_history_matches(
            &Hasher, previous, &next, &history,
        )
        .unwrap();
        captures.push(next);
    }
    let previous = captures.last().unwrap();
    let cancelled = DecisionReviewFixture::cancel(previous, full_history.clone())
        .capture(Some(previous), corrected.recorded_at + Duration::seconds(1));
    assert_eq!(cancelled.review.status, HearingStatus::Cancelled);
    assert_eq!(
        cancelled.review.resolved_values,
        previous.review.resolved_values
    );
    assert_eq!(cancelled.review.sources, previous.review.sources);
    assert_eq!(cancelled.review.participants, previous.review.participants);
    assert_eq!(
        cancelled.review.scheduling_context,
        previous.review.scheduling_context
    );
    captures.push(cancelled);
    assert_eq!(
        captures
            .iter()
            .map(|h| h.review.resolved_values.review_targets()[0]
                .revision()
                .get())
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 4]
    );
    precautionary_hearing_history_with_decision_history_matches(
        &Hasher,
        &captures,
        &origin,
        &full_history,
    )
    .unwrap();
    precautionary_hearing_receipt_with_decision_history_matches(
        &Hasher,
        captures.last().unwrap(),
        &full_history,
    )
    .unwrap();
}

#[test]
fn post_m2_correction_supplies_its_own_effective_context_and_clock() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let mut administrative = AdministrativeFixture::after(&group, &judicial.history, 1);
    let mut context = administrative.context.material().clone();
    context.administration.revision = CaseRevision::new(2).unwrap();
    context.administration.changed_at = administrative.recorded_at;
    administrative.context = PrecautionaryContext::new(&Hasher, context).unwrap();
    administrative.command.context = expectation(&administrative.context);
    let corrected = administrative.capture();
    let history = append_administrative_decision_history(&administrative.history, &corrected);
    let mut fixture = DecisionReviewFixture::schedule(
        vec![record_reference(&corrected.records[0])],
        history.clone(),
    );
    set_context(&mut fixture.hearing, administrative.context.clone());
    let hearing = fixture.capture(None, corrected.recorded_at);
    assert_eq!(hearing.review.scheduling_context, corrected.review.context);
    assert_ne!(
        hearing.review.scheduling_context,
        group.review.material.context
    );
    assert_eq!(hearing.recorded_at, corrected.recorded_at);
    assert!(hearing.recorded_at > group.recorded_at);
    assert_eq!(
        corrected.review.result.last_judicial.reference,
        reference_v2(&group.measures[0])
    );
    precautionary_hearing_receipt_with_decision_history_matches(&Hasher, &hearing, &history)
        .unwrap();
}

#[test]
fn an_old_valid_m2_reference_remains_cancellable_after_a_later_mark() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let history = append_v2(&judicial.history, &group);
    let selected = reference_v2(&group.measures[0]);
    let hearing = DecisionReviewFixture::schedule(vec![selected], history.clone())
        .capture(None, group.recorded_at);
    let origin =
        precautionary_hearing_origin_with_decision_history(&Hasher, &hearing, &history).unwrap();
    let mut mark = AdministrativeFixture::after(&group, &judicial.history, 1);
    mark.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = mark.capture();
    assert_eq!(
        marked.review.result.validity,
        MeasureCaptureValidity::EnteredInError
    );
    let marked_history = append_administrative_decision_history(&mark.history, &marked);
    let checked = resolve_measure_records_with_decision_history(
        &Hasher,
        mark.case_id,
        &[record_reference(&marked.records[0])],
        &marked_history,
    )
    .unwrap();
    assert_eq!(
        checked.targets()[0].validity(),
        MeasureCaptureValidity::EnteredInError
    );
    precautionary_hearing_receipt_with_decision_history_matches(&Hasher, &hearing, &history)
        .unwrap();
    let cancelled = DecisionReviewFixture::cancel(&hearing, history.clone())
        .capture(Some(&hearing), marked.recorded_at + Duration::seconds(1));
    assert_eq!(
        cancelled.review.resolved_values.review_targets(),
        &[selected]
    );
    precautionary_hearing_history_with_decision_history_matches(
        &Hasher,
        &[hearing, cancelled],
        &origin,
        &history,
    )
    .unwrap();
}
