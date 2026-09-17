use super::checked;
use crate::error::ApiError;
use domain::{
    judicial_calendars::CivilDate,
    procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime},
    DomainError,
};
use serde::Deserialize;
use serde_json::{json, Value};
use time::{Date, Month, UtcOffset};

#[derive(Deserialize)]
#[serde(tag = "precision", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DeclaredTime {
    Unknown {},
    Date {
        year: i32,
        month: u8,
        day: u8,
        offset_seconds: Option<i32>,
    },
    Minute {
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        offset_seconds: Option<i32>,
    },
    Second {
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        offset_seconds: Option<i32>,
    },
}
impl DeclaredTime {
    pub(crate) fn validate(self) -> Result<DeclaredProceduralTime, ApiError> {
        let result = match self {
            Self::Unknown {} => return Ok(DeclaredProceduralTime::unknown()),
            Self::Date {
                year,
                month,
                day,
                offset_seconds,
            } => DeclaredProceduralTime::date(date(year, month, day)?, offset(offset_seconds)?),
            Self::Minute {
                year,
                month,
                day,
                hour,
                minute,
                offset_seconds,
            } => DeclaredProceduralTime::minute(
                date(year, month, day)?,
                hour,
                minute,
                offset(offset_seconds)?,
            ),
            Self::Second {
                year,
                month,
                day,
                hour,
                minute,
                second,
                offset_seconds,
            } => DeclaredProceduralTime::second(
                date(year, month, day)?,
                hour,
                minute,
                second,
                offset(offset_seconds)?,
            ),
        };
        checked(result)
    }
}
fn invalid() -> ApiError {
    application::ApplicationError::from(DomainError::InvalidDeclaredProceduralTime).into()
}
fn date(year: i32, month: u8, day: u8) -> Result<CivilDate, ApiError> {
    let month = Month::try_from(month).map_err(|_| invalid())?;
    let date = Date::from_calendar_date(year, month, day).map_err(|_| invalid())?;
    CivilDate::from_date(date).map_err(|_| invalid())
}
fn offset(seconds: Option<i32>) -> Result<Option<UtcOffset>, ApiError> {
    seconds
        .map(|v| UtcOffset::from_whole_seconds(v).map_err(|_| invalid()))
        .transpose()
}
pub(crate) fn project(value: DeclaredProceduralTime) -> Result<Value, ApiError> {
    let precision = match value.precision() {
        DeclaredProceduralPrecision::Unknown => return Ok(json!({"precision":"unknown"})),
        DeclaredProceduralPrecision::Date => "date",
        DeclaredProceduralPrecision::Minute => "minute",
        DeclaredProceduralPrecision::Second => "second",
    };
    let date = value.local_date().ok_or_else(ApiError::internal)?.date();
    let mut result = json!({"precision":precision,"year":date.year(),"month":date.month() as u8,
        "day":date.day(),"offset_seconds":value.offset().map(|v|v.whole_seconds())});
    if let Some(hour) = value.local_hour() {
        result["hour"] = json!(hour);
    }
    if let Some(minute) = value.local_minute() {
        result["minute"] = json!(minute);
    }
    if let Some(second) = value.local_second() {
        result["second"] = json!(second);
    }
    Ok(result)
}
