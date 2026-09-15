use crate::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageNote(String);

impl StageNote {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        normalized(value, 1000, true)
            .filter(|value| !value.is_empty())
            .map(Self)
            .ok_or(DomainError::InvalidStageNote)
    }

    pub fn optional(value: Option<&str>) -> Result<Option<Self>, DomainError> {
        optional(value, 1000, true)
            .map(|value| value.map(Self))
            .ok_or(DomainError::InvalidStageNote)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageCourt(String);

impl StageCourt {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        normalized(value, 200, false)
            .filter(|value| !value.is_empty())
            .map(Self)
            .ok_or(DomainError::InvalidStageCourt)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageReceiptReference(String);

impl StageReceiptReference {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        normalized(value, 200, false)
            .filter(|value| !value.is_empty())
            .map(Self)
            .ok_or(DomainError::InvalidStageReceiptReference)
    }

    pub fn optional(value: Option<&str>) -> Result<Option<Self>, DomainError> {
        optional(value, 200, false)
            .map(|value| value.map(Self))
            .ok_or(DomainError::InvalidStageReceiptReference)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn optional(value: Option<&str>, limit: usize, multiline: bool) -> Option<Option<String>> {
    match value {
        None => Some(None),
        Some(value) => {
            normalized(value, limit, multiline).map(|value| (!value.is_empty()).then_some(value))
        }
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
