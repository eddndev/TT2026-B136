use crate::DomainError;
use std::fmt;
use uuid::Uuid;

/// Stable identity of one declared resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolutionId(Uuid);
impl ResolutionId {
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
impl Default for ResolutionId {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Display for ResolutionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Stable identity of one declared notification practice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NotificationId(Uuid);
impl NotificationId {
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
impl Default for NotificationId {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Display for NotificationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Identity of one submission and its exact operation receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactOperationId(Uuid);
impl FactOperationId {
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
impl Default for FactOperationId {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Display for FactOperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Positive immutable revision; exhaustion never wraps to zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactRevision(u32);
impl FactRevision {
    pub const fn initial() -> Self {
        Self(1)
    }
    pub fn new(value: u32) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::InvalidProceduralFact("revision"));
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
