use super::{
    inconsistent,
    member_fields::MemberFields,
    write_common::{self, DecisionFields},
};
use application::{
    precautionary_measures::{
        measure_group_origin, MeasureDecisionGroupCapture, MeasureDecisionStoredOperation,
    },
    ApplicationError,
};
use domain::crypto::DocumentHasher;
use postgres::Transaction;

pub(super) fn marker(group: &MeasureDecisionGroupCapture) -> String {
    let review = &group.review;
    format!(
        "mg1:case:{}:operation:{}:decision:{}:submission:{}:review:{}:decision_digest:{}:group:{}",
        review.case_id,
        review.command.operation_id,
        review.command.decision_id,
        review.submission_digest.to_hex(),
        review.review_digest.to_hex(),
        group.decision.capture_digest.to_hex(),
        group.capture_digest.to_hex(),
    )
}

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    operation: &MeasureDecisionStoredOperation,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let group = &operation.group;
    let review = &group.review;
    if measure_group_origin(hasher, group, &operation.measure_history)? != operation.origin {
        return Err(inconsistent(
            "measure group differs from its original evidence",
        ));
    }
    write_common::decision(
        tx,
        DecisionFields {
            command: &review.command,
            case: review.case_id,
            context: &review.material.context,
            support: &review.material.support,
            actor: &review.actor,
            at: group.recorded_at,
            submission_digest: review.submission_digest,
            review_digest: review.review_digest,
            decision_digest: group.decision.capture_digest,
            group_digest: group.capture_digest,
            family: "g1",
            marker: marker(group),
        },
        hasher,
    )?;
    for capture in &group.measures {
        let result = &capture.result;
        let root = &result.origin;
        write_common::member(
            tx,
            MemberFields {
                id: result.id,
                revision: result.revision,
                case: capture.case_id,
                operation: capture.operation_id,
                family: "m1",
                action: result.action,
                values: &result.values,
                digest: capture.capture_digest,
                root_operation: root.operation_id,
            },
            result.previous.is_none(),
            hasher,
        )?;
    }
    Ok(())
}
