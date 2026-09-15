//! Stable identity and authorization vocabulary.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainError;

/// Stable identifier for a persisted user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(Uuid);

impl UserId {
    /// Creates a random user identifier.
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

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A user's authorization role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Litigator,
    Paralegal,
    Client,
}

impl Role {
    /// Returns the stable database and wire-format name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Litigator => "litigator",
            Self::Paralegal => "paralegal",
            Self::Client => "client",
        }
    }

    /// Decides whether the role grants a permission.
    pub const fn allows(self, permission: Permission) -> bool {
        match self {
            Self::Owner => true,
            Self::Litigator => matches!(
                permission,
                Permission::ReadDocument
                    | Permission::CreateDocument
                    | Permission::AppendDocument
                    | Permission::SealDocument
                    | Permission::VerifyDocument
                    | Permission::ExportEvidence
            ),
            Self::Paralegal => matches!(
                permission,
                Permission::ReadDocument
                    | Permission::CreateDocument
                    | Permission::AppendDocument
                    | Permission::VerifyDocument
                    | Permission::ExportEvidence
            ),
            Self::Client => false,
        }
    }
}

impl FromStr for Role {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "owner" => Ok(Self::Owner),
            "litigator" => Ok(Self::Litigator),
            "paralegal" => Ok(Self::Paralegal),
            "client" => Ok(Self::Client),
            other => Err(DomainError::InvalidRole(other.to_owned())),
        }
    }
}

/// An application action that requires authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    ReadDocument,
    CreateDocument,
    AppendDocument,
    SealDocument,
    VerifyDocument,
    ExportEvidence,
    VerifyAudit,
    CreateUser,
}
