use crate::ApplicationError;

const MIN_PASSWORD_BYTES: usize = 12;
const MAX_PASSWORD_BYTES: usize = 1024;

pub(crate) fn normalize_email(value: &str) -> Result<String, ApplicationError> {
    let normalized = value.trim().to_ascii_lowercase();
    let mut parts = normalized.split('@');
    let valid = !value.bytes().any(|byte| byte.is_ascii_control())
        && normalized.is_ascii()
        && normalized.len() <= 254
        && !normalized.contains(char::is_whitespace)
        && parts.next().is_some_and(|part| !part.is_empty())
        && parts.next().is_some_and(|part| !part.is_empty())
        && parts.next().is_none();
    if !valid {
        return Err(ApplicationError::InvalidInput("invalid email".to_string()));
    }
    Ok(normalized)
}

pub(super) fn validate_password(value: &str) -> Result<(), ApplicationError> {
    if !(MIN_PASSWORD_BYTES..=MAX_PASSWORD_BYTES).contains(&value.len()) {
        return Err(ApplicationError::InvalidInput(format!(
            "password must contain between {MIN_PASSWORD_BYTES} and {MAX_PASSWORD_BYTES} bytes"
        )));
    }
    Ok(())
}
