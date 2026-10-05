use super::{inconsistent, port};
use application::{hearings::*, ApplicationError};
use domain::{
    audit::{chain_digest, GENESIS_PREVIOUS},
    crypto::{DocumentHasher, Sha256Digest},
};
use postgres::Transaction;

pub(super) fn verify(
    tx: &mut Transaction<'_>,
    detail: &HearingDetail,
    hasher: &dyn DocumentHasher,
) -> Result<i64, ApplicationError> {
    let snapshot = &detail.snapshot;
    let marker = format!(
        "case:{}:hearing:{}:revision:{}:operation:{}:sha256:{}",
        snapshot.case_id,
        snapshot.id,
        snapshot.revision.get(),
        snapshot.receipt.operation_id,
        snapshot.receipt.submission_digest.to_hex()
    );
    let action = match snapshot.receipt.action {
        HearingAction::Schedule => "hearing.scheduled",
        HearingAction::Replace => "hearing.replaced",
        HearingAction::Cancel => "hearing.cancelled",
    };
    let matches = tx
        .query(
            "SELECT sequence FROM audit_events WHERE resource=$1
             AND action IN ('hearing.scheduled','hearing.replaced','hearing.cancelled')
             ORDER BY sequence LIMIT 2",
            &[&marker],
        )
        .map_err(port)?;
    if matches.len() != 1 {
        return Err(inconsistent(
            "initial anchor mutation audit is absent or duplicated",
        ));
    }
    let sequence: i64 = matches[0].get(0);
    if sequence < 0 {
        return Err(inconsistent("initial anchor audit sequence is negative"));
    }
    let row = tx
        .query_opt(
            "SELECT sequence,timestamp,actor,action,resource,chain FROM audit_events
             WHERE sequence=$1 AND octet_length(timestamp) BETWEEN 1 AND 64
             AND octet_length(actor) BETWEEN 1 AND 1280
             AND octet_length(action) BETWEEN 1 AND 64
             AND octet_length(resource) BETWEEN 1 AND 512 AND octet_length(chain)=32",
            &[&sequence],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("initial anchor audit is absent or unbounded"))?;
    let timestamp: String = row.get("timestamp");
    let entry = crate::audit_postgres::decode_event(row).map_err(inconsistent)?;
    if entry.event.sequence != sequence as u64
        || entry.event.resource != marker
        || entry.event.action != action
        || entry.event.actor != snapshot.recorded_by.email
        || entry.event.timestamp != snapshot.recorded_at
        || entry.event.timestamp.offset() != time::UtcOffset::UTC
        || entry.event.timestamp_rfc3339()? != timestamp
    {
        return Err(inconsistent(
            "initial anchor original audit differs from its receipt",
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
            .ok_or_else(|| inconsistent("initial anchor audit predecessor is absent"))?;
        let bytes: Vec<u8> = row.get(0);
        Sha256Digest::from_bytes(&bytes).map_err(inconsistent)?
    };
    if chain_digest(hasher, &previous, &entry.event)? != entry.chain {
        return Err(inconsistent(
            "initial anchor audit chain commitment differs",
        ));
    }
    Ok(sequence)
}
