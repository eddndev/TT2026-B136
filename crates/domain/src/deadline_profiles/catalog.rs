use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeadlineProfileValueError {
    #[error("deadline profile revision must be positive")]
    InvalidRevision,
    #[error("deadline profile revision is exhausted")]
    RevisionExhausted,
    #[error("unknown deadline profile algorithm")]
    InvalidAlgorithm,
    #[error("unknown deadline profile status")]
    InvalidStatus,
}

/// The persisted algorithm selects an immutable interpretation of the definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeadlineProfileAlgorithm {
    V1,
}
impl DeadlineProfileAlgorithm {
    pub const fn tag(self) -> u8 {
        match self {
            Self::V1 => 1,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "v1",
        }
    }
}
impl FromStr for DeadlineProfileAlgorithm {
    type Err = DeadlineProfileValueError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "v1" => Ok(Self::V1),
            _ => Err(Self::Err::InvalidAlgorithm),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeadlineProfileStatus {
    Published,
    Retired,
}
impl DeadlineProfileStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::Retired => "retired",
        }
    }
}
impl FromStr for DeadlineProfileStatus {
    type Err = DeadlineProfileValueError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "published" => Ok(Self::Published),
            "retired" => Ok(Self::Retired),
            _ => Err(Self::Err::InvalidStatus),
        }
    }
}
