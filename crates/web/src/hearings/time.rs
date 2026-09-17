use crate::error::ApiError;
use application::ApplicationError;
use domain::{hearings::HearingTime, DomainError};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

pub(super) fn parse_time(value: &str) -> Result<HearingTime, ApiError> {
    let bytes = value.as_bytes();
    let shape = matches!(bytes.len(), 20 | 25)
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'T')
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
        && [0..4, 5..7, 8..10, 11..13, 14..16, 17..19]
            .into_iter()
            .all(|r| {
                bytes
                    .get(r)
                    .is_some_and(|p| p.iter().all(u8::is_ascii_digit))
            })
        && ((bytes.len() == 20 && bytes[19] == b'Z')
            || (bytes.len() == 25
                && matches!(bytes[19], b'+' | b'-')
                && bytes[22] == b':'
                && bytes[20..22].iter().all(u8::is_ascii_digit)
                && bytes[23..25].iter().all(u8::is_ascii_digit)));
    if !shape || value.ends_with("-00:00") || &value[17..19] > "59" {
        return Err(invalid());
    }
    let at = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| invalid())?;
    HearingTime::new(at).map_err(|_| invalid())
}
pub(super) fn format_time(value: HearingTime) -> Result<String, ApiError> {
    value
        .value()
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())
}
pub(super) fn utc(value: OffsetDateTime) -> Result<String, ApiError> {
    value
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())
}
fn invalid() -> ApiError {
    ApplicationError::Domain(DomainError::InvalidHearingValue("scheduled_at")).into()
}
