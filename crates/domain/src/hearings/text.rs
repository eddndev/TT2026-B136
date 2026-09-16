use crate::DomainError;

/// Plain text describing a venue or connection; it is not an executable URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingVenue(String);

impl HearingVenue {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        normalized(value, 500, false)
            .filter(|value| !value.is_empty())
            .map(Self)
            .ok_or(DomainError::InvalidHearingValue("venue"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Bounded multiline declaration, also used for mandatory reasons and basis text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingNote(String);

impl HearingNote {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        normalized(value, 1000, true)
            .filter(|value| !value.is_empty())
            .map(Self)
            .ok_or(DomainError::InvalidHearingValue("note"))
    }

    pub fn optional(value: Option<&str>) -> Result<Option<Self>, DomainError> {
        value
            .map(|value| {
                normalized(value, 1000, true)
                    .map(|value| (!value.is_empty()).then_some(Self(value)))
                    .ok_or(DomainError::InvalidHearingValue("note"))
            })
            .transpose()
            .map(Option::flatten)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn normalized(value: &str, limit: usize, multiline: bool) -> Option<String> {
    let value = if multiline {
        value.replace("\r\n", "\n")
    } else {
        value.to_owned()
    };
    if value
        .chars()
        .any(|ch| ch.is_control() && !(multiline && ch == '\n'))
    {
        return None;
    }
    let trimmed = value.trim();
    (trimmed.chars().count() <= limit).then(|| trimmed.to_owned())
}
