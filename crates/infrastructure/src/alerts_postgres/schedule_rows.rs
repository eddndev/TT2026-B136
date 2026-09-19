use super::{codec, plan, port, preferences, records, stored, subject};
use application::{alerts::*, ApplicationError};
use domain::{crypto::DocumentHasher, identity::UserId};
use postgres::{Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) struct Scheduled {
    pub id: Uuid,
    pub subject: AlertSubject,
    pub recipient: UserId,
    pub plan: plan::Plan,
    pub origin: AlertOrigin,
    pub subject_title: String,
    pub case_title: String,
    pub case_reference: String,
    pub status: String,
}

pub(super) fn decode(
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<Scheduled, ApplicationError> {
    let value = codec::read(row, hasher)?;
    let subject = codec::read_subject(&value["subject"])?;
    let recipient = UserId::from_uuid(codec::uuid(&value["recipient"])?);
    let plan = plan::Plan {
        key: codec::text(&value["key"])?,
        occurrence: codec::uuid(&value["occurrence"])?,
        kind: codec::read_kind(&value["kind"])?,
        trigger: codec::time(&value["trigger"])?,
        channels: preferences::read_channels(&value["channels"])?,
        superseded: false,
    };
    let origin = codec::read_origin(&value["origin"])?;
    let current = subject::Verified {
        subject,
        origin,
        activity_at: None,
        attention_pending: false,
        review: false,
        ended: None,
        responsible: None,
        subject_title: codec::text(&value["subject_title"])?,
        case_title: codec::text(&value["case_title"])?,
        case_reference: codec::text(&value["case_reference"])?,
    };
    let (kind, id) = codec::subject_key(subject);
    if plan::encode(&plan, &current, recipient) != value
        || row.try_get::<_, i16>("kind").map_err(stored)? != kind
        || row.try_get::<_, Uuid>("subject_id").map_err(stored)? != id
        || row.try_get::<_, Uuid>("case_id").map_err(stored)? != subject.case_id().as_uuid()
        || row.try_get::<_, Uuid>("recipient").map_err(stored)? != recipient.as_uuid()
        || row.try_get::<_, Uuid>("occurrence_id").map_err(stored)? != plan.occurrence
        || row.try_get::<_, String>("occurrence_key").map_err(stored)? != plan.key
        || codec::row_time(row, "trigger_seconds", "trigger_nanos")? != Some(plan.trigger)
        || row.try_get::<_, i64>("generation").map_err(stored)? <= 0
    {
        return Err(stored("alert schedule projection differs"));
    }
    let status: String = row.try_get("status").map_err(stored)?;
    if !matches!(status.as_str(), "planned" | "activated" | "superseded") {
        return Err(stored("invalid alert schedule status"));
    }
    Ok(Scheduled {
        id: row.try_get("id").map_err(stored)?,
        subject,
        recipient,
        plan,
        origin,
        subject_title: current.subject_title,
        case_title: current.case_title,
        case_reference: current.case_reference,
        status,
    })
}

pub(super) fn record(plan: &Scheduled, now: OffsetDateTime, email: bool) -> AlertRecord {
    AlertRecord {
        id: AlertId::from_uuid(Uuid::new_v4()),
        recipient_id: plan.recipient,
        occurrence_id: AlertOccurrenceId::from_uuid(plan.plan.occurrence),
        subject: plan.subject,
        kind: plan.plan.kind,
        origin: plan.origin,
        trigger_at: plan.plan.trigger,
        created_at: now,
        read_at: None,
        state: AlertState::Active,
        email: if email {
            AlertEmailStatus::Pending
        } else {
            AlertEmailStatus::Disabled
        },
        subject_title: plan.subject_title.clone(),
        case_title: plan.case_title.clone(),
        case_reference: plan.case_reference.clone(),
    }
}

pub(super) fn verify_origin(
    tx: &mut Transaction<'_>,
    row: &Scheduled,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    subject::verify_origin(tx, &record(row, row.plan.trigger, false), hasher)
}

pub(super) fn insert_record(
    tx: &mut Transaction<'_>,
    row: &Scheduled,
    record: &AlertRecord,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let (bytes, digest) = codec::payload(&records::encode(record), hasher)?;
    let (kind, subject_id) = codec::subject_key(record.subject);
    tx.execute(
        "INSERT INTO alert_notifications(id,schedule_id,recipient,kind,subject_id,case_id,
        created_seconds,created_nanos,internal_enabled,payload,payload_digest)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        &[
            &record.id.as_uuid(),
            &row.id,
            &record.recipient_id.as_uuid(),
            &kind,
            &subject_id,
            &record.subject.case_id().as_uuid(),
            &record.created_at.unix_timestamp(),
            &(record.created_at.nanosecond() as i32),
            &row.plan.channels.internal,
            &bytes,
            &digest,
        ],
    )
    .map_err(port)?;
    Ok(())
}
