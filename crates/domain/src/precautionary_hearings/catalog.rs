use std::str::FromStr;

use crate::DomainError;

/// Declared purpose; selecting it does not impose or change a measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrecautionaryHearingPurpose {
    Imposition,
    Review,
}

impl PrecautionaryHearingPurpose {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Imposition => 0,
            Self::Review => 1,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Imposition => "imposition",
            Self::Review => "review",
        }
    }
}

impl FromStr for PrecautionaryHearingPurpose {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "imposition" => Ok(Self::Imposition),
            "review" => Ok(Self::Review),
            _ => Err(DomainError::InvalidHearingValue("precautionary_purpose")),
        }
    }
}
