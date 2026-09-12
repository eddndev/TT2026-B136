//! Case identifiers, metadata invariants, and membership policy.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{identity::Role, DomainError};

/// Stable identifier for a persisted legal case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CaseId(Uuid);

impl CaseId {
    /// Creates a random case identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wraps an existing UUID.
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the wrapped UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for CaseId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CaseId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Required, bounded human-facing case metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseMetadata {
    title: String,
    reference: String,
}

impl CaseMetadata {
    /// Trims surrounding whitespace and validates both fields.
    ///
    /// Limits count Unicode scalar values. Control characters are rejected
    /// before trimming, including otherwise removable line breaks and tabs.
    pub fn new(title: &str, reference: &str) -> Result<Self, DomainError> {
        Ok(Self {
            title: validate_field(title, "title", 200)?,
            reference: validate_field(reference, "reference", 100)?,
        })
    }

    /// Returns the normalized title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the normalized case reference.
    pub fn reference(&self) -> &str {
        &self.reference
    }
}

fn validate_field(value: &str, field: &'static str, max: usize) -> Result<String, DomainError> {
    if value.chars().any(char::is_control) {
        return Err(DomainError::InvalidCaseMetadata {
            field,
            reason: "control characters are not allowed",
        });
    }
    let value = value.trim();
    if value.is_empty() {
        return Err(DomainError::InvalidCaseMetadata {
            field,
            reason: "must not be empty",
        });
    }
    if value.chars().count() > max {
        return Err(DomainError::InvalidCaseMetadata {
            field,
            reason: "exceeds the character limit",
        });
    }
    Ok(value.to_owned())
}

/// Owners and litigators may create cases.
pub const fn can_create_case(role: Role) -> bool {
    matches!(role, Role::Owner | Role::Litigator)
}

/// Only owners may assign or remove case members.
pub const fn can_manage_members(role: Role) -> bool {
    matches!(role, Role::Owner)
}

/// Owners may read all cases; every other role requires current membership.
pub const fn can_read_case(role: Role, is_member: bool) -> bool {
    matches!(role, Role::Owner) || is_member
}
