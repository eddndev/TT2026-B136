use crate::DomainError;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HearingResultOccurrence {
    Occurred,
    NotStarted,
}
impl HearingResultOccurrence {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Occurred => 0,
            Self::NotStarted => 1,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Occurred => "occurred",
            Self::NotStarted => "not_started",
        }
    }
}
impl FromStr for HearingResultOccurrence {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "occurred" => Ok(Self::Occurred),
            "not_started" => Ok(Self::NotStarted),
            _ => Err(DomainError::InvalidHearingResultValue("occurrence")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HearingResultExtent {
    Partial,
    Concluded,
    Unspecified,
}
impl HearingResultExtent {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Partial => 0,
            Self::Concluded => 1,
            Self::Unspecified => 2,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Partial => "partial",
            Self::Concluded => "concluded",
            Self::Unspecified => "unspecified",
        }
    }
}
impl FromStr for HearingResultExtent {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "partial" => Ok(Self::Partial),
            "concluded" => Ok(Self::Concluded),
            "unspecified" => Ok(Self::Unspecified),
            _ => Err(DomainError::InvalidHearingResultValue("extent")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HearingResultStatus {
    Recorded,
    Withdrawn,
}
impl HearingResultStatus {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Recorded => 0,
            Self::Withdrawn => 1,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Recorded => "recorded",
            Self::Withdrawn => "withdrawn",
        }
    }
}
impl FromStr for HearingResultStatus {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "recorded" => Ok(Self::Recorded),
            "withdrawn" => Ok(Self::Withdrawn),
            _ => Err(DomainError::InvalidHearingResultValue("status")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HearingResultProvenanceKind {
    OperatorNote,
    OralReference,
    WrittenRecord,
}
impl HearingResultProvenanceKind {
    pub const fn tag(self) -> u8 {
        match self {
            Self::OperatorNote => 0,
            Self::OralReference => 1,
            Self::WrittenRecord => 2,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OperatorNote => "operator_note",
            Self::OralReference => "oral_reference",
            Self::WrittenRecord => "written_record",
        }
    }
}
impl FromStr for HearingResultProvenanceKind {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "operator_note" => Ok(Self::OperatorNote),
            "oral_reference" => Ok(Self::OralReference),
            "written_record" => Ok(Self::WrittenRecord),
            _ => Err(DomainError::InvalidHearingResultValue("provenance.kind")),
        }
    }
}
