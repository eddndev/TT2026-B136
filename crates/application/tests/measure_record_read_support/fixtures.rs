use super::*;

pub fn root_fixture(serial: u128) -> crate::measure_decision_fixtures::Fixture {
    let mut fixture = crate::measure_decision_fixtures::Fixture::single();
    fixture.command.operation_id =
        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(2000 + serial));
    fixture.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(1000 + serial));
    let measure = id(3000 + serial);
    let mut effects = fixture.command.outcome.changes().unwrap().to_vec();
    let MeasureEffect::Impose(proposal) = &mut effects[0] else {
        unreachable!()
    };
    proposal.id = measure;
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
    fixture.material.result_sources[0].id = measure;
    fixture
}
pub fn from_group(
    group: &MeasureDecisionGroupCapture,
    ancestors: &MeasureHistoryEvidence,
) -> MeasureRecordDetail {
    MeasureRecordDetail {
        case_id: group.review.case_id,
        reference: reference(&group.measures[0]),
        record: OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V1(Box::new(owned(group)))),
        record_history: MeasureDecisionRecordHistoryEvidence {
            records: MeasureRecordHistoryEvidence {
                judicial: crate::effect_support::append_history(ancestors, group),
                administrative: vec![],
            },
            decisions: vec![],
        },
    }
}
pub fn initial(serial: u128) -> MeasureRecordDetail {
    from_group(
        &root_fixture(serial).capture(),
        &crate::effect_support::empty_history(),
    )
}
pub fn administrative(
    previous: &MeasureRecordDetail,
    serial: u128,
    mark: bool,
) -> MeasureRecordDetail {
    let checked = resolve_measure_records_with_decision_history(
        &Hasher,
        previous.case_id,
        &[previous.reference],
        &previous.record_history,
    )
    .unwrap();
    let prior = &checked.targets()[0];
    let supervision = match prior.values().supervision() {
        MeasureSupervision::Known { statement, .. } => statement,
        MeasureSupervision::Unknown { reason } => reason,
    };
    let command = MeasureAdministrativeCommand {
        operation_id: MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(5000 + serial)),
        target: previous.reference,
        context: expectation(prior.context()),
        reason: note("Record a clerical correction or erroneous capture"),
        action: if mark {
            MeasureAdministrativeAction::MarkEnteredInError
        } else {
            MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
                note(&format!("Corrected conditions {serial}")),
                prior.values().validity().clone(),
                supervision.clone(),
            ))
        },
    };
    let actor = crate::measure_decision_fixtures::Fixture::single().actor;
    let captured = prepare_measure_administrative_record_with_decision_history(
        &Hasher,
        &actor,
        previous.case_id,
        command,
        prior.context().clone(),
        &previous.record_history,
    )
    .unwrap()
    .into_capture(&Hasher, prior.recorded_at() + Duration::seconds(1))
    .unwrap();
    let origin = measure_administrative_origin_with_decision_history(
        &Hasher,
        &captured,
        &previous.record_history,
    )
    .unwrap();
    let row = captured.records[0].clone();
    let record = OwnedMeasureRecord::Administrative {
        owner: MeasureAdministrativeRef {
            operation_id: origin.operation_id,
            capture_digest: captured.capture_digest,
        },
        capture: Box::new(row.clone()),
    };
    let mut record_history = previous.record_history.clone();
    record_history
        .records
        .administrative
        .push(MeasureAdministrativeEvidence {
            origin,
            capture: captured,
        });
    MeasureRecordDetail {
        case_id: previous.case_id,
        reference: record_reference(&row),
        record,
        record_history,
    }
}
pub fn mixed(serial: u128) -> Vec<MeasureRecordDetail> {
    let first = initial(serial);
    let correction = administrative(&first, serial, false);
    let a = &correction.record_history.records.administrative[0].capture;
    let mut judicial = FixtureV2::confirm(a, &first.record_history.records);
    judicial.identities(10_000 + serial);
    let group = judicial.capture();
    let third = MeasureRecordDetail {
        case_id: group.review.case_id,
        reference: reference_v2(&group.measures[0]),
        record: owned_v2(&group, 0),
        record_history: append_v2(&judicial.history, &group),
    };
    let fourth = administrative(&third, 100 + serial, false);
    vec![first, correction, third, fourth]
}
