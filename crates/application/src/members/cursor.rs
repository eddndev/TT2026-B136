use super::{MemberError, Role, UserId};

pub(super) fn prefix(value: Option<&str>) -> Result<Option<String>, MemberError> {
    let Some(value) = value else { return Ok(None) };
    if !value.is_ascii() || value.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(MemberError::Invalid(
            "email prefix must be ASCII without controls",
        ));
    }
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.len() > 254 {
        return Err(MemberError::Invalid("email prefix exceeds 254 bytes"));
    }
    Ok((!normalized.is_empty()).then_some(normalized))
}

pub(super) fn limit(value: u32) -> Result<(), MemberError> {
    if !(1..=100).contains(&value) {
        return Err(MemberError::Invalid("page limit must be between 1 and 100"));
    }
    Ok(())
}

pub(super) fn role(value: Option<Role>) -> &'static str {
    value.map_or("-", Role::as_str)
}

pub(super) fn encoded_prefix(value: Option<&str>) -> String {
    let Some(value) = value else {
        return "-".into();
    };
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 15)]));
    }
    output
}

pub(super) fn after(
    raw: Option<&str>,
    id_position: usize,
    canonical: impl FnOnce(UserId) -> String,
) -> Result<Option<UserId>, MemberError> {
    let Some(raw) = raw else { return Ok(None) };
    if raw.len() > 768 || !raw.is_ascii() {
        return Err(MemberError::Invalid("invalid directory cursor"));
    }
    let identifier = raw
        .split(':')
        .nth(id_position)
        .ok_or(MemberError::Invalid("invalid directory cursor"))?;
    let id = UserId::from_uuid(
        identifier
            .parse()
            .map_err(|_| MemberError::Invalid("invalid directory cursor"))?,
    );
    if canonical(id) != raw {
        return Err(MemberError::Invalid(
            "directory cursor differs from its filters",
        ));
    }
    Ok(Some(id))
}
