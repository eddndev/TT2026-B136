use std::str::FromStr;

use crate::DomainError;

/// Procedural position, independent of administrative case availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaseStage {
    Investigation,
    Intermediate,
    Trial,
}

impl CaseStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Investigation => "investigation",
            Self::Intermediate => "intermediate",
            Self::Trial => "trial",
        }
    }

    pub const fn permits(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Investigation, Self::Intermediate) | (Self::Intermediate, Self::Trial)
        )
    }
}

impl FromStr for CaseStage {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "investigation" => Ok(Self::Investigation),
            "intermediate" => Ok(Self::Intermediate),
            "trial" => Ok(Self::Trial),
            _ => Err(DomainError::InvalidCaseStage),
        }
    }
}
