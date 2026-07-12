//! Hash-chain computation and verification for the audit trail.
//!
//! Chain rule: the chain value of entry `n` is
//! `SHA-256(chain(n - 1) || canonical_bytes(entry n))`, where `||` is byte
//! concatenation and `canonical_bytes` is the encoding documented on
//! [`AuditEvent::canonical_bytes`]. For the first entry the previous chain
//! value is [`GENESIS_PREVIOUS`], 32 zero bytes. Because every chain value
//! covers the full history before it, altering, inserting, deleting, or
//! reordering any past entry changes the recomputed value at that position.

use crate::audit::event::{AuditEvent, ChainedEvent};
use crate::crypto::digest::{Sha256Digest, SHA256_LEN};
use crate::crypto::hasher::DocumentHasher;
use crate::error::DomainError;

/// Previous chain value used for the first entry: 32 zero bytes.
///
/// A fixed, data-independent genesis value means an empty log needs no
/// stored state and the first entry is computed by the same rule as every
/// other entry.
pub const GENESIS_PREVIOUS: Sha256Digest = Sha256Digest::from_array([0u8; SHA256_LEN]);

/// Computes the chain value of an event given the previous chain value.
///
/// # Errors
///
/// Propagates the canonical-encoding failures of
/// [`AuditEvent::canonical_bytes`].
pub fn chain_digest(
    hasher: &dyn DocumentHasher,
    previous: &Sha256Digest,
    event: &AuditEvent,
) -> Result<Sha256Digest, DomainError> {
    let canonical = event.canonical_bytes()?;
    let mut linked = Vec::with_capacity(SHA256_LEN + canonical.len());
    linked.extend_from_slice(previous.as_bytes());
    linked.extend_from_slice(&canonical);
    Ok(hasher.hash_bytes(&linked))
}

/// Outcome of verifying a whole chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainVerification {
    /// Every stored chain value matches its recomputation.
    Valid {
        /// Number of entries that were checked.
        entries: usize,
    },
    /// At least one stored chain value does not match.
    Broken {
        /// Zero-based index of the first entry whose stored chain value
        /// differs from the recomputed one.
        first_broken_index: usize,
    },
}

/// Recomputes every link of `entries` and compares it to the stored value.
///
/// The recomputation starts from [`GENESIS_PREVIOUS`] and applies the chain
/// rule documented at the top of this module, so alteration, insertion,
/// deletion, and reordering of past entries are all detected. The result
/// reports the first index that does not match; an empty slice is valid.
///
/// # Errors
///
/// Propagates the canonical-encoding failures of
/// [`AuditEvent::canonical_bytes`].
pub fn verify_chain(
    hasher: &dyn DocumentHasher,
    entries: &[ChainedEvent],
) -> Result<ChainVerification, DomainError> {
    let mut previous = GENESIS_PREVIOUS;
    for (index, entry) in entries.iter().enumerate() {
        let expected = chain_digest(hasher, &previous, &entry.event)?;
        if expected != entry.chain {
            return Ok(ChainVerification::Broken {
                first_broken_index: index,
            });
        }
        previous = entry.chain;
    }
    Ok(ChainVerification::Valid {
        entries: entries.len(),
    })
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;

    /// Deterministic test hasher: four seeded FNV-1a lanes over the input.
    ///
    /// Not a cryptographic hash; it only has to be deterministic and to make
    /// accidental collisions between the small test inputs implausible, so
    /// the chain logic can be exercised without a crypto crate in this
    /// crate's dependency graph.
    struct FnvLaneHasher;

    impl DocumentHasher for FnvLaneHasher {
        fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
            let mut out = [0u8; SHA256_LEN];
            for lane in 0u64..4 {
                let mut state = 0xcbf2_9ce4_8422_2325u64 ^ lane.wrapping_mul(0x9e37_79b9_7f4a_7c15);
                for &byte in data {
                    state = (state ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
                }
                let start = lane as usize * 8;
                out[start..start + 8].copy_from_slice(&state.to_be_bytes());
            }
            Sha256Digest::from_array(out)
        }

        fn hash_stream(&self, reader: &mut dyn std::io::Read) -> Result<Sha256Digest, DomainError> {
            let mut data = Vec::new();
            reader
                .read_to_end(&mut data)
                .map_err(|err| DomainError::StreamRead {
                    message: err.to_string(),
                })?;
            Ok(self.hash_bytes(&data))
        }
    }

    fn event(sequence: u64, actor: &str, action: &str, resource: &str) -> AuditEvent {
        let base = OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap();
        let timestamp = base + time::Duration::seconds(sequence as i64);
        AuditEvent::new(sequence, timestamp, actor, action, resource)
    }

    fn build_chain(count: u64) -> Vec<ChainedEvent> {
        let hasher = FnvLaneHasher;
        let mut previous = GENESIS_PREVIOUS;
        let mut entries = Vec::new();
        for sequence in 0..count {
            let event = event(sequence, "ana", "open", "case-1");
            let chain = chain_digest(&hasher, &previous, &event).unwrap();
            previous = chain;
            entries.push(ChainedEvent { event, chain });
        }
        entries
    }

    #[test]
    fn the_first_link_hashes_the_zero_genesis_and_the_canonical_bytes() {
        let hasher = FnvLaneHasher;
        let first = event(0, "ana", "open", "case-1");
        let mut expected_input = vec![0u8; SHA256_LEN];
        expected_input.extend_from_slice(&first.canonical_bytes().unwrap());
        assert_eq!(
            chain_digest(&hasher, &GENESIS_PREVIOUS, &first).unwrap(),
            hasher.hash_bytes(&expected_input)
        );
    }

    #[test]
    fn an_empty_log_verifies_as_valid() {
        assert_eq!(
            verify_chain(&FnvLaneHasher, &[]).unwrap(),
            ChainVerification::Valid { entries: 0 }
        );
    }

    #[test]
    fn an_intact_chain_verifies_as_valid() {
        let entries = build_chain(5);
        assert_eq!(
            verify_chain(&FnvLaneHasher, &entries).unwrap(),
            ChainVerification::Valid { entries: 5 }
        );
    }

    #[test]
    fn tampering_with_any_field_breaks_the_chain_at_that_index() {
        for index in 0..4usize {
            let mut entries = build_chain(4);
            match index {
                0 => entries[0].event.actor = "mallory".into(),
                1 => entries[1].event.action = "delete".into(),
                2 => entries[2].event.resource = "case-9".into(),
                _ => entries[3].event.sequence = 99,
            }
            assert_eq!(
                verify_chain(&FnvLaneHasher, &entries).unwrap(),
                ChainVerification::Broken {
                    first_broken_index: index,
                }
            );
        }
    }

    #[test]
    fn tampering_with_a_timestamp_breaks_the_chain() {
        let mut entries = build_chain(3);
        entries[1].event.timestamp += time::Duration::seconds(60);
        assert_eq!(
            verify_chain(&FnvLaneHasher, &entries).unwrap(),
            ChainVerification::Broken {
                first_broken_index: 1,
            }
        );
    }

    #[test]
    fn tampering_with_a_stored_chain_value_breaks_the_chain_at_that_index() {
        let mut entries = build_chain(3);
        let mut bytes = *entries[2].chain.as_bytes();
        bytes[0] ^= 0x01;
        entries[2].chain = Sha256Digest::from_array(bytes);
        assert_eq!(
            verify_chain(&FnvLaneHasher, &entries).unwrap(),
            ChainVerification::Broken {
                first_broken_index: 2,
            }
        );
    }

    #[test]
    fn deleting_an_intermediate_entry_breaks_the_chain() {
        let mut entries = build_chain(5);
        entries.remove(2);
        assert_eq!(
            verify_chain(&FnvLaneHasher, &entries).unwrap(),
            ChainVerification::Broken {
                first_broken_index: 2,
            }
        );
    }

    #[test]
    fn swapping_two_entries_breaks_the_chain_at_the_first_moved_one() {
        let mut entries = build_chain(5);
        entries.swap(1, 3);
        assert_eq!(
            verify_chain(&FnvLaneHasher, &entries).unwrap(),
            ChainVerification::Broken {
                first_broken_index: 1,
            }
        );
    }

    #[test]
    fn inserting_a_forged_entry_breaks_the_chain() {
        let mut entries = build_chain(4);
        let forged = ChainedEvent {
            event: event(9, "mallory", "insert", "case-1"),
            chain: entries[1].chain,
        };
        entries.insert(2, forged);
        assert_eq!(
            verify_chain(&FnvLaneHasher, &entries).unwrap(),
            ChainVerification::Broken {
                first_broken_index: 2,
            }
        );
    }
}
