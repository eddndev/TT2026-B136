use super::{rejected, storage};
use application::ApplicationError;
use postgres::Transaction;
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) fn lock_pending(transaction: &mut Transaction<'_>) -> Result<u64, ApplicationError> {
    let mut after: Option<Uuid> = None;
    let mut count = 0_u64;
    loop {
        let rows = transaction
            .query(
                "SELECT id FROM member_invitations
             WHERE accepted_at IS NULL AND cancelled_at IS NULL
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
    // Sample the database clock only after all pending invitations are locked.
    let seconds: i64 = transaction
        .query_one(
            "SELECT floor(extract(epoch FROM pg_catalog.clock_timestamp()))::pg_catalog.int8",
            &[],
        )
        .map_err(|_| storage())?
        .get(0);
    let at = OffsetDateTime::from_unix_timestamp(seconds).map_err(|_| rejected())?;
    if seconds < 0 || !(1..=9999).contains(&at.year()) {
        return Err(rejected());
    }
    let future: bool = transaction
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM member_invitations
         WHERE accepted_at IS NULL AND cancelled_at IS NULL AND issued_at>$1)",
            &[&seconds],
        )
        .map_err(|_| storage())?
        .get(0);
    if future {
        return Err(rejected());
    }
    let changed = transaction
        .execute(
            "UPDATE member_invitations SET cancelled_at=$1, protected_token=NULL,
             pending_operation=NULL, pending_digest=NULL, pending_material=NULL,
             delivery_state='rejected', lease_id=NULL, lease_until=NULL,
             provider_deadline=NULL, retry_at=NULL
         WHERE accepted_at IS NULL AND cancelled_at IS NULL",
            &[&seconds],
        )
        .map_err(|_| storage())?;
    let remains: bool = transaction
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM member_invitations
         WHERE accepted_at IS NULL AND cancelled_at IS NULL)",
            &[],
        )
        .map_err(|_| storage())?
        .get(0);
    if changed != pending || remains {
        return Err(rejected());
    }
    Ok(at)
}
