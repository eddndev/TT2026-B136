use super::{helpers::*, inconsistent, Result};
use application::hearing_results::DeclaredHearingResultTime;
use domain::{judicial_calendars::CivilDate, procedural_time::DeclaredProceduralTime};
use serde_json::Value;
use time::{Date, Month, Time, UtcOffset};

fn offset(value: &Value) -> Result<UtcOffset> {
    UtcOffset::from_whole_seconds(integer(value)?).map_err(|_| inconsistent())
}
fn civil(value: &Value) -> Result<CivilDate> {
    string(value)?.parse().map_err(|_| inconsistent())
}
pub(super) fn value_time(value: &Value) -> Result<DeclaredProceduralTime> {
    declared(value, false)
}
pub(super) fn source_time(value: &Value) -> Result<DeclaredProceduralTime> {
    declared(value, true)
}
fn declared(value: &Value, source: bool) -> Result<DeclaredProceduralTime> {
    let precision = string(&value["precision"])?;
    if precision == "unknown" {
        fields(value, &["precision"])?;
        return Ok(DeclaredProceduralTime::unknown());
    }
    let mut expected = if source {
        vec!["precision", "date", "offset_seconds"]
    } else {
        vec!["precision", "year", "month", "day", "offset_seconds"]
    };
    match precision {
        "date" => {}
        "minute" => expected.extend(["hour", "minute"]),
        "second" => expected.extend(["hour", "minute", "second"]),
        _ => return Err(inconsistent()),
    }
    fields(value, &expected)?;
    let date = if source {
        civil(&value["date"])?
    } else {
        CivilDate::from_date(
            Date::from_calendar_date(
                integer(&value["year"])?,
                Month::try_from(byte(&value["month"])?).map_err(|_| inconsistent())?,
                byte(&value["day"])?,
            )
            .map_err(|_| inconsistent())?,
        )
        .map_err(|_| inconsistent())?
    };
    let offset = optional(&value["offset_seconds"], offset)?;
    match precision {
        "date" => DeclaredProceduralTime::date(date, offset),
        "minute" => DeclaredProceduralTime::minute(
            date,
            byte(&value["hour"])?,
            byte(&value["minute"])?,
            offset,
        ),
        "second" => DeclaredProceduralTime::second(
            date,
            byte(&value["hour"])?,
            byte(&value["minute"])?,
            byte(&value["second"])?,
            offset,
        ),
        _ => return Err(inconsistent()),
    }
    .map_err(|_| inconsistent())
}
pub(super) fn hearing_time(value: &Value) -> Result<DeclaredHearingResultTime> {
    match string(&value["precision"])? {
        "date" => {
            fields(value, &["precision", "date", "offset_seconds"])?;
            DeclaredHearingResultTime::date(
                civil(&value["date"])?.date(),
                offset(&value["offset_seconds"])?,
            )
            .map_err(|_| inconsistent())
        }
        "instant" => {
            fields(
                value,
                &[
                    "precision",
                    "date",
                    "hour",
                    "minute",
                    "second",
                    "offset_seconds",
                ],
            )?;
            let time = Time::from_hms(
                byte(&value["hour"])?,
                byte(&value["minute"])?,
                byte(&value["second"])?,
            )
            .map_err(|_| inconsistent())?;
            let date = civil(&value["date"])?.date();
            DeclaredHearingResultTime::instant(
                date.with_time(time)
                    .assume_offset(offset(&value["offset_seconds"])?),
            )
            .map_err(|_| inconsistent())
        }
        _ => Err(inconsistent()),
    }
}
