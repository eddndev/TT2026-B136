use super::{invalid, Reader};
use crate::ApplicationError;
use domain::{judicial_calendars::CivilDate, procedural_time::DeclaredProceduralPrecision};
use std::num::NonZeroU32;
use time::{OffsetDateTime, UtcOffset};

pub(super) fn quantity(reader: &mut Reader<'_>) -> Result<NonZeroU32, ApplicationError> {
    NonZeroU32::new(reader.u32()?).ok_or_else(|| invalid("zero quantity"))
}
pub(super) fn count(reader: &mut Reader<'_>, maximum: u32) -> Result<usize, ApplicationError> {
    let value = reader.u32()?;
    if value > maximum {
        return Err(invalid("collection exceeds its bound"));
    }
    usize::try_from(value).map_err(invalid)
}
pub(super) fn date(reader: &mut Reader<'_>) -> Result<CivilDate, ApplicationError> {
    CivilDate::from_days_since_epoch(reader.i32()?).map_err(invalid)
}
pub(super) fn instant(reader: &mut Reader<'_>) -> Result<OffsetDateTime, ApplicationError> {
    let seconds = i64::from_be_bytes(reader.take(8)?.try_into().map_err(invalid)?);
    let nanos = reader.u32()?;
    if nanos >= 1_000_000_000 {
        return Err(invalid("invalid nanosecond"));
    }
    let offset = UtcOffset::from_whole_seconds(reader.i32()?).map_err(invalid)?;
    OffsetDateTime::from_unix_timestamp_nanos(
        i128::from(seconds) * 1_000_000_000 + i128::from(nanos),
    )
    .map_err(invalid)?
    .checked_to_offset(offset)
    .ok_or_else(|| invalid("instant offset out of range"))
}
pub(super) fn precision(
    reader: &mut Reader<'_>,
) -> Result<DeclaredProceduralPrecision, ApplicationError> {
    match reader.byte()? {
        0 => Ok(DeclaredProceduralPrecision::Unknown),
        1 => Ok(DeclaredProceduralPrecision::Date),
        2 => Ok(DeclaredProceduralPrecision::Minute),
        3 => Ok(DeclaredProceduralPrecision::Second),
        _ => Err(invalid("invalid precision")),
    }
}
pub(super) fn write_date(bytes: &mut Vec<u8>, value: CivilDate) {
    bytes.extend(value.days_since_epoch().to_be_bytes());
}
pub(super) fn write_instant(bytes: &mut Vec<u8>, value: OffsetDateTime) {
    bytes.extend(value.unix_timestamp().to_be_bytes());
    bytes.extend(value.nanosecond().to_be_bytes());
    bytes.extend(value.offset().whole_seconds().to_be_bytes());
}
pub(super) fn write_precision(value: DeclaredProceduralPrecision) -> u8 {
    match value {
        DeclaredProceduralPrecision::Unknown => 0,
        DeclaredProceduralPrecision::Date => 1,
        DeclaredProceduralPrecision::Minute => 2,
        DeclaredProceduralPrecision::Second => 3,
    }
}
