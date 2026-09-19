use super::super::{codec, delivery, port, records, schedule_rows, stored, subject};
use application::{alerts::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::{Row, Transaction};
use uuid::Uuid;

pub(super) fn schedule(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let scheduled = schedule_rows::decode(row, hasher)?;
    schedule_rows::verify_origin(tx, &scheduled, hasher)?;
    let preview = schedule_rows::record(&scheduled, scheduled.plan.trigger, false);
    preview.validate(preview.recipient_id, preview.created_at)?;
    let (kind, id) = codec::subject_key(scheduled.subject);
    let suffix = match scheduled.plan.kind {
        AlertKind::Upcoming {
            lead_hours,
            activity_at,
        } => format!(
            "upcoming:{}:{}:{}",
            activity_at.unix_timestamp(),
            activity_at.nanosecond(),
            lead_hours.get()
        ),
        AlertKind::OverdueUnattended { .. } => format!("overdue:{}", scheduled.plan.occurrence),
        AlertKind::ReviewRequired => format!("review:{}", scheduled.plan.occurrence),
        AlertKind::DueChangedSoon { .. } => format!("changed:{}", scheduled.plan.occurrence),
    };
    if scheduled.plan.key != format!("v1:{kind}:{id}:{suffix}") {
        return Err(stored("schedule occurrence key differs"));
    }
    let state = tx
        .query_opt(
            "SELECT generation,case_id FROM alert_subject_state WHERE kind=$1 AND id=$2",
            &[&kind, &id],
        )
        .map_err(port)?
        .ok_or_else(|| stored("schedule state absent"))?;
    if row.try_get::<_, i64>("generation").map_err(stored)?
        > state.try_get::<_, i64>("generation").map_err(stored)?
        || state.try_get::<_, Uuid>("case_id").map_err(stored)?
            != scheduled.subject.case_id().as_uuid()
    {
        return Err(stored("schedule generation differs"));
    }
    let notification = tx
        .query_opt(
            "SELECT id FROM alert_notifications WHERE schedule_id=$1",
            &[&scheduled.id],
        )
        .map_err(port)?;
    if notification.is_some() != (scheduled.status == "activated") {
        return Err(stored("schedule activation receipt differs"));
    }
    Ok(())
}

pub(super) fn notification(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let mut record = records::decode(row, hasher)?;
    subject::verify_origin(tx, &record, hasher)?;
    let schedule_id: Uuid = row.try_get("schedule_id").map_err(stored)?;
    let plan_row = tx
        .query_opt("SELECT * FROM alert_schedule WHERE id=$1", &[&schedule_id])
        .map_err(port)?
        .ok_or_else(|| stored("notification plan absent"))?;
    let scheduled = schedule_rows::decode(&plan_row, hasher)?;
    let email = record.email == AlertEmailStatus::Pending;
    let mut expected = schedule_rows::record(&scheduled, record.created_at, email);
    expected.id = record.id;
    if scheduled.status != "activated"
        || records::encode(&expected) != records::encode(&record)
        || row.try_get::<_, bool>("internal_enabled").map_err(stored)?
            != scheduled.plan.channels.internal
        || email && !scheduled.plan.channels.email
    {
        return Err(stored("notification differs from activated plan"));
    }
    record.email = delivery::status(tx, record.id, hasher)?;
    let mut checked = record.created_at;
    if let Some(at) = record.read_at {
        checked = checked.max(at);
    }
    if let AlertState::Resolved { at, .. } = record.state {
        checked = checked.max(at);
    }
    if let AlertEmailStatus::Accepted { accepted_at } = record.email {
        checked = checked.max(accepted_at);
    }
    record.validate(record.recipient_id, checked)?;
    if record.read_at.is_some()
        && tx
            .query_opt(
                "SELECT operation_id FROM alert_read_receipts WHERE alert_id=$1 LIMIT 1",
                &[&record.id.as_uuid()],
            )
            .map_err(port)?
            .is_none()
    {
        return Err(stored("notification read has no receipt"));
    }
    Ok(())
}

pub(super) fn read_receipt(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let id = AlertId::from_uuid(row.try_get("alert_id").map_err(stored)?);
    let recipient: Uuid = row.try_get("recipient").map_err(stored)?;
    let at = codec::row_time(row, "read_seconds", "read_nanos")?
        .ok_or_else(|| stored("read receipt timestamp absent"))?;
    let record = records::load(tx, id, hasher)?;
    if recipient != record.recipient_id.as_uuid()
        || record.read_at != Some(at)
        || at < record.created_at
    {
        return Err(stored("notification read receipt differs"));
    }
    Ok(())
}
