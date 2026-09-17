//! Stable deadline identity and registry state, independent of legal effects.
mod identity;
pub use identity::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeadlineValueError {
    #[error("deadline revision must be positive")]
    InvalidRevision,
    #[error("deadline revision is exhausted")]
    RevisionExhausted,
    #[error("unknown deadline registry status")]
    InvalidStatus,
}

/// Attention and expiration are independent of whether the registry is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeadlineStatus {
    Active,
    Retired,
}
impl DeadlineStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Retired => "retired",
        }
    }
}
impl FromStr for DeadlineStatus {
    type Err = DeadlineValueError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(Self::Active),
            "retired" => Ok(Self::Retired),
            _ => Err(DeadlineValueError::InvalidStatus),
        }
    }
}
