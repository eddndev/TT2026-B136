use super::*;
use crate::{identity::IdentityWorkflow, ApplicationError};
use domain::{clock::Clock, identity::Role};
use std::{collections::HashSet, sync::Arc};
use time::UtcOffset;

pub struct AuditEventService {
    store: Arc<dyn AuditEventStore>,
    identity: Arc<dyn IdentityWorkflow>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl AuditEventService {
    pub fn new(
        store: Arc<dyn AuditEventStore>,
        identity: Arc<dyn IdentityWorkflow>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            identity,
            clock,
        }
    }
}
impl AuditEventWorkflow for AuditEventService {
    fn read(
        &self,
        token: &str,
        query: AuditEventQuery,
    ) -> Result<AuditEventPage, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if actor.role != Role::Owner {
            return Err(ApplicationError::PermissionDenied);
        }
        let checked_at = self.clock.now().to_offset(UtcOffset::UTC);
        if !(1..=9999).contains(&checked_at.year()) {
            return Err(ApplicationError::InvalidConfiguration(
                "audit clock is outside RFC3339".into(),
            ));
        }
        let batch = self.store.read(&actor, &query, checked_at)?;
        validate_batch(&query, &batch)?;
        let next_cursor = if batch.has_more {
            let last = batch.events.last().ok_or_else(inconsistent)?;
            let snapshot = batch.snapshot_max_sequence.ok_or_else(inconsistent)?;
            Some(
                query
                    .cursor_after(snapshot, last)
                    .map_err(|_| inconsistent())?,
            )
        } else {
            None
        };
        if self.identity.authenticate(token)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(AuditEventPage {
            checked_at,
            snapshot_max_sequence: batch.snapshot_max_sequence,
            events: batch.events,
            has_more: batch.has_more,
            next_cursor,
        })
    }
}

fn validate_batch(
    query: &AuditEventQuery,
    batch: &AuditEventBatch,
) -> Result<(), ApplicationError> {
    let maximum = batch.snapshot_max_sequence;
    if maximum.is_some_and(|value| value > i64::MAX as u64)
        || batch.events.len() > query.limit() as usize
        || (batch.has_more && batch.events.len() != query.limit() as usize)
        || (maximum.is_none() && (!batch.events.is_empty() || batch.has_more))
        || query
            .cursor()
            .is_some_and(|cursor| maximum != Some(cursor.snapshot_max_sequence))
    {
        return Err(inconsistent());
    }
    let mut previous = query.cursor().map(|cursor| cursor.after);
    let mut sequences = HashSet::new();
    let mut text_bytes = 0usize;
    for event in &batch.events {
        let position = AuditEventPosition::of(event);
        if maximum.is_none_or(|value| event.sequence > value)
            || !query.matches(event)
            || previous.is_some_and(|prior| position <= prior)
            || !sequences.insert(event.sequence)
        {
            return Err(inconsistent());
        }
        for value in [&event.actor, &event.action, &event.resource] {
            text_bytes = text_bytes
                .checked_add(value.len())
                .filter(|value| *value <= MAX_AUDIT_PAGE_TEXT_BYTES)
                .ok_or(ApplicationError::AuditQueryCapacityExceeded)?;
        }
        previous = Some(position);
    }
    Ok(())
}

fn inconsistent() -> ApplicationError {
    ApplicationError::Port("stored audit page violates its query contract".into())
}
