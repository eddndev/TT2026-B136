use crate::DomainError;
use std::str::FromStr;

macro_rules! catalog {
    ($name:ident, $field:literal, $( $variant:ident = $tag:literal => $wire:literal ),+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name { $( $variant ),+ }
        impl $name {
            pub const fn as_str(self) -> &'static str { match self { $( Self::$variant => $wire ),+ } }
            pub const fn tag(self) -> u8 { match self { $( Self::$variant => $tag ),+ } }
            pub(crate) fn from_tag(tag: u8) -> Result<Self, DomainError> {
                match tag { $( $tag => Ok(Self::$variant), )+ _ => Err(DomainError::InvalidJudicialCalendarValue($field)) }
            }
        }
        impl FromStr for $name {
            type Err = DomainError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value { $( $wire => Ok(Self::$variant), )+ _ => Err(DomainError::InvalidJudicialCalendarValue($field)) }
            }
        }
    }
}
catalog!(JudicialCalendarClassification, "classification", Countable = 0 => "countable", Excluded = 1 => "excluded", Unresolved = 2 => "unresolved");
catalog!(JudicialCalendarJurisdiction, "scope.jurisdiction", Federal = 0 => "federal", Local = 1 => "local");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JudicialCalendarStatus {
    Published,
    Retired,
}
impl JudicialCalendarStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::Retired => "retired",
        }
    }
    pub const fn tag(self) -> u8 {
        match self {
            Self::Published => 0,
            Self::Retired => 1,
        }
    }
}
impl FromStr for JudicialCalendarStatus {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "published" => Ok(Self::Published),
            "retired" => Ok(Self::Retired),
            _ => Err(DomainError::InvalidJudicialCalendarValue("status")),
        }
    }
}
