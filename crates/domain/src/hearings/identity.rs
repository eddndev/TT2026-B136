use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainError;

/// Stable identity of one hearing, independent of its replacement revisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HearingId(Uuid);

impl HearingId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for HearingId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for HearingId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Identity chosen for one submission, used to reconcile an uncertain response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HearingOperationId(Uuid);

impl HearingOperationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for HearingOperationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for HearingOperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Positive immutable revision of a hearing's organizational record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32")]
pub struct HearingRevision(u32);

impl HearingRevision {
    pub const fn initial() -> Self {
        Self(1)
    }

    pub fn new(value: u32) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::InvalidHearingRevision);
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl TryFrom<u32> for HearingRevision {
    type Error = DomainError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
