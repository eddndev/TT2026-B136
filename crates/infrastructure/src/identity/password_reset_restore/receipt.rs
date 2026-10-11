use super::{
    rejected, storage, CapabilityKind, PasswordResetRestoreHead, PasswordResetRestoreRequest,
    PasswordResetRestoreResult,
};
use application::ApplicationError;
use domain::audit::{verify_chain, ChainVerification, ChainedEvent};
use postgres::Transaction;
use std::collections::HashSet;
use time::OffsetDateTime;
use uuid::Uuid;

const ACTOR: &str = "database-restore";

pub(super) fn verified_events(
    transaction: &mut Transaction<'_>,
) -> Result<Vec<ChainedEvent>, ApplicationError> {
    let events = crate::audit_postgres::load_transaction(transaction).map_err(|_| storage())?;
    if events
        .iter()
        .enumerate()
        .any(|(index, entry)| u64::try_from(index).ok() != Some(entry.event.sequence))
        || verify_chain(&crate::RingSha256Hasher, &events).map_err(|_| rejected())?
            != (ChainVerification::Valid {
                entries: events.len(),
            })
    {
        return Err(rejected());
    }
    Ok(events)
}

pub(super) fn head(entry: Option<&ChainedEvent>) -> Option<PasswordResetRestoreHead> {
    entry.map(|entry| PasswordResetRestoreHead {
        sequence: entry.event.sequence,
        chain: entry.chain,
    })
}

pub(super) fn existing(
    events: &[ChainedEvent],
    request: &PasswordResetRestoreRequest,
    kind: CapabilityKind,
) -> Result<Option<PasswordResetRestoreResult>, ApplicationError> {
    let mut identities = HashSet::new();
    let mut found = None;
    for (index, entry) in events
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.event.action == kind.action())
    {
        let (identity, count) = parse(entry, kind)?;
        if !identities.insert(identity) {
            return Err(rejected());
        }
        if identity == request.operation_id {
            if head(events[..index].last()) != request.expected_head {
                return Err(rejected());
            }
            found = Some(PasswordResetRestoreResult {
                applied: false,
                invalidated: count,
                audit_sequence: entry.event.sequence,
                audit_head: entry.chain,
            });
        }
    }
    Ok(found)
}

fn parse(entry: &ChainedEvent, kind: CapabilityKind) -> Result<(Uuid, u64), ApplicationError> {
    let (identity, count) = entry
        .event
        .resource
        .strip_prefix(kind.resource_prefix())
        .and_then(|value| value.split_once(":cancelled:"))
        .ok_or_else(rejected)?;
    let id = Uuid::parse_str(identity).map_err(|_| rejected())?;
    let amount: u64 = count.parse().map_err(|_| rejected())?;
    if entry.event.actor != ACTOR
        || id.is_nil()
        || id.to_string() != identity
        || amount.to_string() != count
    {
        return Err(rejected());
    }
    Ok((id, amount))
}

pub(super) fn append(
    transaction: &mut Transaction<'_>,
    operation_id: Uuid,
    count: u64,
    at: OffsetDateTime,
    kind: CapabilityKind,
) -> Result<PasswordResetRestoreResult, ApplicationError> {
    let resource = format!("{}{operation_id}:cancelled:{count}", kind.resource_prefix());
    let event =
        crate::audit_postgres::append_transaction(transaction, ACTOR, kind.action(), &resource, at)
            .map_err(|_| storage())?;
    Ok(PasswordResetRestoreResult {
        applied: true,
        invalidated: count,
        audit_sequence: event.event.sequence,
        audit_head: event.chain,
    })
}
