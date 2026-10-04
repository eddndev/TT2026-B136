use crate::ApplicationError;

pub(super) fn blob(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value);
}

pub(super) fn validate_actor_email(value: &str) -> Result<(), ApplicationError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().any(char::is_control)
        || u32::try_from(value.len()).is_err()
    {
        return Err(ApplicationError::InvalidInput(
            "invalid captured actor email".into(),
        ));
    }
    Ok(())
}
