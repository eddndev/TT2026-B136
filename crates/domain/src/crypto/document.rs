//! Identity and versioning for a stored document.
//!
//! Encryption binds a ciphertext to a document identity and version so that a
//! ciphertext cannot be silently swapped for another document or rolled back
//! to an earlier version. These value objects carry that identity.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

/// Stable unique identity of a document.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(Uuid);

impl DocumentId {
    /// Generates a fresh random identity.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wraps an existing UUID.
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Returns the inner UUID.
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DocumentId({})", self.0)
    }
}

/// Monotonic version counter for a document, starting at 1.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u32")]
pub struct DocumentVersion(u32);

impl DocumentVersion {
    /// The version assigned to a newly stored document.
    pub const fn initial() -> Self {
        Self(1)
    }

    /// Builds a version from a raw counter, rejecting zero.
    pub fn new(value: u32) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::InvalidDocumentVersion);
        }
        Ok(Self(value))
    }

    /// Returns the raw counter.
    pub const fn get(&self) -> u32 {
        self.0
    }

    /// Returns the next version, rejecting an exhausted counter.
    pub fn next(&self) -> Result<Self, DomainError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(DomainError::DocumentVersionExhausted)
    }
}

impl TryFrom<u32> for DocumentVersion {
    type Error = DomainError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for DocumentVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for DocumentVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DocumentVersion({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ids_are_distinct() {
        assert_ne!(DocumentId::new(), DocumentId::new());
    }

    #[test]
    fn id_round_trips_through_uuid() {
        let id = DocumentId::new();
        assert_eq!(DocumentId::from_uuid(id.as_uuid()), id);
    }

    #[test]
    fn initial_version_is_one() {
        assert_eq!(DocumentVersion::initial().get(), 1);
    }

    #[test]
    fn version_rejects_zero() {
        assert_eq!(
            DocumentVersion::new(0),
            Err(DomainError::InvalidDocumentVersion)
        );
    }

    #[test]
    fn next_increments_version() {
        let version = DocumentVersion::new(3).unwrap();
        assert_eq!(version.next().unwrap().get(), 4);
    }
}
