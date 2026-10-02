use super::{cursor, AuditEventCursor};
use crate::ApplicationError;
use domain::{audit::AuditEvent, clock::OffsetDateTime};
use time::{Duration, UtcOffset};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEventQuery {
    from: OffsetDateTime,
    until: OffsetDateTime,
    actor: Option<String>,
    action: Option<String>,
    resource: Option<String>,
    limit: u32,
    cursor: Option<AuditEventCursor>,
}

impl AuditEventQuery {
    pub fn new(
        from: OffsetDateTime,
        until: OffsetDateTime,
        actor: Option<&str>,
        action: Option<&str>,
        resource: Option<&str>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Self, ApplicationError> {
        let from = from.to_offset(UtcOffset::UTC);
        let until = until.to_offset(UtcOffset::UTC);
        if !(1..=9999).contains(&from.year())
            || !(1..=9999).contains(&until.year())
            || from >= until
            || until - from > Duration::days(366)
        {
            return Err(invalid("audit interval must contain at most 366 days"));
        }
        if !(1..=100).contains(&limit) {
            return Err(invalid("audit page limit must be between 1 and 100"));
        }
        let mut query = Self {
            from,
            until,
            actor: filter(actor, 254)?,
            action: filter(action, 128)?,
            resource: filter(resource, 1024)?,
            limit,
            cursor: None,
        };
        query.cursor = cursor.map(|raw| cursor::decode(&query, raw)).transpose()?;
        Ok(query)
    }
    pub const fn from(&self) -> OffsetDateTime {
        self.from
    }
    pub const fn until(&self) -> OffsetDateTime {
        self.until
    }
    pub fn actor(&self) -> Option<&str> {
        self.actor.as_deref()
    }
    pub fn action(&self) -> Option<&str> {
        self.action.as_deref()
    }
    pub fn resource(&self) -> Option<&str> {
        self.resource.as_deref()
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn cursor(&self) -> Option<AuditEventCursor> {
        self.cursor
    }
    pub fn cursor_after(
        &self,
        snapshot_max_sequence: u64,
        last: &AuditEvent,
    ) -> Result<String, ApplicationError> {
        if !self.matches(last) {
            return Err(invalid("audit continuation event differs from the filters"));
        }
        let cursor = AuditEventCursor {
            snapshot_max_sequence,
            after: super::AuditEventPosition::of(last),
        };
        cursor::validate(self, cursor)?;
        Ok(cursor::encode(self, cursor))
    }
    pub(super) fn matches(&self, event: &AuditEvent) -> bool {
        event.timestamp >= self.from
            && event.timestamp < self.until
            && self.actor().is_none_or(|value| value == event.actor)
            && self.action().is_none_or(|value| value == event.action)
            && self.resource().is_none_or(|value| value == event.resource)
    }
}

fn filter(value: Option<&str>, limit: usize) -> Result<Option<String>, ApplicationError> {
    let Some(value) = value else { return Ok(None) };
    if value.is_empty() || value.len() > limit || value.chars().any(char::is_control) {
        return Err(invalid(
            "audit filter is empty, contains controls or exceeds its byte limit",
        ));
    }
    Ok(Some(value.to_owned()))
}

pub(super) fn invalid(message: &str) -> ApplicationError {
    ApplicationError::InvalidInput(message.into())
}
