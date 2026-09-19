use crate::{
    deadline_reevaluation::SourceEventReference,
    deadlines::{DeadlineError, DeadlineId},
    ApplicationError,
};

/// Bound one dispatch transaction without imposing a lifetime job quota.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineDispatchLimit(u32);

impl DeadlineDispatchLimit {
    pub fn new(value: u32) -> Result<Self, ApplicationError> {
        if !(1..=100).contains(&value) {
            return Err(DeadlineError::Invalid("dispatch page limit").into());
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Default for DeadlineDispatchLimit {
    fn default() -> Self {
        Self(20)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineDispatchStream {
    Events,
    LegacyBootstrap,
}

/// The caller selects a stream and page size, never an arbitrary cursor or actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineDispatchRequest {
    pub stream: DeadlineDispatchStream,
    pub limit: DeadlineDispatchLimit,
}

/// Committed event position. Present sequences must fit positive PostgreSQL BIGINT.
/// An active event and exclusive UUID appear together after a partial page. UUID
/// nil is a valid position; None denotes the position before all UUID values.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineEventDispatchPosition {
    pub completed_sequence: Option<u64>,
    pub active_sequence: Option<u64>,
    pub after_deadline_id: Option<DeadlineId>,
}

/// The two streams advance independently. Completing a legacy sweep resets its
/// position so a later sweep can find new deadlines below the previous cursor.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineDispatchProgress {
    pub event: DeadlineEventDispatchPosition,
    pub bootstrap_after_deadline_id: Option<DeadlineId>,
}

/// Progress disclosed after commit. A completed scan covers one event or legacy
/// sweep; it does not mean that jobs have executed or all deadlines are current.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineDispatchBatch {
    pub stream: DeadlineDispatchStream,
    pub event: Option<SourceEventReference>,
    pub selected: u32,
    pub inserted: u32,
    pub completed_scan: bool,
    pub progress: DeadlineDispatchProgress,
}
