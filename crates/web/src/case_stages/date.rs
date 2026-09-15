use application::case_stages::{DeclaredStagePrecision, DeclaredStageTime};
use domain::DomainError;
use serde::{Deserialize, Serialize};
use time::{
    format_description::well_known::Rfc3339, macros::format_description, Date, OffsetDateTime,
    UtcOffset,
};

use crate::error::ApiError;

#[derive(Deserialize, Serialize)]
#[serde(tag = "precision", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum DeclaredTime {
    Date { date: String, offset: String },
    Instant { at: String },
}
impl DeclaredTime {
    pub fn validate(self) -> Result<DeclaredStageTime, DomainError> {
        match self {
            Self::Date { date, offset } => {
                let bytes = date.as_bytes();
                if bytes.len() != 10
                    || bytes[4] != b'-'
                    || bytes[7] != b'-'
                    || !bytes
                        .iter()
                        .enumerate()
                        .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
                {
                    return Err(DomainError::InvalidDeclaredStageTime);
                }
                let date = Date::parse(&date, format_description!("[year]-[month]-[day]"))
                    .map_err(|_| DomainError::InvalidDeclaredStageTime)?;
                DeclaredStageTime::date(date, parse_offset(&offset)?)
            }
            Self::Instant { at } => {
                // RFC 3339 permits leap seconds; the domain clock cannot retain them.
                if at.get(17..19) == Some("60") {
                    return Err(DomainError::InvalidDeclaredStageTime);
                }
                let offset_start = if at.ends_with(['Z', 'z']) {
                    at.len() - 1
                } else {
                    at.len()
                        .checked_sub(6)
                        .ok_or(DomainError::InvalidDeclaredStageTime)?
                };
                if !at.ends_with(['Z', 'z']) {
                    parse_offset(
                        at.get(offset_start..)
                            .ok_or(DomainError::InvalidDeclaredStageTime)?,
                    )?;
                }
                if let Some(fraction) = at.get(19..offset_start).and_then(|s| s.strip_prefix('.')) {
                    if fraction.is_empty()
                        || fraction.len() > 9
                        || !fraction.bytes().all(|b| b.is_ascii_digit())
                    {
                        return Err(DomainError::InvalidDeclaredStageTime);
                    }
                }
                let value = OffsetDateTime::parse(&at, &Rfc3339)
                    .map_err(|_| DomainError::InvalidDeclaredStageTime)?;
                DeclaredStageTime::instant(value)
            }
        }
    }
}
fn parse_offset(value: &str) -> Result<UtcOffset, DomainError> {
    let bytes = value.as_bytes();
    if bytes.len() != 6
        || !matches!(bytes[0], b'+' | b'-')
        || bytes[3] != b':'
        || ![bytes[1], bytes[2], bytes[4], bytes[5]]
            .iter()
            .all(u8::is_ascii_digit)
        || value == "-00:00"
    {
        return Err(DomainError::InvalidDeclaredStageTime);
    }
    let hours = (bytes[1] - b'0') * 10 + bytes[2] - b'0';
    let minutes = (bytes[4] - b'0') * 10 + bytes[5] - b'0';
    if hours > 14 || minutes > 59 || (hours == 14 && minutes != 0) {
        return Err(DomainError::InvalidDeclaredStageTime);
    }
    let sign = if bytes[0] == b'-' { -1 } else { 1 };
    UtcOffset::from_hms(sign * hours as i8, sign * minutes as i8, 0)
        .map_err(|_| DomainError::InvalidDeclaredStageTime)
}
impl TryFrom<DeclaredStageTime> for DeclaredTime {
    type Error = ApiError;
    fn try_from(value: DeclaredStageTime) -> Result<Self, Self::Error> {
        match value.precision() {
            DeclaredStagePrecision::Date => Ok(Self::Date {
                date: value
                    .local_date()
                    .format(format_description!("[year]-[month]-[day]"))
                    .map_err(|_| ApiError::internal())?,
                offset: value
                    .offset()
                    .format(format_description!(
                        "[offset_hour sign:mandatory]:[offset_minute]"
                    ))
                    .map_err(|_| ApiError::internal())?,
            }),
            DeclaredStagePrecision::Instant => Ok(Self::Instant {
                at: value
                    .instant_value()
                    .ok_or_else(ApiError::internal)?
                    .format(&Rfc3339)
                    .map_err(|_| ApiError::internal())?,
            }),
        }
    }
}
