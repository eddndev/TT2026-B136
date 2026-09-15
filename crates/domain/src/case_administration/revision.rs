use std::str::FromStr;

use crate::DomainError;

/// Organizational availability in the firm, separate from a legal stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaseAdministrativeStatus {
    Active,
    Closed,
}

impl CaseAdministrativeStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Closed => "closed",
        }
    }
}
impl FromStr for CaseAdministrativeStatus {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(Self::Active),
            "closed" => Ok(Self::Closed),
            _ => Err(DomainError::InvalidCaseAdministrativeStatus),
        }
    }
}

/// A positive immutable administrative revision; a baseline is not a revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CaseRevision(u32);
impl CaseRevision {
    pub const FIRST: Self = Self(1);
    pub fn new(value: u32) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::InvalidCaseRevision);
        }
        Ok(Self(value))
    }
    pub const fn get(self) -> u32 {
        self.0
    }
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// A separate positive counter for registered stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CaseStageRevision(u32);
impl CaseStageRevision {
    pub const FIRST: Self = Self(1);
    pub fn new(value: u32) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::InvalidCaseStageRevision);
        }
        Ok(Self(value))
    }
    pub const fn get(self) -> u32 {
        self.0
    }
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// The only stage supplied by complete new penal registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InitialCaseStage {
    Investigation,
}
impl InitialCaseStage {
    pub const fn as_str(self) -> &'static str {
        "investigation"
    }
}
