use domain::{audit::AuditEvent, clock::OffsetDateTime};

/// Aggregate UTF-8 bytes of stored actor, action and resource values per page.
pub const MAX_AUDIT_PAGE_TEXT_BYTES: usize = 262_144;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuditEventPosition {
    pub seconds: i64,
    pub nanos: u32,
    pub sequence: u64,
}

impl AuditEventPosition {
    pub fn of(event: &AuditEvent) -> Self {
        Self {
            seconds: event.timestamp.unix_timestamp(),
            nanos: event.timestamp.nanosecond(),
            sequence: event.sequence,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditEventCursor {
    pub snapshot_max_sequence: u64,
    pub after: AuditEventPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEventBatch {
    pub snapshot_max_sequence: Option<u64>,
    pub events: Vec<AuditEvent>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEventPage {
    pub checked_at: OffsetDateTime,
    pub snapshot_max_sequence: Option<u64>,
    pub events: Vec<AuditEvent>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}
