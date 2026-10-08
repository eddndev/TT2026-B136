use super::measure_request::invalid;
use crate::{error::ApiError, procedural_facts::object::Object};
use domain::{
    hearings::HearingNote,
    judicial_calendars::CivilDate,
    precautionary_measures::{MeasureTime, MeasureValidity},
    procedural_time::DeclaredProceduralTime,
};
use serde::Deserialize;
use time::{Date, Month, UtcOffset};

#[derive(Deserialize)]
#[serde(tag = "precision", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum DeclaredTime {
    Unknown {
        reason: String,
    },
    Date {
        year: i32,
        month: u8,
        day: u8,
        #[serde(deserialize_with = "super::primitives::nullable")]
        offset_seconds: Option<i32>,
    },
    Minute {
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        #[serde(deserialize_with = "super::primitives::nullable")]
        offset_seconds: Option<i32>,
    },
    Second {
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        #[serde(deserialize_with = "super::primitives::nullable")]
        offset_seconds: Option<i32>,
    },
}
impl DeclaredTime {
    pub(super) fn validate(self) -> Result<MeasureTime, ApiError> {
        let value = match self {
            Self::Unknown { reason } => {
                return MeasureTime::new(
                    DeclaredProceduralTime::unknown(),
                    Some(HearingNote::new(&reason).map_err(|_| invalid())?),
                )
                .map_err(|_| invalid())
            }
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
        }
        .map_err(|_| invalid())?;
        MeasureTime::new(value, None).map_err(|_| invalid())
    }
}
fn date(year: i32, month: u8, day: u8) -> Result<CivilDate, ApiError> {
    let month = Month::try_from(month).map_err(|_| invalid())?;
    CivilDate::from_date(Date::from_calendar_date(year, month, day).map_err(|_| invalid())?)
        .map_err(|_| invalid())
}
fn offset(value: Option<i32>) -> Result<Option<UtcOffset>, ApiError> {
    value
        .map(|v| UtcOffset::from_whole_seconds(v).map_err(|_| invalid()))
        .transpose()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Validity {
    start: Object<DeclaredTime>,
    statement: String,
    #[serde(deserialize_with = "super::primitives::nullable")]
    end: Option<Object<DeclaredTime>>,
}
impl Validity {
    pub(super) fn validate(self) -> Result<MeasureValidity, ApiError> {
        MeasureValidity::new(
            self.start.0.validate()?,
            HearingNote::new(&self.statement).map_err(|_| invalid())?,
            self.end.map(|v| v.0.validate()).transpose()?,
        )
        .map_err(|_| invalid())
    }
}
