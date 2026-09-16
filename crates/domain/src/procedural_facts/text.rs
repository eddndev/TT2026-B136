use crate::DomainError;

/// Nonempty single-line declaration, bounded to 200 Unicode scalar values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactLabel(String);
impl FactLabel {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        normalized(value, 200, false, "label").map(Self)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Nonempty multiline declaration, bounded to 1000 Unicode scalar values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactText(String);
impl FactText {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        normalized(value, 1000, true, "text").map(Self)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn normalized(
    value: &str,
    limit: usize,
    multiline: bool,
    field: &'static str,
) -> Result<String, DomainError> {
    let value = if multiline {
        value.replace("\r\n", "\n")
    } else {
        value.to_owned()
    };
    if value
        .chars()
        .any(|ch| ch.is_control() && !(multiline && ch == '\n'))
    {
        return Err(DomainError::InvalidProceduralFact(field));
    }
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > limit {
        return Err(DomainError::InvalidProceduralFact(field));
    }
    Ok(trimmed.to_owned())
}
