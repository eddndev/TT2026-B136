use super::{
    authorization, codec, invalidate, plan, port, preferences, schedule_rows, schedule_selection,
    stored, subject, PostgresAlertStore,
};
use application::{alerts::*, ApplicationError};
use domain::identity::UserId;
use postgres::Transaction;
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) fn run(
    store: &PostgresAlertStore,
    tx: &mut Transaction<'_>,
    mut selection: schedule_selection::Selection,
    now: OffsetDateTime,
) -> Result<AlertSchedulerRun, ApplicationError> {
    let hasher = store.hasher.as_ref();
    let (kind, id) = codec::subject_key(selection.subject);
    let mut row = tx
        .query_opt(
            "SELECT * FROM alert_subject_state WHERE kind=$1 AND id=$2",
            &[&kind, &id],
        )
        .map_err(port)?;
    if row.is_none() {
        invalidate::invalidate(tx, selection.subject, hasher)?;
        row = tx
            .query_opt(
                "SELECT * FROM alert_subject_state WHERE kind=$1 AND id=$2",
                &[&kind, &id],
            )
            .map_err(port)?;
    }
    let row = row.ok_or_else(|| stored("alert subject state disappeared"))?;
    if row.try_get::<_, Uuid>("case_id").map_err(stored)? != selection.subject.case_id().as_uuid() {
        return Err(stored("alert subject case differs"));
    }
    let generation: i64 = row.try_get("generation").map_err(stored)?;
    let previous = codec::read(&row, hasher)?;
    let current = subject::load(tx, selection.subject, hasher, now)?;
    let state = plan::state(&previous, &current, now)?;
    if state != previous {
        selection.after_recipient = None;
        let (bytes, digest) = codec::payload(&state, hasher)?;
        tx.execute(
            "UPDATE alert_subject_state SET payload=$3,payload_digest=$4 WHERE kind=$1 AND id=$2",
            &[&kind, &id, &bytes, &digest],
        )
        .map_err(port)?;
    }
    let recipients = recipients(tx, &current, selection.after_recipient)?;
    let has_more = recipients.len() > 32;
    let mut scheduled = 0;
    let mut superseded = 0;
    let mut last = None;
    for user in recipients.into_iter().take(32) {
        last = Some(user.as_uuid());
        if authorization::recipient(tx, current.subject, user, current.responsible)?.is_none() {
            continue;
        }
        let prefs = preferences::load(tx, user, hasher, store.transport())?;
        let proposals = plan::plans(&current, &state, &prefs.values, now)?;
        let counts = reconcile(tx, store, &current, user, generation, proposals)?;
        scheduled += counts.0;
        superseded += counts.1;
    }
    schedule_selection::advance(tx, selection.subject, if has_more { last } else { None })?;
    Ok(AlertSchedulerRun::Reconciled {
        subject: selection.subject,
        scheduled,
        superseded,
    })
}

fn recipients(
    tx: &mut Transaction<'_>,
    current: &subject::Verified,
    after: Option<Uuid>,
) -> Result<Vec<UserId>, ApplicationError> {
    if matches!(current.subject, AlertSubject::Deadline { .. }) {
        return Ok(current
            .responsible
            .filter(|user| after.is_none_or(|id| user.as_uuid() > id))
            .into_iter()
            .collect());
    }
    tx.query(
        "SELECT m.user_id FROM case_memberships m JOIN users u ON u.id=m.user_id
        WHERE m.case_id=$1 AND u.active AND u.role IN ('owner','litigator','paralegal')
        AND ($2::uuid IS NULL OR m.user_id>$2) ORDER BY m.user_id LIMIT 33",
        &[&current.subject.case_id().as_uuid(), &after],
    )
    .map_err(port)?
    .iter()
    .map(|row| {
        row.try_get("user_id")
            .map(UserId::from_uuid)
            .map_err(stored)
    })
    .collect()
}

fn reconcile(
    tx: &mut Transaction<'_>,
    store: &PostgresAlertStore,
    current: &subject::Verified,
    recipient: UserId,
    generation: i64,
    mut proposals: Vec<plan::Plan>,
) -> Result<(u32, u32), ApplicationError> {
    let hasher = store.hasher.as_ref();
    for proposal in &mut proposals {
        proposal.superseded |=
            !proposal.channels.internal && (!proposal.channels.email || store.email.is_none());
    }
    let (kind, id) = codec::subject_key(current.subject);
    let existing = tx
        .query(
            "SELECT * FROM alert_schedule WHERE kind=$1 AND subject_id=$2
        AND recipient=$3 AND status='planned' ORDER BY id LIMIT 65",
            &[&kind, &id, &recipient.as_uuid()],
        )
        .map_err(port)?;
    if existing.len() > 64 {
        return Err(stored("too many unfinished alert plans for one recipient"));
    }
    let mut superseded = 0;
    for row in existing {
        let saved = schedule_rows::decode(&row, hasher)?;
        schedule_rows::verify_origin(tx, &saved, hasher)?;
        if !proposals
            .iter()
            .any(|plan| plan.key == saved.plan.key && !plan.superseded)
        {
            superseded += tx.execute("UPDATE alert_schedule SET status='superseded' WHERE id=$1 AND status='planned'", &[&saved.id]).map_err(port)? as u32;
        }
    }
    let mut scheduled = 0;
    for mut proposal in proposals {
        let previous = tx
            .query_opt(
                "SELECT * FROM alert_schedule WHERE recipient=$1 AND occurrence_key=$2",
                &[&recipient.as_uuid(), &proposal.key],
            )
            .map_err(port)?;
        if let Some(row) = previous {
            let saved = schedule_rows::decode(&row, hasher)?;
            schedule_rows::verify_origin(tx, &saved, hasher)?;
            if saved.status == "activated" || (saved.status == "superseded" && proposal.superseded)
            {
                continue;
            }
            proposal.occurrence = saved.plan.occurrence;
            proposal.trigger = saved.plan.trigger;
            let (bytes, digest) =
                codec::payload(&plan::encode(&proposal, current, recipient), hasher)?;
            let status = if proposal.superseded {
                "superseded"
            } else {
                "planned"
            };
            tx.execute("UPDATE alert_schedule SET generation=$2,status=$3,payload=$4,payload_digest=$5 WHERE id=$1", &[&saved.id,&generation,&status,&bytes,&digest]).map_err(port)?;
        } else {
            let (bytes, digest) =
                codec::payload(&plan::encode(&proposal, current, recipient), hasher)?;
            let status = if proposal.superseded {
                "superseded"
            } else {
                "planned"
            };
            tx.execute("INSERT INTO alert_schedule(id,kind,subject_id,case_id,recipient,occurrence_key,
                occurrence_id,trigger_seconds,trigger_nanos,status,generation,payload,payload_digest)
                VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
                &[&Uuid::new_v4(),&kind,&id,&current.subject.case_id().as_uuid(),&recipient.as_uuid(),
                  &proposal.key,&proposal.occurrence,&proposal.trigger.unix_timestamp(),&(proposal.trigger.nanosecond() as i32),
                  &status,&generation,&bytes,&digest]).map_err(port)?;
            if proposal.superseded {
                superseded += 1;
            } else {
                scheduled += 1;
            }
        }
    }
    Ok((scheduled, superseded))
}
