use crate::DomainError;
use std::str::FromStr;

/// Declared family; availability and admissibility require a separate legal assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Revocation,
    Appeal,
}
impl ResourceKind {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Revocation => 0,
            Self::Appeal => 1,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Revocation => "revocation",
            Self::Appeal => "appeal",
        }
    }
}
impl FromStr for ResourceKind {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "revocation" => Ok(Self::Revocation),
            "appeal" => Ok(Self::Appeal),
            _ => Err(DomainError::InvalidProceduralResource("kind")),
        }
    }
}

/// Declared mode, wrapped in FactDeclaration when not yet known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceMode {
    Oral,
    Written,
}
impl ResourceMode {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Oral => 0,
            Self::Written => 1,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Oral => "oral",
            Self::Written => "written",
        }
    }
}
impl FromStr for ResourceMode {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "oral" => Ok(Self::Oral),
            "written" => Ok(Self::Written),
            _ => Err(DomainError::InvalidProceduralResource("mode")),
        }
    }
}

/// Organizational availability, separate from withdrawal or a court disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceStatus {
    Active,
    Archived,
}
impl ResourceStatus {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Active => 0,
            Self::Archived => 1,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }
}
impl FromStr for ResourceStatus {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            _ => Err(DomainError::InvalidProceduralResource("status")),
        }
    }
}

/// Recorded assertion about an act, without deriving effects or a mandatory sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceActKind {
    Interposition,
    Admission,
    Inadmissibility,
    Withdrawal,
    Resolution,
}
impl ResourceActKind {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Interposition => 0,
            Self::Admission => 1,
            Self::Inadmissibility => 2,
            Self::Withdrawal => 3,
            Self::Resolution => 4,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Interposition => "interposition",
            Self::Admission => "admission",
            Self::Inadmissibility => "inadmissibility",
            Self::Withdrawal => "withdrawal",
            Self::Resolution => "resolution",
        }
    }
}
impl FromStr for ResourceActKind {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "interposition" => Ok(Self::Interposition),
            "admission" => Ok(Self::Admission),
            "inadmissibility" => Ok(Self::Inadmissibility),
            "withdrawal" => Ok(Self::Withdrawal),
            "resolution" => Ok(Self::Resolution),
            _ => Err(DomainError::InvalidProceduralResource("act_kind")),
        }
    }
}
