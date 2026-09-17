use crate::DomainError;

pub(super) fn required(
    value: &str,
    max: usize,
    multiline: bool,
    field: &'static str,
) -> Result<String, DomainError> {
    let normalized = value.replace("\r\n", "\n");
    if normalized
        .chars()
        .any(|c| c.is_control() && !(multiline && c == '\n'))
    {
        return Err(DomainError::InvalidJudicialCalendarValue(field));
    }
    let value = normalized.trim();
    if value.is_empty() || value.chars().count() > max {
        return Err(DomainError::InvalidJudicialCalendarValue(field));
    }
    Ok(value.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarReason(String);
impl JudicialCalendarReason {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        required(value, 1000, true, "reason").map(Self)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
