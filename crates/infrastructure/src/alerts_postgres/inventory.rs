//! Read-only replay of persisted activity notification evidence at startup.
#[path = "inventory_records.rs"]
mod record_checks;
#[path = "inventory_state.rs"]
mod state_checks;
use super::{delivery, port, preferences, stored};
use application::{alerts::AlertEmailTransport, ApplicationError};
use postgres::{Client, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    preference_history(&mut tx)?;
    states(&mut tx)?;
    state_checks::cursor(&mut tx)?;
    for (table, key, kind) in [
        ("alert_schedule", "id", 0),
        ("alert_notifications", "id", 1),
        ("alert_read_receipts", "operation_id", 2),
    ] {
        records(&mut tx, table, key, kind)?;
    }
    delivery::validate(&mut tx, &crate::RingSha256Hasher)?;
    let orphan:bool=tx.query_one("SELECT
        EXISTS(SELECT 1 FROM alert_preferences p LEFT JOIN users u ON u.id=p.user_id WHERE u.id IS NULL) OR
        EXISTS(SELECT 1 FROM alert_schedule s LEFT JOIN users u ON u.id=s.recipient WHERE u.id IS NULL) OR
        EXISTS(SELECT 1 FROM alert_email_attempts a LEFT JOIN alert_email_outbox o ON o.id=a.delivery_id WHERE o.id IS NULL)",&[]).map_err(port)?.get(0);
    if orphan {
        return Err(stored("alert inventory has orphan evidence"));
    }
    tx.rollback().map_err(port)
}
fn preference_history(tx: &mut Transaction<'_>) -> Result<(), ApplicationError> {
    let mut after_user: Option<Uuid> = None;
    let mut after_revision = 0i64;
    let mut last: Option<(Uuid, u32, OffsetDateTime)> = None;
    loop {
        let rows=tx.query("SELECT * FROM alert_preferences WHERE $1::uuid IS NULL OR (user_id,revision)>($1,$2) ORDER BY user_id,revision LIMIT 64",&[&after_user,&after_revision]).map_err(port)?;
        for row in &rows {
            let saved =
                preferences::decode(row, &crate::RingSha256Hasher, AlertEmailTransport::Disabled)?;
            let user = saved.user_id.as_uuid();
            let at = saved
                .updated_at
                .ok_or_else(|| stored("persisted preference time absent"))?;
            let expected = match last {
                Some((previous, revision, time)) if previous == user => {
                    if at < time {
                        return Err(stored("preference history time regressed"));
                    }
                    revision
                        .checked_add(1)
                        .ok_or_else(|| stored("preference revision overflow"))?
                }
                _ => 1,
            };
            if saved.revision != expected {
                return Err(stored("preference history has a gap"));
            }
            last = Some((user, saved.revision, at));
            after_user = Some(user);
            after_revision = i64::from(saved.revision);
        }
        if rows.len() < 64 {
            return Ok(());
        }
    }
}
fn states(tx: &mut Transaction<'_>) -> Result<(), ApplicationError> {
    let mut kind = -1i16;
    let mut id = Uuid::nil();
    loop {
        let rows=tx.query("SELECT * FROM alert_subject_state WHERE (kind,id)>($1,$2) ORDER BY kind,id LIMIT 64",&[&kind,&id]).map_err(port)?;
        for row in &rows {
            state_checks::validate(tx, row, &crate::RingSha256Hasher)?;
            kind = row.try_get("kind").map_err(stored)?;
            id = row.try_get("id").map_err(stored)?;
        }
        if rows.len() < 64 {
            return Ok(());
        }
    }
}
fn records(
    tx: &mut Transaction<'_>,
    table: &str,
    key: &str,
    kind: u8,
) -> Result<(), ApplicationError> {
    // Identifiers come only from the fixed table list in validate.
    let query =
        format!("SELECT * FROM {table} WHERE $1::uuid IS NULL OR {key}>$1 ORDER BY {key} LIMIT 64");
    let mut after: Option<Uuid> = None;
    loop {
        let rows = tx.query(&query, &[&after]).map_err(port)?;
        for row in &rows {
            match kind {
                0 => record_checks::schedule(tx, row, &crate::RingSha256Hasher)?,
                1 => record_checks::notification(tx, row, &crate::RingSha256Hasher)?,
                _ => record_checks::read_receipt(tx, row, &crate::RingSha256Hasher)?,
            }
            after = Some(row.try_get(key).map_err(stored)?);
        }
        if rows.len() < 64 {
            return Ok(());
        }
    }
}
