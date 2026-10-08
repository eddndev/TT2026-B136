use super::*;

pub(super) fn alternating() -> FixtureV2 {
    let (first, prior) = corrected();
    let initial = FixtureV2::confirm(&prior, &first.history);
    let group = initial.capture();
    let mut history = append_v2(&initial.history, &group);
    let values = &group.measures[0].result.values;
    let supervision = match values.supervision() {
        MeasureSupervision::Known { statement, .. } => statement,
        MeasureSupervision::Unknown { reason } => reason,
    };
    let command = MeasureAdministrativeCommand {
        operation_id: MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(600)),
        target: reference_v2(&group.measures[0]),
        context: expectation(&group.review.material.context),
        reason: note("Correct the latest judicial transcription"),
        action: MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
            note("Corrected after the next judicial decision"),
            values.validity().clone(),
            supervision.clone(),
        )),
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
    let origin =
        measure_administrative_origin_with_decision_history(&Hasher, &capture, &history).unwrap();
    history
        .records
        .administrative
        .push(MeasureAdministrativeEvidence {
            origin,
            capture: capture.clone(),
        });
    let mut next = initial;
    next.identities(2);
    next.material.context = capture.review.context.clone();
    next.command.context = expectation(&next.material.context);
    effects(
        &mut next,
        vec![MeasureEffect::Confirm {
            previous: record_reference(&capture.records[0]),
        }],
    );
    next.material.predecessors = vec![OwnedMeasureRecord::Administrative {
        owner: MeasureAdministrativeRef {
            operation_id: capture.review.command.operation_id,
            capture_digest: capture.capture_digest,
        },
        capture: Box::new(capture.records[0].clone()),
    }];
    next.material.result_sources = vec![MeasureResultSources {
        id: capture.review.result.id,
        sources: capture.review.result.sources.clone(),
    }];
    next.history = history;
    next.recorded_at = capture.recorded_at + Duration::seconds(1);
    next
}
