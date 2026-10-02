use super::{rejected, storage};
use application::ApplicationError;
use postgres::Transaction;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) fn lock_pending(transaction: &mut Transaction<'_>) -> Result<u64, ApplicationError> {
    let mut after: Option<Uuid> = None;
    let mut count = 0_u64;
    loop {
        let rows = transaction
            .query(
                "SELECT id FROM password_reset_capabilities
             WHERE consumed_at IS NULL AND cancelled_at IS NULL
             AND ($1::pg_catalog.uuid IS NULL OR id>$1) ORDER BY id LIMIT 128 FOR UPDATE",
                &[&after],
            )
            .map_err(|_| storage())?;
        if rows.is_empty() {
            return Ok(count);
        }
        count = count
            .checked_add(u64::try_from(rows.len()).map_err(|_| rejected())?)
            .ok_or_else(rejected)?;
        after = rows.last().map(|row| row.get(0));
    }
}

pub(super) fn cancel(
    transaction: &mut Transaction<'_>,
    pending: u64,
) -> Result<OffsetDateTime, ApplicationError> {
    // Acquire every pending row before sampling time; a waiting snapshot is stale.
    let micros: i64 = transaction
        .query_one(
            "SELECT (extract(epoch FROM pg_catalog.clock_timestamp())*1000000)::pg_catalog.int8",
            &[],
        )
        .map_err(|_| storage())?
        .get(0);
    let at = OffsetDateTime::from_unix_timestamp_nanos(i128::from(micros) * 1000)
        .map_err(|_| rejected())?;
    if !(1..=9999).contains(&at.year()) {
        return Err(rejected());
    }
    let timestamp = at.format(&Rfc3339).map_err(|_| rejected())?;
    let future: bool = transaction
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM password_reset_capabilities
         WHERE consumed_at IS NULL AND cancelled_at IS NULL
         AND issued_at>$1::pg_catalog.text::pg_catalog.timestamptz)",
            &[&timestamp],
        )
        .map_err(|_| storage())?
        .get(0);
    if future {
        return Err(rejected());
    }
    let changed = transaction.execute(
        "UPDATE password_reset_capabilities SET cancelled_at=$1::pg_catalog.text::pg_catalog.timestamptz
         WHERE consumed_at IS NULL AND cancelled_at IS NULL", &[&timestamp],
    ).map_err(|_| storage())?;
    let remains: bool = transaction
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM password_reset_capabilities
         WHERE consumed_at IS NULL AND cancelled_at IS NULL)",
            &[],
        )
        .map_err(|_| storage())?
        .get(0);
    if changed != pending || remains {
        return Err(rejected());
    }
    Ok(at)
}
