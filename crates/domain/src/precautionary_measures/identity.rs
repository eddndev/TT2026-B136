use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Identity of one declared decision, independent of measure and appointment identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MeasureDecisionId(Uuid);

impl MeasureDecisionId {
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
impl Default for MeasureDecisionId {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Display for MeasureDecisionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Identity of one decision submission, retained for exact reconciliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MeasureDecisionOperationId(Uuid);

impl MeasureDecisionOperationId {
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
impl Default for MeasureDecisionOperationId {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Display for MeasureDecisionOperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Identity of one administrative correction, separate from judicial decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MeasureCorrectionOperationId(Uuid);

impl MeasureCorrectionOperationId {
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
impl Default for MeasureCorrectionOperationId {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Display for MeasureCorrectionOperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}
