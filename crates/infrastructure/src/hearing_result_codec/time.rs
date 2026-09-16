use super::{helpers::*, inconsistent, Result};
use ::time::{Date, Month, OffsetDateTime, UtcOffset};
use application::hearing_results::DeclaredHearingResultTime;
use serde_json::Value;

pub(super) fn declared(value: &Value) -> Result<DeclaredHearingResultTime> {
    let delta = value["offset_seconds"]
        .as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .ok_or_else(inconsistent)?;
    let offset = UtcOffset::from_whole_seconds(delta).map_err(|_| inconsistent())?;
    match string(&value["precision"])? {
        "date" => {
            fields(value, &["precision", "date", "offset_seconds"])?;
            let raw = string(&value["date"])?;
            if raw.len() != 10
                || raw.bytes().enumerate().any(|(i, b)| {
                    if i == 4 || i == 7 {
                        b != b'-'
                    } else {
                        !b.is_ascii_digit()
                    }
                })
            {
                return Err(inconsistent());
            }
            let year = raw[..4].parse().map_err(|_| inconsistent())?;
            let month = Month::try_from(raw[5..7].parse::<u8>().map_err(|_| inconsistent())?)
                .map_err(|_| inconsistent())?;
            let day = raw[8..].parse().map_err(|_| inconsistent())?;
            DeclaredHearingResultTime::date(
                Date::from_calendar_date(year, month, day).map_err(|_| inconsistent())?,
                offset,
            )
            .map_err(|_| inconsistent())
        }
        "instant" => {
            fields(value, &["precision", "seconds", "offset_seconds"])?;
            let seconds = value["seconds"].as_i64().ok_or_else(inconsistent)?;
            let instant = OffsetDateTime::from_unix_timestamp(seconds)
                .map_err(|_| inconsistent())?
                .checked_to_offset(offset)
                .ok_or_else(inconsistent)?;
            DeclaredHearingResultTime::instant(instant).map_err(|_| inconsistent())
        }
        _ => Err(inconsistent()),
    }
}
