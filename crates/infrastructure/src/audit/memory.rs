//! Vec-backed audit log for tests and short-lived processes.

use domain::audit::{chain_digest, AuditEvent, AuditLog, ChainedEvent, GENESIS_PREVIOUS};
use domain::crypto::DocumentHasher;
use domain::DomainError;
use time::OffsetDateTime;

/// [`AuditLog`] held entirely in memory.
///
/// Entries live in a Vec and are lost when the value drops; use
/// [`super::FileAuditLog`] when the trail must survive the process.
pub struct InMemoryAuditLog<H> {
    hasher: H,
    entries: Vec<ChainedEvent>,
}

impl<H: DocumentHasher> InMemoryAuditLog<H> {
    /// Creates an empty log that chains entries with `hasher`.
    pub fn new(hasher: H) -> Self {
        Self {
            hasher,
            entries: Vec::new(),
        }
    }
}

impl<H: DocumentHasher> AuditLog for InMemoryAuditLog<H> {
    fn append(
        &mut self,
        actor: &str,
        action: &str,
        resource: &str,
        timestamp: OffsetDateTime,
    ) -> Result<ChainedEvent, DomainError> {
        let sequence = self.entries.len() as u64;
        let previous = self
            .entries
            .last()
            .map_or(GENESIS_PREVIOUS, |entry| entry.chain);
        let event = AuditEvent::new(sequence, timestamp, actor, action, resource);
        let chain = chain_digest(&self.hasher, &previous, &event)?;
        let entry = ChainedEvent { event, chain };
        self.entries.push(entry.clone());
        Ok(entry)
    }

    fn load_all(&self) -> Result<Vec<ChainedEvent>, DomainError> {
        Ok(self.entries.clone())
    }
}

#[cfg(test)]
mod tests {
    use domain::audit::{verify_chain, ChainVerification};

    use super::*;
    use crate::hashing::RingSha256Hasher;

    fn timestamp(offset_seconds: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_735_689_600 + offset_seconds).unwrap()
    }

    #[test]
    fn an_empty_log_loads_no_entries() {
        let log = InMemoryAuditLog::new(RingSha256Hasher::new());
        assert_eq!(log.load_all().unwrap(), vec![]);
    }

    #[test]
    fn appended_entries_get_consecutive_sequences_and_verify() {
        let mut log = InMemoryAuditLog::new(RingSha256Hasher::new());
        for i in 0..3 {
            let entry = log.append("ana", "open", "case-1", timestamp(i)).unwrap();
            assert_eq!(entry.event.sequence, i as u64);
        }
        let entries = log.load_all().unwrap();
        assert_eq!(
            verify_chain(&RingSha256Hasher::new(), &entries).unwrap(),
            ChainVerification::Valid { entries: 3 }
        );
    }

    #[test]
    fn the_first_entry_links_to_the_genesis_value() {
        let mut log = InMemoryAuditLog::new(RingSha256Hasher::new());
        let entry = log.append("ana", "open", "case-1", timestamp(0)).unwrap();
        let expected =
            chain_digest(&RingSha256Hasher::new(), &GENESIS_PREVIOUS, &entry.event).unwrap();
        assert_eq!(entry.chain, expected);
    }

    #[test]
    fn each_entry_links_to_the_chain_value_of_the_previous_one() {
        let mut log = InMemoryAuditLog::new(RingSha256Hasher::new());
        let first = log.append("ana", "open", "case-1", timestamp(0)).unwrap();
        let second = log.append("bob", "close", "case-1", timestamp(1)).unwrap();
        let expected = chain_digest(&RingSha256Hasher::new(), &first.chain, &second.event).unwrap();
        assert_eq!(second.chain, expected);
    }
}
