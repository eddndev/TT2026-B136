use std::str::FromStr;

use crate::case_stages::CaseStage;
use crate::DomainError;

/// Ordinary appointment classification; it does not declare a hearing result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HearingKind {
    Initial,
    Intermediate,
    OralTrial,
    Sentencing,
}

impl HearingKind {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Initial => 0,
            Self::Intermediate => 1,
            Self::OralTrial => 2,
            Self::Sentencing => 3,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Intermediate => "intermediate",
            Self::OralTrial => "oral_trial",
            Self::Sentencing => "sentencing",
        }
    }

    pub const fn required_stage(self) -> CaseStage {
        match self {
            Self::Initial => CaseStage::Investigation,
            Self::Intermediate => CaseStage::Intermediate,
            Self::OralTrial | Self::Sentencing => CaseStage::Trial,
        }
    }
}

impl FromStr for HearingKind {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "initial" => Ok(Self::Initial),
            "intermediate" => Ok(Self::Intermediate),
            "oral_trial" => Ok(Self::OralTrial),
            "sentencing" => Ok(Self::Sentencing),
            _ => Err(DomainError::InvalidHearingValue("kind")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HearingModality {
    InPerson,
    Videoconference,
}

impl HearingModality {
    pub const fn tag(self) -> u8 {
        match self {
            Self::InPerson => 0,
            Self::Videoconference => 1,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InPerson => "in_person",
            Self::Videoconference => "videoconference",
        }
    }
}

impl FromStr for HearingModality {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "in_person" => Ok(Self::InPerson),
            "videoconference" => Ok(Self::Videoconference),
            _ => Err(DomainError::InvalidHearingValue("modality")),
        }
    }
}

/// Organizational availability, independent of whether an appointment is past.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HearingStatus {
    Scheduled,
    Cancelled,
}

impl HearingStatus {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Scheduled => 0,
            Self::Cancelled => 1,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for HearingStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "scheduled" => Ok(Self::Scheduled),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(DomainError::InvalidHearingValue("status")),
        }
    }
}
