use crate::DomainError;

/// Bounded multiline declaration; no Unicode composition is performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultText(String);
impl HearingResultText {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        normalized(value, 1000, true)
            .filter(|value| !value.is_empty())
            .map(Self)
            .ok_or(DomainError::InvalidHearingResultValue("text"))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn optional(value: Option<&str>) -> Result<Option<Self>, DomainError> {
        value
            .map(|value| {
                normalized(value, 1000, true)
                    .map(|value| (!value.is_empty()).then_some(Self(value)))
                    .ok_or(DomainError::InvalidHearingResultValue("text"))
            })
            .transpose()
            .map(Option::flatten)
    }
}

/// Bounded single-line declaration; no Unicode composition is performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultCapacity(String);
impl HearingResultCapacity {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        normalized(value, 100, false)
            .filter(|value| !value.is_empty())
            .map(Self)
            .ok_or(DomainError::InvalidHearingResultValue("capacity"))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Bounded multiline declaration; no Unicode composition is performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultObservation(String);
impl HearingResultObservation {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        normalized(value, 500, true)
            .filter(|value| !value.is_empty())
            .map(Self)
            .ok_or(DomainError::InvalidHearingResultValue("observation"))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn optional(value: Option<&str>) -> Result<Option<Self>, DomainError> {
        value
            .map(|value| {
                normalized(value, 500, true)
                    .map(|value| (!value.is_empty()).then_some(Self(value)))
                    .ok_or(DomainError::InvalidHearingResultValue("observation"))
            })
            .transpose()
            .map(Option::flatten)
    }
}

/// Bounded single-line declaration; no Unicode composition is performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultReference(String);
impl HearingResultReference {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        normalized(value, 200, false)
            .filter(|value| !value.is_empty())
            .map(Self)
            .ok_or(DomainError::InvalidHearingResultValue(
                "provenance.reference",
            ))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn optional(value: Option<&str>) -> Result<Option<Self>, DomainError> {
        value
            .map(|value| {
                normalized(value, 200, false)
                    .map(|value| (!value.is_empty()).then_some(Self(value)))
                    .ok_or(DomainError::InvalidHearingResultValue(
                        "provenance.reference",
                    ))
            })
            .transpose()
            .map(Option::flatten)
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
