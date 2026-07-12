//! Audit events and their canonical byte encoding.

use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use crate::crypto::digest::Sha256Digest;
use crate::error::DomainError;

/// One recorded action in the audit trail.
///
/// The sequence number is assigned by the audit log when the event is
/// appended: the first event gets 0 and each later event gets the next
/// integer. The remaining fields describe who did what to which resource,
/// and when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEvent {
    /// Zero-based position of the event in the log.
    pub sequence: u64,
    /// Instant the event was recorded.
    pub timestamp: OffsetDateTime,
    /// Identity that performed the action.
    pub actor: String,
    /// Action that was performed.
    pub action: String,
    /// Resource the action applied to.
    pub resource: String,
}

impl AuditEvent {
    /// Builds an event from its fields.
    pub fn new(
        sequence: u64,
        timestamp: OffsetDateTime,
        actor: impl Into<String>,
        action: impl Into<String>,
        resource: impl Into<String>,
    ) -> Self {
        Self {
            sequence,
            timestamp,
            actor: actor.into(),
            action: action.into(),
            resource: resource.into(),
        }
    }

    /// Renders the timestamp as an RFC 3339 string at UTC.
    ///
    /// This is the only textual form of the timestamp the audit trail uses:
    /// the canonical encoding hashes these exact bytes, and adapters that
    /// store timestamps as text must store this exact string so that
    /// verification recomputes the same canonical bytes.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::TimestampNotRepresentable`] when the instant
    /// falls outside the year range RFC 3339 can express.
    pub fn timestamp_rfc3339(&self) -> Result<String, DomainError> {
        self.timestamp
            .to_offset(UtcOffset::UTC)
            .format(&Rfc3339)
            .map_err(|err| DomainError::TimestampNotRepresentable(err.to_string()))
    }

    /// Serializes the event into its canonical byte encoding, the exact
    /// bytes covered by the hash chain.
    ///
    /// Layout, in order, with every variable-length field prefixed by its
    /// byte length as a 4-byte big-endian integer so that no two distinct
    /// events can produce the same bytes:
    ///
    /// ```text
    /// sequence   8 bytes, big endian
    /// timestamp  4-byte length || RFC 3339 UTC string bytes
    /// actor      4-byte length || UTF-8 bytes
    /// action     4-byte length || UTF-8 bytes
    /// resource   4-byte length || UTF-8 bytes
    /// ```
    ///
    /// This encoding exists only for hashing. How an event is stored is a
    /// separate concern of each audit log adapter.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::TimestampNotRepresentable`] when the timestamp
    /// cannot be rendered, or [`DomainError::AuditFieldTooLong`] when a
    /// field does not fit its 4-byte length prefix.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, DomainError> {
        let timestamp = self.timestamp_rfc3339()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        push_length_prefixed(&mut bytes, timestamp.as_bytes())?;
        push_length_prefixed(&mut bytes, self.actor.as_bytes())?;
        push_length_prefixed(&mut bytes, self.action.as_bytes())?;
        push_length_prefixed(&mut bytes, self.resource.as_bytes())?;
        Ok(bytes)
    }
}

/// Appends `field` to `out` behind its 4-byte big-endian length.
fn push_length_prefixed(out: &mut Vec<u8>, field: &[u8]) -> Result<(), DomainError> {
    let len = u32::try_from(field.len()).map_err(|_| DomainError::AuditFieldTooLong {
        max: u32::MAX as usize,
    })?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(field);
    Ok(())
}

/// An audit event together with its chain value.
///
/// The chain value binds the event to every event before it; how it is
/// computed is documented in the `chain` sibling module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainedEvent {
    /// The recorded event.
    pub event: AuditEvent,
    /// SHA-256 chain value covering this event and all previous ones.
    pub chain: Sha256Digest,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_timestamp() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap()
    }

    #[test]
    fn timestamp_renders_as_rfc3339_at_utc() {
        let event = AuditEvent::new(0, sample_timestamp(), "ana", "open", "case-1");
        assert_eq!(event.timestamp_rfc3339().unwrap(), "2025-01-01T00:00:00Z");
    }

    #[test]
    fn canonical_bytes_follow_the_documented_layout() {
        let event = AuditEvent::new(7, sample_timestamp(), "ana", "open", "case-1");
        let bytes = event.canonical_bytes().unwrap();

        let mut expected = Vec::new();
        expected.extend_from_slice(&7u64.to_be_bytes());
        for field in ["2025-01-01T00:00:00Z", "ana", "open", "case-1"] {
            expected.extend_from_slice(&(field.len() as u32).to_be_bytes());
            expected.extend_from_slice(field.as_bytes());
        }
        assert_eq!(bytes, expected);
    }

    #[test]
    fn shifting_a_field_boundary_changes_the_canonical_bytes() {
        // Both events concatenate to the same text ("ab" + "c" == "a" + "bc");
        // the length prefixes must still keep them apart.
        let left = AuditEvent::new(0, sample_timestamp(), "ab", "c", "r");
        let right = AuditEvent::new(0, sample_timestamp(), "a", "bc", "r");
        assert_ne!(
            left.canonical_bytes().unwrap(),
            right.canonical_bytes().unwrap()
        );
    }

    #[test]
    fn each_field_influences_the_canonical_bytes() {
        let base = AuditEvent::new(0, sample_timestamp(), "ana", "open", "case-1");
        let variants = [
            AuditEvent {
                sequence: 1,
                ..base.clone()
            },
            AuditEvent {
                timestamp: sample_timestamp() + time::Duration::seconds(1),
                ..base.clone()
            },
            AuditEvent {
                actor: "bob".into(),
                ..base.clone()
            },
            AuditEvent {
                action: "close".into(),
                ..base.clone()
            },
            AuditEvent {
                resource: "case-2".into(),
                ..base.clone()
            },
        ];
        let baseline = base.canonical_bytes().unwrap();
        for variant in variants {
            assert_ne!(variant.canonical_bytes().unwrap(), baseline);
        }
    }

    #[test]
    fn a_non_utc_timestamp_is_normalized_to_utc() {
        let offset = UtcOffset::from_hms(-6, 0, 0).unwrap();
        let local = AuditEvent::new(
            0,
            sample_timestamp().to_offset(offset),
            "ana",
            "open",
            "case-1",
        );
        let utc = AuditEvent::new(0, sample_timestamp(), "ana", "open", "case-1");
        assert_eq!(
            local.canonical_bytes().unwrap(),
            utc.canonical_bytes().unwrap()
        );
    }

    #[test]
    fn a_timestamp_outside_rfc3339_range_is_rejected() {
        // Year -1 is representable by the datetime type but not by RFC 3339.
        let ancient = OffsetDateTime::from_unix_timestamp(-63_000_000_000).unwrap();
        let event = AuditEvent::new(0, ancient, "ana", "open", "case-1");
        assert!(matches!(
            event.canonical_bytes(),
            Err(DomainError::TimestampNotRepresentable(_))
        ));
    }
}
