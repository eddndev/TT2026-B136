use application::hearing_results::{DeclaredHearingResultPrecision, DeclaredHearingResultTime};
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
    pub fn validate(self) -> Result<DeclaredHearingResultTime, DomainError> {
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
                    return Err(DomainError::InvalidHearingResultValue("event_time"));
                }
                let date = Date::parse(&date, format_description!("[year]-[month]-[day]"))
                    .map_err(|_| DomainError::InvalidHearingResultValue("event_time"))?;
                DeclaredHearingResultTime::date(date, parse_offset(&offset)?)
            }
            Self::Instant { at } => {
                let value = parse_instant(&at)?;
                DeclaredHearingResultTime::instant(value)
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
        return Err(DomainError::InvalidHearingResultValue("event_time"));
    }
    let hours = (bytes[1] - b'0') * 10 + bytes[2] - b'0';
    let minutes = (bytes[4] - b'0') * 10 + bytes[5] - b'0';
    if hours > 14 || minutes > 59 || (hours == 14 && minutes != 0) {
        return Err(DomainError::InvalidHearingResultValue("event_time"));
    }
    let sign = if bytes[0] == b'-' { -1 } else { 1 };
    UtcOffset::from_hms(sign * hours as i8, sign * minutes as i8, 0)
        .map_err(|_| DomainError::InvalidHearingResultValue("event_time"))
}
impl TryFrom<DeclaredHearingResultTime> for DeclaredTime {
    type Error = ApiError;
    fn try_from(value: DeclaredHearingResultTime) -> Result<Self, Self::Error> {
        match value.precision() {
            DeclaredHearingResultPrecision::Date => Ok(Self::Date {
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
            DeclaredHearingResultPrecision::Instant => Ok(Self::Instant {
                at: value
                    .instant_value()
                    .ok_or_else(ApiError::internal)?
                    .format(&Rfc3339)
                    .map_err(|_| ApiError::internal())?,
            }),
        }
    }
}

fn parse_instant(value: &str) -> Result<OffsetDateTime, DomainError> {
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
        return Err(DomainError::InvalidHearingResultValue("event_time"));
    }
    let at = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| DomainError::InvalidHearingResultValue("event_time"))?;
    Ok(at)
}
pub(super) fn utc(value: OffsetDateTime) -> Result<String, ApiError> {
    value
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())
}
