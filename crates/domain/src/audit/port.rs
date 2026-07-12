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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::chain::{chain_digest, GENESIS_PREVIOUS};
    use crate::audit::event::AuditEvent;
    use crate::crypto::digest::Sha256Digest;
    use crate::crypto::hasher::DocumentHasher;

    /// Minimal in-crate double proving the port is object safe and usable.
    struct SingleSlotLog {
        entry: Option<ChainedEvent>,
    }

    /// Test double returning a constant digest; only the call flow matters.
    struct ConstantHasher;

    impl DocumentHasher for ConstantHasher {
        fn hash_bytes(&self, _data: &[u8]) -> Sha256Digest {
            Sha256Digest::from_array([9u8; 32])
        }

        fn hash_stream(
            &self,
            _reader: &mut dyn std::io::Read,
        ) -> Result<Sha256Digest, DomainError> {
            Ok(self.hash_bytes(&[]))
        }
    }

    impl AuditLog for SingleSlotLog {
        fn append(
            &mut self,
            actor: &str,
            action: &str,
            resource: &str,
            timestamp: OffsetDateTime,
        ) -> Result<ChainedEvent, DomainError> {
            let event = AuditEvent::new(0, timestamp, actor, action, resource);
            let chain = chain_digest(&ConstantHasher, &GENESIS_PREVIOUS, &event)?;
            let entry = ChainedEvent { event, chain };
            self.entry = Some(entry.clone());
            Ok(entry)
        }

        fn load_all(&self) -> Result<Vec<ChainedEvent>, DomainError> {
            Ok(self.entry.clone().into_iter().collect())
        }
    }

    #[test]
    fn port_is_object_safe_and_round_trips_an_entry() {
        let mut log = SingleSlotLog { entry: None };
        let dynamic: &mut dyn AuditLog = &mut log;
        let timestamp = OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap();
        let appended = dynamic.append("ana", "open", "case-1", timestamp).unwrap();
        assert_eq!(dynamic.load_all().unwrap(), vec![appended]);
    }
}
