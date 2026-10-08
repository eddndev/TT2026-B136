use crate::record_decision_support::*;
use time::Duration;
use uuid::Uuid;

fn correction_after_v2(
    group: &MeasureDecisionGroupCaptureV2,
    ancestors: &MeasureDecisionRecordHistoryEvidence,
    mark: bool,
) -> (
    MeasureAdministrativeCapture,
    MeasureDecisionRecordHistoryEvidence,
) {
    let history = append_v2(ancestors, group);
    let prior = &group.measures[0];
    let supervision = match prior.result.values.supervision() {
        MeasureSupervision::Known { statement, .. } => statement.clone(),
        MeasureSupervision::Unknown { reason } => reason.clone(),
    };
    let command = MeasureAdministrativeCommand {
        operation_id: MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(3000)),
        target: reference_v2(prior),
        context: expectation(&group.review.material.context),
        reason: note("Administrative record after the later judicial capture"),
        action: if mark {
            MeasureAdministrativeAction::MarkEnteredInError
        } else {
            MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
                note("Correct the later judicial transcription"),
                prior.result.values.validity().clone(),
                supervision,
            ))
        },
    };
    let capture = prepare_measure_administrative_record_with_decision_history(
        &Hasher,
        &group.review.actor,
        group.review.case_id,
        command,
        group.review.material.context.clone(),
        &history,
    )
    .unwrap()
    .into_capture(&Hasher, group.recorded_at + Duration::seconds(1))
    .unwrap();
    (capture, history)
}

#[test]
fn two_corrections_then_v2_then_correction_keep_the_actual_latest_judicial_family() {
    let first_fixture = RecordFixture::initial();
    let first = first_fixture.capture();
    let second_fixture = RecordFixture::next(&first, &first_fixture.history, 1);
    let second = second_fixture.capture();
    let judicial_fixture = FixtureV2::confirm(&second, &second_fixture.history);
    let judicial = judicial_fixture.capture();
    assert_eq!(judicial.measures[0].result.revision.get(), 4);
    let (third, mut history) = correction_after_v2(&judicial, &judicial_fixture.history, false);
    let result = &third.records[0].result;
    assert_eq!(result.revision.get(), 5);
    assert_eq!(result.previous, reference_v2(&judicial.measures[0]));
    assert_eq!(result.last_action, MeasureCaptureAction::Confirm);
    assert_eq!(result.record_root, second.review.result.record_root);
    assert_eq!(result.judicial_origin, second.review.result.judicial_origin);
    assert_eq!(
        result.last_judicial.reference,
        reference_v2(&judicial.measures[0])
    );
    assert_eq!(
        result.last_judicial.owner.group_digest,
        judicial.capture_digest
    );
    assert_eq!(third.review.support, judicial.decision.support);
    measure_administrative_capture_with_decision_history_matches(&Hasher, &third, &history)
        .unwrap();
    let origin =
        measure_administrative_origin_with_decision_history(&Hasher, &third, &history).unwrap();
    history
        .records
        .administrative
        .push(MeasureAdministrativeEvidence {
            origin,
            capture: third.clone(),
        });
    history.records.administrative.reverse();
    history.decisions.reverse();
    let selected = record_reference(&third.records[0]);
    let checked = resolve_measure_records_with_decision_history(
        &Hasher,
        first_fixture.case_id,
        &[selected],
        &history,
    )
    .unwrap();
    let record = &checked.targets()[0];
    assert_eq!(record.reference(), selected);
    assert_eq!(record.values(), &third.review.result.values);
    assert_eq!(record.recorded_at(), third.recorded_at);
    let OwnedJudicialMeasure::V2(actual) = record.last_judicial() else {
        panic!("actual V2 judicial record expected")
    };
    assert_eq!(actual.capture, judicial.measures[0]);
    assert_ne!(record.values(), &actual.capture.result.values);
}

#[test]
fn successive_v2_decisions_resolve_by_exact_family_and_preserve_first_creation_ids() {
    let correction = RecordFixture::initial();
    let administrative = correction.capture();
    let first_fixture = FixtureV2::confirm(&administrative, &correction.history);
    let first = first_fixture.capture();
    let second_fixture = FixtureV2::next(&first, &first_fixture.history, 2);
    let second = second_fixture.capture();
    let mut history = append_v2(&second_fixture.history, &second);
    history.decisions.reverse();
    let checked = resolve_measure_records_with_decision_history(
        &Hasher,
        first_fixture.case_id,
        &[reference_v2(&second.measures[0])],
        &history,
    )
    .unwrap();
    let record = &checked.targets()[0];
    assert_eq!(record.reference().revision().get(), 4);
    assert_eq!(
        record.judicial_origin(),
        administrative.review.result.judicial_origin
    );
    assert_eq!(
        record.record_root(),
        administrative.review.result.record_root
    );
    assert_eq!(record.record(), &owned_v2(&second, 0));
    assert_eq!(record.last_judicial().reference(), record.reference());
    assert_eq!(record.last_judicial().actor(), &second.review.actor);
    assert_eq!(record.last_judicial().recorded_at(), second.recorded_at);
}

#[test]
fn marking_a_v2_record_retains_the_real_judicial_capture_and_remains_readable() {
    let fixture = FixtureV2::initial(crate::measure_decision_fixtures::Fixture::single());
    let judicial = fixture.capture();
    let (marked, mut history) = correction_after_v2(&judicial, &fixture.history, true);
    let origin =
        measure_administrative_origin_with_decision_history(&Hasher, &marked, &history).unwrap();
    history
        .records
        .administrative
        .push(MeasureAdministrativeEvidence {
            origin,
            capture: marked.clone(),
        });
    let checked = resolve_measure_records_with_decision_history(
        &Hasher,
        fixture.case_id,
        &[record_reference(&marked.records[0])],
        &history,
    )
    .unwrap();
    let record = &checked.targets()[0];
    assert_eq!(record.validity(), MeasureCaptureValidity::EnteredInError);
    assert_eq!(record.values(), &judicial.measures[0].result.values);
    assert_eq!(
        record.last_judicial().reference(),
        reference_v2(&judicial.measures[0])
    );
    assert!(matches!(
        record.last_judicial(),
        OwnedJudicialMeasure::V2(_)
    ));
}

#[test]
fn terminal_v2_records_allow_administrative_correction_without_changing_the_action() {
    let correction = RecordFixture::initial();
    let administrative = correction.capture();
    let mut fixture = FixtureV2::confirm(&administrative, &correction.history);
    fixture.effects(vec![MeasureEffect::Revoke {
        previous: record_reference(&administrative.records[0]),
    }]);
    let judicial = fixture.capture();
    let (corrected, history) = correction_after_v2(&judicial, &fixture.history, false);
    assert_eq!(
        corrected.review.result.last_action,
        MeasureCaptureAction::Revoke
    );
    assert!(corrected.review.result.values.validity().end().is_none());
    assert_eq!(
        corrected.review.result.last_judicial.reference,
        reference_v2(&judicial.measures[0])
    );
    measure_administrative_capture_with_decision_history_matches(&Hasher, &corrected, &history)
        .unwrap();
}

#[test]
fn legacy_resolvers_keep_old_exact_records_and_do_not_accept_a_v2_owner_as_v1() {
    let correction = RecordFixture::initial();
    let administrative = correction.capture();
    let fixture = FixtureV2::confirm(&administrative, &correction.history);
    let judicial = fixture.capture();
    let selected = record_reference(&administrative.records[0]);
    let legacy = resolve_measure_records(
        &Hasher,
        fixture.case_id,
        &[selected],
        &fixture.history.records,
    )
    .unwrap();
    let unified = resolve_measure_records_with_decision_history(
        &Hasher,
        fixture.case_id,
        &[selected],
        &fixture.history,
    )
    .unwrap();
    assert_eq!(legacy.targets()[0].record(), unified.targets()[0].record());
    assert!(resolve_measure_records(
        &Hasher,
        fixture.case_id,
        &[reference_v2(&judicial.measures[0])],
        &fixture.history.records
    )
    .is_err());
    let (corrected, history) = correction_after_v2(&judicial, &fixture.history, false);
    assert!(measure_administrative_capture_with_history_matches(
        &Hasher,
        &corrected,
        &history.records
    )
    .is_err());
}
