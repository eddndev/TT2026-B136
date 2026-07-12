//! Use cases over the audit trail: append an event, verify the chain, and
//! list the entries for display.

use domain::audit::{verify_chain, AuditLog, ChainVerification, ChainedEvent};
use domain::clock::Clock;
use domain::crypto::DocumentHasher;

use crate::error::ApplicationError;

/// Records one action in the audit log.
///
/// The caller names the actor, action, and resource; the timestamp comes
/// from the clock port and the sequence number and chain value are assigned
/// by the audit log.
pub struct AppendAuditEvent<L, C> {
    log: L,
    clock: C,
}

impl<L: AuditLog, C: Clock> AppendAuditEvent<L, C> {
    /// Builds the use case over an audit log and a clock.
    pub fn new(log: L, clock: C) -> Self {
        Self { log, clock }
    }

    /// Appends the event and returns it as stored, chain value included.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::Domain`] when the log cannot store the
    /// event or the event cannot be encoded for hashing.
    pub fn execute(
        &mut self,
        actor: &str,
        action: &str,
        resource: &str,
    ) -> Result<ChainedEvent, ApplicationError> {
        let timestamp = self.clock.now();
        Ok(self.log.append(actor, action, resource, timestamp)?)
    }
}

/// Recomputes the whole hash chain of the audit log and reports the result.
pub struct VerifyAuditChain<L, H> {
    log: L,
    hasher: H,
}

impl<L: AuditLog, H: DocumentHasher> VerifyAuditChain<L, H> {
    /// Builds the use case over an audit log and a hasher.
    pub fn new(log: L, hasher: H) -> Self {
        Self { log, hasher }
    }

    /// Loads every entry and verifies the chain from its genesis value.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::Domain`] when the log cannot be read or
    /// an entry cannot be encoded for hashing.
    pub fn execute(&self) -> Result<ChainVerification, ApplicationError> {
        let entries = self.log.load_all()?;
        Ok(verify_chain(&self.hasher, &entries)?)
    }
}

/// Loads the audit trail for display.
pub struct ShowAuditLog<L> {
    log: L,
}

impl<L: AuditLog> ShowAuditLog<L> {
    /// Builds the use case over an audit log.
    pub fn new(log: L) -> Self {
        Self { log }
    }

    /// Returns every stored entry in append order.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::Domain`] when the log cannot be read.
    pub fn execute(&self) -> Result<Vec<ChainedEvent>, ApplicationError> {
        Ok(self.log.load_all()?)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use domain::audit::{chain_digest, AuditEvent, GENESIS_PREVIOUS};
    use domain::clock::OffsetDateTime;
    use domain::crypto::Sha256Digest;
    use domain::DomainError;
    use mockall::mock;
    use mockall::predicate::eq;

    use super::*;

    mock! {
        Log {}

        impl AuditLog for Log {
            fn append(
                &mut self,
                actor: &str,
                action: &str,
                resource: &str,
                timestamp: OffsetDateTime,
            ) -> Result<ChainedEvent, DomainError>;
            fn load_all(&self) -> Result<Vec<ChainedEvent>, DomainError>;
        }
    }

    mock! {
        FixedClock {}

        impl Clock for FixedClock {
            fn now(&self) -> OffsetDateTime;
        }
    }

    mock! {
        Hasher {}

        impl DocumentHasher for Hasher {
            fn hash_bytes(&self, data: &[u8]) -> Sha256Digest;
            fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError>;
        }
    }

    fn instant() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap()
    }

    fn entry(sequence: u64, fill: u8) -> ChainedEvent {
        ChainedEvent {
            event: AuditEvent::new(sequence, instant(), "ana", "open", "case-1"),
            chain: Sha256Digest::from_array([fill; 32]),
        }
    }

    #[test]
    fn append_takes_the_timestamp_from_the_clock_and_returns_the_stored_entry() {
        let mut clock = MockFixedClock::new();
        clock.expect_now().times(1).returning(instant);

        let stored = entry(0, 7);
        let expected = stored.clone();
        let mut log = MockLog::new();
        log.expect_append()
            .with(eq("ana"), eq("open"), eq("case-1"), eq(instant()))
            .times(1)
            .return_once(move |_, _, _, _| Ok(stored));

        let mut use_case = AppendAuditEvent::new(log, clock);
        let result = use_case.execute("ana", "open", "case-1").unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn append_surfaces_a_storage_failure_as_a_domain_error() {
        let mut clock = MockFixedClock::new();
        clock.expect_now().times(1).returning(instant);

        let mut log = MockLog::new();
        log.expect_append()
            .times(1)
            .returning(|_, _, _, _| Err(DomainError::AuditStorageFailure("disk full".to_string())));

        let mut use_case = AppendAuditEvent::new(log, clock);
        let err = use_case.execute("ana", "open", "case-1").unwrap_err();
        assert!(matches!(
            err,
            ApplicationError::Domain(DomainError::AuditStorageFailure(_))
        ));
    }

    #[test]
    fn verify_reports_an_empty_log_as_valid_without_hashing() {
        let mut log = MockLog::new();
        log.expect_load_all().times(1).returning(|| Ok(Vec::new()));

        let use_case = VerifyAuditChain::new(log, MockHasher::new());
        assert_eq!(
            use_case.execute().unwrap(),
            ChainVerification::Valid { entries: 0 }
        );
    }

    #[test]
    fn verify_accepts_an_entry_whose_stored_chain_matches_the_recomputation() {
        let digest = Sha256Digest::from_array([5u8; 32]);
        let mut stored = entry(0, 0);
        stored.chain = digest;
        let entries = vec![stored.clone()];

        let mut log = MockLog::new();
        log.expect_load_all()
            .times(1)
            .return_once(move || Ok(entries));

        // The recomputed link must hash the genesis value followed by the
        // canonical bytes of the event.
        let canonical = stored.event.canonical_bytes().unwrap();
        let mut hasher = MockHasher::new();
        hasher.expect_hash_bytes().times(1).returning(move |data| {
            let mut expected = GENESIS_PREVIOUS.as_bytes().to_vec();
            expected.extend_from_slice(&canonical);
            assert_eq!(data, expected.as_slice());
            digest
        });

        let use_case = VerifyAuditChain::new(log, hasher);
        assert_eq!(
            use_case.execute().unwrap(),
            ChainVerification::Valid { entries: 1 }
        );
    }

    #[test]
    fn verify_reports_the_first_index_whose_chain_does_not_match() {
        let entries = vec![entry(0, 1), entry(1, 2)];
        let mut log = MockLog::new();
        log.expect_load_all()
            .times(1)
            .return_once(move || Ok(entries));

        // The hasher returns a digest that matches no stored chain value,
        // so the first entry already fails.
        let mut hasher = MockHasher::new();
        hasher
            .expect_hash_bytes()
            .returning(|_| Sha256Digest::from_array([9u8; 32]));

        let use_case = VerifyAuditChain::new(log, hasher);
        assert_eq!(
            use_case.execute().unwrap(),
            ChainVerification::Broken {
                first_broken_index: 0,
            }
        );
    }

    #[test]
    fn show_returns_the_entries_in_append_order() {
        let entries = vec![entry(0, 1), entry(1, 2)];
        let expected = entries.clone();
        let mut log = MockLog::new();
        log.expect_load_all()
            .times(1)
            .return_once(move || Ok(entries));

        let use_case = ShowAuditLog::new(log);
        assert_eq!(use_case.execute().unwrap(), expected);
    }

    #[test]
    fn show_surfaces_a_storage_failure_as_a_domain_error() {
        let mut log = MockLog::new();
        log.expect_load_all().times(1).returning(|| {
            Err(DomainError::AuditStorageFailure(
                "unreadable line".to_string(),
            ))
        });

        let use_case = ShowAuditLog::new(log);
        assert!(matches!(
            use_case.execute().unwrap_err(),
            ApplicationError::Domain(DomainError::AuditStorageFailure(_))
        ));
    }

    /// The use cases and the real chain rule agree: a chain built with
    /// `chain_digest` through a fake hasher passes verification.
    #[test]
    fn verify_accepts_a_chain_built_with_the_domain_chain_rule() {
        struct XorHasher;
        impl DocumentHasher for XorHasher {
            fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
                let mut out = [0u8; 32];
                for (i, byte) in data.iter().enumerate() {
                    out[i % 32] ^= *byte;
                }
                Sha256Digest::from_array(out)
            }
            fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
                Ok(self.hash_bytes(&[]))
            }
        }

        let mut previous = GENESIS_PREVIOUS;
        let mut entries = Vec::new();
        for sequence in 0..3u64 {
            let event = AuditEvent::new(sequence, instant(), "ana", "open", "case-1");
            let chain = chain_digest(&XorHasher, &previous, &event).unwrap();
            previous = chain;
            entries.push(ChainedEvent { event, chain });
        }

        let mut log = MockLog::new();
        let loaded = entries.clone();
        log.expect_load_all()
            .times(1)
            .return_once(move || Ok(loaded));

        let use_case = VerifyAuditChain::new(log, XorHasher);
        assert_eq!(
            use_case.execute().unwrap(),
            ChainVerification::Valid { entries: 3 }
        );
    }
}
