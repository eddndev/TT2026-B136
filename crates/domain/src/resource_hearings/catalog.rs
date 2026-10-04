use std::str::FromStr;

use crate::procedural_resources::{ResourceKind, ResourceMode};
use crate::DomainError;

/// Declared appointment classification, without deciding legal admissibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceHearingKind {
    AppealArguments,
    WrittenRevocation,
}

impl ResourceHearingKind {
    pub const fn tag(self) -> u8 {
        match self {
            Self::AppealArguments => 0,
            Self::WrittenRevocation => 1,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AppealArguments => "appeal_arguments",
            Self::WrittenRevocation => "written_revocation",
        }
    }

    /// Compare explicit declarations; callers must not infer an unknown mode.
    pub const fn is_compatible_with(self, kind: ResourceKind, mode: ResourceMode) -> bool {
        matches!(
            (self, kind, mode),
            (
                Self::AppealArguments,
                ResourceKind::Appeal,
                ResourceMode::Written
            ) | (
                Self::WrittenRevocation,
                ResourceKind::Revocation,
                ResourceMode::Written
            )
        )
    }
}

impl FromStr for ResourceHearingKind {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "appeal_arguments" => Ok(Self::AppealArguments),
            "written_revocation" => Ok(Self::WrittenRevocation),
            _ => Err(DomainError::InvalidHearingValue("resource_hearing_kind")),
        }
    }
}
