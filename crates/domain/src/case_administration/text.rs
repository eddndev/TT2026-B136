use crate::DomainError;

pub(super) fn required(
    value: &str,
    field: &'static str,
    maximum: usize,
) -> Result<String, DomainError> {
    validate_controls(value, field, false)?;
    let value = normalized(value, field, maximum)?;
    if value.is_empty() {
        return Err(invalid(field, "must not be empty"));
    }
    Ok(value.to_owned())
}

pub(super) fn optional(
    value: Option<&str>,
    field: &'static str,
    maximum: usize,
    multiline: bool,
) -> Result<Option<String>, DomainError> {
    let Some(value) = value else { return Ok(None) };
    let normalized_lines;
    let value = if multiline {
        normalized_lines = value.replace("\r\n", "\n");
        normalized_lines.as_str()
    } else {
        value
    };
    validate_controls(value, field, multiline)?;
    let value = normalized(value, field, maximum)?;
    Ok((!value.is_empty()).then(|| value.to_owned()))
}

fn validate_controls(value: &str, field: &'static str, multiline: bool) -> Result<(), DomainError> {
    if value
        .chars()
        .any(|c| c.is_control() && !(multiline && c == '\n'))
    {
        return Err(invalid(field, "control characters are not allowed"));
    }
    Ok(())
}

fn normalized<'a>(
    value: &'a str,
    field: &'static str,
    maximum: usize,
) -> Result<&'a str, DomainError> {
    let value = value.trim();
    if value.chars().count() > maximum {
        return Err(invalid(field, "exceeds the character limit"));
    }
    Ok(value)
}

pub(super) fn invalid(field: &'static str, reason: &'static str) -> DomainError {
    DomainError::InvalidPenalCaseProfile { field, reason }
}
