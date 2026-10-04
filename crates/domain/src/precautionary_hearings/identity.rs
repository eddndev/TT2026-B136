use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainError;

/// Stable identity of a precautionary appointment, distinct from an ordinary hearing root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PrecautionaryHearingId(Uuid);

impl PrecautionaryHearingId {
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

impl Default for PrecautionaryHearingId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PrecautionaryHearingId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Identity of one submission, retained when reconciling an uncertain response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PrecautionaryHearingOperationId(Uuid);

impl PrecautionaryHearingOperationId {
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

impl Default for PrecautionaryHearingOperationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PrecautionaryHearingOperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Positive immutable revision of a precautionary appointment; exhaustion never wraps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32")]
pub struct PrecautionaryHearingRevision(u32);

impl PrecautionaryHearingRevision {
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

impl TryFrom<u32> for PrecautionaryHearingRevision {
    type Error = DomainError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Stable identity of a declared measure, distinct from its appointment or decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MeasureId(Uuid);

impl MeasureId {
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

impl Default for MeasureId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MeasureId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Positive immutable revision of a declared measure; exhaustion never wraps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32")]
pub struct MeasureRevision(u32);

impl MeasureRevision {
    pub const fn initial() -> Self {
        Self(1)
    }

    pub fn new(value: u32) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::InvalidHearingValue("measure_revision"));
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

impl TryFrom<u32> for MeasureRevision {
    type Error = DomainError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
