use super::{
    authorization, codec, delivery, invalidate, plan, port, preferences, schedule_rows, subject,
    PostgresAlertStore,
};
use application::{alerts::*, ApplicationError};
use domain::alerts::AlertWindow;
use postgres::Transaction;
use time::OffsetDateTime;

pub(super) fn next(
    store: &PostgresAlertStore,
    tx: &mut Transaction<'_>,
    now: OffsetDateTime,
) -> Result<Option<AlertSchedulerRun>, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT * FROM alert_schedule WHERE status='planned'
        AND (trigger_seconds,trigger_nanos)<=($1,$2)
        ORDER BY trigger_seconds,trigger_nanos,id LIMIT 1",
            &[&now.unix_timestamp(), &(now.nanosecond() as i32)],
        )
        .map_err(port)?;
    let Some(row) = row else { return Ok(None) };
    let hasher = store.hasher.as_ref();
    let mut saved = schedule_rows::decode(&row, hasher)?;
    schedule_rows::verify_origin(tx, &saved, hasher)?;
    let current = subject::load(tx, saved.subject, hasher, now)?;
    let prefs = preferences::load(tx, saved.recipient, hasher, store.transport())?;
    let channels = plan::channels(saved.plan.kind, &prefs.values, saved.subject);
    let eligible = subject::reason(saved.plan.kind, &current, now).is_none()
        && authorization::recipient(tx, saved.subject, saved.recipient, current.responsible)?
            .is_some()
        && (channels.internal || (channels.email && store.email.is_some()))
        && nearest(&saved, &prefs.values, now);
    if !eligible {
        tx.execute(
            "UPDATE alert_schedule SET status='superseded' WHERE id=$1",
            &[&saved.id],
        )
        .map_err(port)?;
        // A changed source can invalidate a date before reevaluation writes another revision.
        invalidate::invalidate(tx, saved.subject, hasher)?;
        return Ok(Some(AlertSchedulerRun::Reconciled {
            subject: saved.subject,
            scheduled: 0,
            superseded: 1,
        }));
    }
    saved.plan.channels = channels;
    saved.origin = current.origin;
    saved.subject_title = current.subject_title.clone();
    saved.case_title = current.case_title.clone();
    saved.case_reference = current.case_reference.clone();
    let (bytes, digest) = codec::payload(
        &plan::encode(&saved.plan, &current, saved.recipient),
        hasher,
    )?;
    tx.execute(
        "UPDATE alert_schedule SET payload=$2,payload_digest=$3,status='activated' WHERE id=$1",
        &[&saved.id, &bytes, &digest],
    )
    .map_err(port)?;
    let record = schedule_rows::record(&saved, now, channels.email && store.email.is_some());
    record.validate(saved.recipient, now)?;
    schedule_rows::insert_record(tx, &saved, &record, hasher)?;
    if record.email == AlertEmailStatus::Pending {
        delivery::enqueue(tx, record.id, now, hasher)?;
    }
    Ok(Some(AlertSchedulerRun::Activated {
        alert_id: record.id,
    }))
}

fn nearest(
    row: &schedule_rows::Scheduled,
    prefs: &AlertPreferenceValues,
    now: OffsetDateTime,
) -> bool {
    let AlertKind::Upcoming {
        lead_hours,
        activity_at,
    } = row.plan.kind
    else {
        return true;
    };
    let family = if matches!(row.subject, AlertSubject::Hearing { .. }) {
        &prefs.hearing_upcoming
    } else {
        &prefs.deadline_upcoming
    };
    family
        .anticipations
        .hours()
        .iter()
        .filter(|lead| {
            AlertWindow::upcoming(activity_at, **lead).is_ok_and(|window| window.contains(now))
        })
        .map(|lead| lead.get())
        .min()
        == Some(lead_hours.get())
}
