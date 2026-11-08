//! Outbound port for storing and reading the audit trail.

use time::OffsetDateTime;

use crate::audit::event::ChainedEvent;
use crate::error::DomainError;

/// Append-only storage of chained audit events.
///
/// Implementations assign the sequence number (the first entry gets 0) and
/// compute the chain value of each appended event with the chain rule of the
/// `chain` sibling module, linking it to the last stored entry or to the
/// genesis value when the log is empty. Nothing is ever removed or rewritten
/// through this port.
pub trait AuditLog {
    /// Records one event and returns it with its assigned sequence number
    /// and chain value.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::AuditStorageFailure`] when the backing storage
    /// cannot be read or written, or a canonical-encoding error when the
    /// event cannot be hashed.
    fn append(
        &mut self,
        actor: &str,
        action: &str,
        resource: &str,
        timestamp: OffsetDateTime,
    ) -> Result<ChainedEvent, DomainError>;

    /// Loads every stored entry in append order.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::AuditStorageFailure`] when the backing storage
    /// cannot be read or holds an undecodable entry.
    fn load_all(&self) -> Result<Vec<ChainedEvent>, DomainError>;
}
