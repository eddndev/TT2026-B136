use super::super::{decode, inconsistent, port};
use application::ApplicationError;
use domain::{
    audit::{chain_digest, GENESIS_PREVIOUS},
    crypto::DocumentHasher,
};
use postgres::{Row, Transaction};
use time::OffsetDateTime;

pub(crate) fn time(row: &Row) -> Result<OffsetDateTime, ApplicationError> {
    let at = OffsetDateTime::from_unix_timestamp(row.get("recorded_at_seconds"))
        .map_err(inconsistent)?
        .replace_nanosecond(
            u32::try_from(row.get::<_, i32>("recorded_at_nanoseconds")).map_err(inconsistent)?,
        )
        .map_err(inconsistent)?;
    if !(1..=9999).contains(&at.year()) {
        return Err(inconsistent(
            "declaration timestamp exceeds supported years",
        ));
    }
    Ok(at)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn verify(
    tx: &mut Transaction<'_>,
    sequence: i64,
    actor: &str,
    action: &str,
    marker: &str,
    at: OffsetDateTime,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    if sequence < 0 {
        return Err(inconsistent("negative original audit sequence"));
    }
    let count: i64 = tx
        .query_one(
            "SELECT count(*) FROM (SELECT sequence FROM audit_events
         WHERE resource=$1 AND action=$2 LIMIT 2) matching",
            &[&marker, &action],
        )
        .map_err(port)?
        .get(0);
    if count != 1 {
        return Err(inconsistent(
            "original declaration audit is absent or duplicated",
        ));
    }
    let row = tx
        .query_opt(
            "SELECT sequence,timestamp,actor,action,resource,chain FROM audit_events
         WHERE sequence=$1 AND octet_length(timestamp) BETWEEN 1 AND 64
         AND octet_length(actor) BETWEEN 1 AND 1280 AND octet_length(action) BETWEEN 1 AND 64
         AND octet_length(resource) BETWEEN 1 AND 512 AND octet_length(chain)=32",
            &[&sequence],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("original audit is absent or oversized"))?;
    let timestamp: String = row.get("timestamp");
    let entry = crate::audit_postgres::decode_event(row).map_err(inconsistent)?;
    if entry.event.sequence != sequence as u64
        || entry.event.actor != actor
        || entry.event.action != action
        || entry.event.resource != marker
        || entry.event.timestamp != at
        || entry.event.timestamp.offset() != time::UtcOffset::UTC
        || entry.event.timestamp_rfc3339()? != timestamp
    {
        return Err(inconsistent(
            "original audit differs from the exact declaration",
        ));
    }
    let previous = if sequence == 0 {
        GENESIS_PREVIOUS
    } else {
        let row = tx
            .query_opt(
                "SELECT chain FROM audit_events WHERE sequence=$1 AND octet_length(chain)=32",
                &[&(sequence - 1)],
            )
            .map_err(port)?
            .ok_or_else(|| inconsistent("original audit predecessor is absent"))?;
        decode::digest(row.get("chain"))?
    };
    if chain_digest(hasher, &previous, &entry.event)? != entry.chain {
        return Err(inconsistent("original declaration audit chain differs"));
    }
    Ok(())
}
