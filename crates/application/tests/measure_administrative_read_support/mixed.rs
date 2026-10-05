use super::*;

pub fn next(
    previous: &MeasureAdministrativeStoredOperation,
    serial: u128,
) -> MeasureAdministrativeStoredOperation {
    let row = &previous.capture.records[0];
    let mut history = previous.record_history.clone();
    history
        .records
        .administrative
        .push(MeasureAdministrativeEvidence {
            origin: previous.origin.clone(),
            capture: previous.capture.clone(),
        });
    let supervision = match row.result.values.supervision() {
        MeasureSupervision::Known { statement, .. } => statement,
        MeasureSupervision::Unknown { reason } => reason,
    };
    let command = MeasureAdministrativeCommand {
        operation_id: operation_id(serial),
        target: record_reference(row),
        context: expectation(&previous.capture.review.context),
        reason: note("Correct another transcription"),
        action: MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
            note(&format!("Another correction {serial}")),
            row.result.values.validity().clone(),
            supervision.clone(),
        )),
    };
    capture(
        command,
        previous.capture.review.context.clone(),
        history,
        previous.capture.recorded_at + Duration::seconds(1),
    )
}

pub fn after_m2() -> MeasureAdministrativeStoredOperation {
    let first = operation(10);
    let mut judicial = FixtureV2::confirm(&first.capture, &first.record_history.records);
    judicial.identities(90);
    let group = judicial.capture();
    let row = &group.measures[0];
    let supervision = match row.result.values.supervision() {
        MeasureSupervision::Known { statement, .. } => statement,
        MeasureSupervision::Unknown { reason } => reason,
    };
    let command = MeasureAdministrativeCommand {
        operation_id: operation_id(20),
        target: reference_v2(row),
        context: expectation(&group.review.material.context),
        reason: note("Correct terms recorded after the judicial decision"),
        action: MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
            note("Corrected terms after M2"),
            row.result.values.validity().clone(),
            supervision.clone(),
        )),
    };
    capture(
        command,
        group.review.material.context.clone(),
        append_v2(&judicial.history, &group),
        group.recorded_at + Duration::seconds(1),
    )
}

#[test]
fn reads_preserve_complete_g1_c_g2_c_history_without_repacking_the_judicial_family() {
    let original = after_m2();
    assert_eq!(original.record_history.records.judicial.groups.len(), 1);
    assert_eq!(original.record_history.records.administrative.len(), 1);
    assert_eq!(original.record_history.decisions.len(), 1);
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let store = successful_store(&actor, &original, original.clone(), kind);
        assert_eq!(
            read(&service(store, identity(&actor)), kind, &original).unwrap(),
            vec![original.clone()]
        );
    }
}

#[test]
fn reads_validate_each_mixed_ancestor_family_and_preserve_original_closure_order() {
    let original = next(&after_m2(), 30);
    for mutation in 0..6 {
        let mut returned = original.clone();
        match mutation {
            0 => returned.record_history.records.administrative.clear(),
            1 => returned.record_history.decisions.clear(),
            2 => {
                returned.record_history.decisions[0].origin.group_digest =
                    Sha256Digest::from_array([99; 32])
            }
            3 => {
                returned.record_history.records.administrative[0]
                    .origin
                    .capture_digest = Sha256Digest::from_array([99; 32])
            }
            4 => returned.record_history.decisions[0]
                .capture
                .measures
                .clear(),
            _ => returned.record_history.records.administrative[0]
                .capture
                .records
                .clear(),
        }
        reject_reads(&original, &returned);
    }
    let mut reordered = original.clone();
    reordered.record_history.records.administrative.reverse();
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let store = successful_store(&actor, &original, reordered.clone(), kind);
        assert_eq!(
            read(&service(store, identity(&actor)), kind, &original).unwrap(),
            vec![reordered.clone()]
        );
    }
}

#[test]
fn older_valid_receipt_and_later_mark_remain_individually_readable_without_head_admission() {
    let first = operation(10);
    let mut mark = next(&first, 20);
    mark = capture(
        MeasureAdministrativeCommand {
            action: MeasureAdministrativeAction::MarkEnteredInError,
            ..mark.capture.review.command.clone()
        },
        mark.capture.review.context.clone(),
        mark.record_history.clone(),
        mark.capture.recorded_at,
    );
    assert_eq!(
        mark.capture.review.result.validity,
        MeasureCaptureValidity::EnteredInError
    );
    let expected = page(first.origin.case_id, vec![first, mark]);
    assert_eq!(
        list_result(MeasureAdministrativeReadQuery::default(), expected.clone()).unwrap(),
        expected
    );
}
