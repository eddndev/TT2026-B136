use super::{codec, plan, port, stored, subject};
use application::{
    alerts::AlertSubject,
    deadlines::{DeadlineAction, DeadlineAttention, DeadlineDetail},
    ApplicationError,
};
use domain::crypto::DocumentHasher;
use postgres::Transaction;
use serde_json::json;

pub(crate) fn invalidate(
    tx: &mut Transaction<'_>,
    subject: AlertSubject,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let (kind, id) = codec::subject_key(subject);
    let (bytes, digest) = codec::payload(&initial(), hasher)?;
    tx.execute(
        "INSERT INTO alert_subject_state(kind,id,case_id,generation,dirty,payload,payload_digest)
        VALUES($1,$2,$3,1,true,$4,$5) ON CONFLICT(kind,id) DO UPDATE
        SET generation=alert_subject_state.generation+1,dirty=true",
        &[&kind, &id, &subject.case_id().as_uuid(), &bytes, &digest],
    )
    .map_err(port)?;
    Ok(())
}

pub(crate) fn deadline(
    tx: &mut Transaction<'_>,
    value: &DeadlineDetail,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let subject = AlertSubject::Deadline {
        case_id: value.case_id,
        id: value.id,
    };
    invalidate(tx, subject, hasher)?;
    let accepted = value.receipt.action == DeadlineAction::Correct
        && value.review_state() == application::deadline_tracking::DeadlineReviewState::Accepted;
    if accepted || matches!(value.attention, DeadlineAttention::Recorded { .. }) {
        // Preserve the end of an episode even if another human mutation arrives
        // before the scheduler next observes this subject.
        let row = tx
            .query_opt(
                "SELECT * FROM alert_subject_state WHERE kind=1 AND id=$1",
                &[&value.id.as_uuid()],
            )
            .map_err(port)?
            .ok_or_else(|| stored("alert deadline state absent"))?;
        let mut payload = codec::read(&row, hasher)?;
        if accepted {
            let current = subject::load(tx, subject, hasher, value.recorded_at)?;
            payload = plan::state(&payload, &current, value.recorded_at)?;
        }
        if matches!(value.attention, DeadlineAttention::Recorded { .. }) {
            payload["overdue_episode"] = serde_json::Value::Null;
        }
        let (bytes, digest) = codec::payload(&payload, hasher)?;
        tx.execute(
            "UPDATE alert_subject_state SET payload=$2,payload_digest=$3 WHERE kind=1 AND id=$1",
            &[&value.id.as_uuid(), &bytes, &digest],
        )
        .map_err(port)?;
    }
    Ok(())
}
pub(super) fn initial() -> serde_json::Value {
    json!({"snapshot":null,"last_due":null,"review_episode":null,"overdue_episode":null,"changed_episode":null})
}
