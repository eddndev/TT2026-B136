use super::*;

pub fn judicial(original: &MeasureDecisionStoredOperation, index: usize) -> MeasureRecordDetail {
    let capture = original.group.measures[index].clone();
    let reference = crate::administrative_fixture::reference(&capture);
    let record =
        OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V1(Box::new(OwnedMeasureMaterial {
            owner: MeasureGroupRef {
                operation_id: original.origin.operation_id,
                decision_id: original.origin.decision_id,
                group_digest: original.group.capture_digest,
            },
            capture,
        })));
    let mut groups = original.measure_history.groups.clone();
    groups.push(MeasureGroupEvidence {
        origin: original.origin.clone(),
        capture: original.group.clone(),
    });
    MeasureRecordDetail {
        case_id: original.origin.case_id,
        reference,
        record,
        record_history: MeasureDecisionRecordHistoryEvidence {
            records: MeasureRecordHistoryEvidence {
                judicial: MeasureHistoryEvidence { groups },
                administrative: vec![],
            },
            decisions: vec![],
        },
    }
}
pub fn administrative(original: &MeasureAdministrativeStoredOperation) -> MeasureRecordDetail {
    let capture = original.capture.records[0].clone();
    let reference = crate::administrative_fixture::corrected_reference(&original.capture);
    let record = OwnedMeasureRecord::Administrative {
        owner: MeasureAdministrativeRef {
            operation_id: original.origin.operation_id,
            capture_digest: original.capture.capture_digest,
        },
        capture: Box::new(capture),
    };
    let mut record_history = original.record_history.clone();
    record_history
        .records
        .administrative
        .push(MeasureAdministrativeEvidence {
            origin: original.origin.clone(),
            capture: original.capture.clone(),
        });
    MeasureRecordDetail {
        case_id: original.origin.case_id,
        reference,
        record,
        record_history,
    }
}
pub fn same_detail(actual: &MeasureRecordDetail, expected: &MeasureRecordDetail) {
    let mut actual = actual.clone();
    let mut expected = expected.clone();
    for row in [&mut actual, &mut expected] {
        row.record_history
            .records
            .judicial
            .groups
            .sort_by_key(|g| g.origin.operation_id.as_uuid());
        row.record_history
            .records
            .administrative
            .sort_by_key(|a| a.origin.operation_id.as_uuid());
        row.record_history
            .decisions
            .sort_by_key(|g| g.origin.operation_id.as_uuid());
    }
    assert_eq!(actual, expected);
}
pub fn chain(
    db: &mut Fixture,
) -> (
    Seed,
    MeasureDecisionStoredOperation,
    MeasureAdministrativeStoredOperation,
    MeasureAdministrativeStoredOperation,
) {
    let (seed, judicial, command) = crate::administrative_fixture::setup(db);
    let first = crate::administrative_fixture::persist(db, seed.actor.clone(), command);
    let marked = crate::administrative_fixture::persist(
        db,
        seed.actor.clone(),
        crate::administrative_fixture::mark(
            crate::administrative_fixture::corrected_reference(&first.capture),
            seed.command.context,
        ),
    );
    (seed, judicial, first, marked)
}
