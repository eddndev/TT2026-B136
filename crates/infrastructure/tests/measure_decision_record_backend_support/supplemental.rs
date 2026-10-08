use super::*;
pub(super) use crate::administrative_fixture::{correction, mark, snapshot};
pub(super) use domain::precautionary_hearings::MeasureId;

#[path = "administrative.rs"]
mod administrative;
#[path = "anchors.rs"]
mod anchors;
#[path = "effects.rs"]
mod effects;
#[path = "integrity.rs"]
mod integrity;
#[path = "replay.rs"]
mod replay;
#[path = "schema.rs"]
mod schema;

fn same_operation(
    actual: &MeasureDecisionRecordStoredOperation,
    expected: &MeasureDecisionRecordStoredOperation,
) {
    let mut actual = actual.clone();
    let mut expected = expected.clone();
    for operation in [&mut actual, &mut expected] {
        let history = &mut operation.record_history;
        history
            .records
            .judicial
            .groups
            .sort_by_key(|g| g.origin.operation_id.as_uuid());
        history
            .records
            .administrative
            .sort_by_key(|a| a.origin.operation_id.as_uuid());
        history
            .decisions
            .sort_by_key(|g| g.origin.operation_id.as_uuid());
    }
    assert_eq!(actual, expected);
}

fn reopened(db: &Fixture, actor: &Principal, expected: &MeasureDecisionRecordStoredOperation) {
    let workflow = service_with_format(
        db,
        actor.clone(),
        FormatCheck(Some(Box::new(|| {
            panic!("original V2 replay must skip support admission")
        }))),
    );
    let command = expected.group.review.command.clone();
    assert_eq!(
        workflow
            .prepare("session", db.case, command.clone())
            .unwrap(),
        MeasureDecisionRecordReview::V2(Box::new(expected.group.review.clone()))
    );
    let MeasureDecisionRecordReceipt::V2(actual) = workflow
        .submit(
            "session",
            db.case,
            command,
            confirmation(&expected.group.review),
        )
        .unwrap()
    else {
        panic!("replay must preserve the V2 family")
    };
    same_operation(&actual, expected);
    assert_eq!(
        measure_decision_group_v2_bytes(&actual.group).unwrap(),
        measure_decision_group_v2_bytes(&expected.group).unwrap()
    );
}

fn changed_values(previous: &MeasureValues, conditions: &str) -> MeasureValues {
    MeasureValues::new(MeasureValuesInput {
        subject: previous.subject(),
        kind: previous.kind(),
        conditions: note(conditions),
        validity: previous.validity().clone(),
        supervision: previous.supervision().clone(),
    })
}
