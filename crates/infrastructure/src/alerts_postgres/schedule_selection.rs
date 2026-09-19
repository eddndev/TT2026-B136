use super::{codec, port, stored};
use application::{alerts::AlertSubject, ApplicationError};
use postgres::Transaction;
use serde_json::json;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub(super) struct Selection {
    pub subject: AlertSubject,
    pub after_recipient: Option<Uuid>,
}

pub(super) fn next(
    tx: &mut Transaction<'_>,
    now: OffsetDateTime,
) -> Result<Option<Selection>, ApplicationError> {
    let cursor = tx
        .query_opt("SELECT * FROM alert_scan_cursor WHERE singleton", &[])
        .map_err(port)?
        .ok_or_else(|| stored("missing alert scan cursor"))?;
    let active_kind: Option<i16> = cursor.try_get("active_kind").map_err(stored)?;
    let active_id: Option<Uuid> = cursor.try_get("active_id").map_err(stored)?;
    if let (Some(kind), Some(id)) = (active_kind, active_id) {
        let row = tx
            .query_opt(
                "SELECT case_id FROM alert_subject_state WHERE kind=$1 AND id=$2",
                &[&kind, &id],
            )
            .map_err(port)?
            .ok_or_else(|| stored("active alert scan subject is missing"))?;
        let case: Uuid = row.try_get("case_id").map_err(stored)?;
        return Ok(Some(Selection {
            subject: codec::read_subject(&json!([kind, case, id]))?,
            after_recipient: cursor.try_get("after_recipient").map_err(stored)?,
        }));
    }
    if active_kind.is_some() || active_id.is_some() {
        return Err(stored("partial active alert scan cursor"));
    }
    let dirty = tx
        .query_opt(
            "SELECT kind,id,case_id FROM alert_subject_state WHERE dirty ORDER BY kind,id LIMIT 1",
            &[],
        )
        .map_err(port)?;
    let row = if let Some(row) = dirty {
        row
    } else {
        if codec::row_time(&cursor, "next_seconds", "next_nanos")?.is_some_and(|at| at > now) {
            return Ok(None);
        }
        let kind: i16 = cursor.try_get("kind").map_err(stored)?;
        let id: Option<Uuid> = cursor.try_get("id").map_err(stored)?;
        let row = tx
            .query_opt(
                "SELECT * FROM (
            SELECT 0::smallint AS kind,id,case_id FROM case_hearings
            UNION ALL SELECT 1::smallint AS kind,id,case_id FROM case_deadlines) roots
            WHERE kind>$1 OR (kind=$1 AND ($2::uuid IS NULL OR id>$2))
            ORDER BY kind,id LIMIT 1",
                &[&kind, &id],
            )
            .map_err(port)?;
        let Some(row) = row else {
            if kind == 0 && id.is_none() {
                return Ok(None);
            }
            let next = now
                .checked_add(Duration::seconds(1))
                .ok_or_else(|| stored("alert scan clock overflow"))?;
            tx.execute("UPDATE alert_scan_cursor SET kind=0,id=NULL,cycle=cycle+1,next_seconds=$1,next_nanos=$2
                WHERE singleton", &[&next.unix_timestamp(), &(next.nanosecond() as i32)]).map_err(port)?;
            super::audit(
                tx,
                "system:activity-alerts",
                "alert.scan_completed",
                "alert-scan",
                now,
            )?;
            return Ok(None);
        };
        row
    };
    let kind: i16 = row.try_get("kind").map_err(stored)?;
    let id: Uuid = row.try_get("id").map_err(stored)?;
    let case: Uuid = row.try_get("case_id").map_err(stored)?;
    let subject = codec::read_subject(&json!([kind, case, id]))?;
    tx.execute("UPDATE alert_scan_cursor SET active_kind=$1,active_id=$2,after_recipient=NULL WHERE singleton", &[&kind,&id]).map_err(port)?;
    Ok(Some(Selection {
        subject,
        after_recipient: None,
    }))
}

pub(super) fn advance(
    tx: &mut Transaction<'_>,
    subject: AlertSubject,
    after: Option<Uuid>,
) -> Result<(), ApplicationError> {
    let (kind, id) = codec::subject_key(subject);
    if let Some(recipient) = after {
        tx.execute(
            "UPDATE alert_scan_cursor SET after_recipient=$1 WHERE singleton",
            &[&recipient],
        )
        .map_err(port)?;
    } else {
        tx.execute(
            "UPDATE alert_subject_state SET dirty=false WHERE kind=$1 AND id=$2",
            &[&kind, &id],
        )
        .map_err(port)?;
        tx.execute(
            "UPDATE alert_scan_cursor SET kind=$1,id=$2,active_kind=NULL,active_id=NULL,
            after_recipient=NULL,next_seconds=NULL,next_nanos=NULL WHERE singleton",
            &[&kind, &id],
        )
        .map_err(port)?;
    }
    Ok(())
}
