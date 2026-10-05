use super::{primitives::*, Result};
use domain::{
    hearings::HearingNote,
    judicial_calendars::CivilDate,
    precautionary_measures::MeasureTime,
    procedural_time::{DeclaredProceduralPrecision as Precision, DeclaredProceduralTime},
};
use serde_json::{json, Value};
use time::{Date, Month, UtcOffset};

pub(crate) fn decode(value: &Value) -> Result<MeasureTime> {
    let precision = string(&value["precision"])?;
    let declared = match precision {
        "unknown" => {
            fields(value, &["precision", "reason"])?;
            return MeasureTime::new(
                DeclaredProceduralTime::unknown(),
                Some(note(&value["reason"])?),
            )
            .map_err(|_| inconsistent());
        }
        "date" => {
            fields(
                value,
                &["precision", "year", "month", "day", "offset_seconds"],
            )?;
            DeclaredProceduralTime::date(date(value)?, offset(&value["offset_seconds"])?)
        }
        "minute" => {
            fields(
                value,
                &[
                    "precision",
                    "year",
                    "month",
                    "day",
                    "hour",
                    "minute",
                    "offset_seconds",
                ],
            )?;
            DeclaredProceduralTime::minute(
                date(value)?,
                component(&value["hour"])?,
                component(&value["minute"])?,
                offset(&value["offset_seconds"])?,
            )
        }
        "second" => {
            fields(
                value,
                &[
                    "precision",
                    "year",
                    "month",
                    "day",
                    "hour",
                    "minute",
                    "second",
                    "offset_seconds",
                ],
            )?;
            DeclaredProceduralTime::second(
                date(value)?,
                component(&value["hour"])?,
                component(&value["minute"])?,
                component(&value["second"])?,
                offset(&value["offset_seconds"])?,
            )
        }
        _ => return Err(inconsistent()),
    }
    .map_err(|_| inconsistent())?;
    MeasureTime::new(declared, None).map_err(|_| inconsistent())
}

fn component(value: &Value) -> Result<u8> {
    u8::try_from(integer(value)?).map_err(|_| inconsistent())
}

fn date(value: &Value) -> Result<CivilDate> {
    let year = u16::try_from(integer(&value["year"])?).map_err(|_| inconsistent())?;
    let month = Month::try_from(component(&value["month"])?).map_err(|_| inconsistent())?;
    let day = component(&value["day"])?;
    let date = Date::from_calendar_date(i32::from(year), month, day).map_err(|_| inconsistent())?;
    CivilDate::from_date(date).map_err(|_| inconsistent())
}

fn offset(value: &Value) -> Result<Option<UtcOffset>> {
    if value.is_null() {
        return Ok(None);
    }
    let seconds = value
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(inconsistent)?;
    UtcOffset::from_whole_seconds(seconds)
        .map(Some)
        .map_err(|_| inconsistent())
}

pub(crate) fn view(value: &MeasureTime) -> Value {
    let declared = value.declared();
    let precision = match declared.precision() {
        Precision::Unknown => {
            return json!({
                "precision": "unknown",
                "reason": value.unknown_reason().map(HearingNote::as_str),
            });
        }
        Precision::Date => "date",
        Precision::Minute => "minute",
        Precision::Second => "second",
    };
    let date = declared
        .local_date()
        .expect("known precision contains a date")
        .date();
    let mut result = json!({
        "precision": precision,
        "year": date.year(),
        "month": date.month() as u8,
        "day": date.day(),
        "offset_seconds": declared.offset().map(UtcOffset::whole_seconds),
    });
    if let Some(hour) = declared.local_hour() {
        result["hour"] = json!(hour);
    }
    if let Some(minute) = declared.local_minute() {
        result["minute"] = json!(minute);
    }
    if let Some(second) = declared.local_second() {
        result["second"] = json!(second);
    }
    result
}
