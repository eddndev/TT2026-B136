use super::{FactDeclaration, FactStatus};
use domain::{
    hearing_results::{DeclaredHearingResultPrecision, DeclaredHearingResultTime},
    procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime},
};

pub(super) fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
pub(super) fn optional<T>(
    bytes: &mut Vec<u8>,
    value: Option<&T>,
    encode: impl FnOnce(&mut Vec<u8>, &T),
) {
    bytes.push(u8::from(value.is_some()));
    if let Some(value) = value {
        encode(bytes, value);
    }
}
pub(super) fn declaration<T>(
    bytes: &mut Vec<u8>,
    value: &FactDeclaration<T>,
    encode: impl FnOnce(&mut Vec<u8>, &T),
) {
    match value {
        FactDeclaration::Unknown(reason) => {
            bytes.push(0);
            text(bytes, reason.as_str());
        }
        FactDeclaration::Known(value) => {
            bytes.push(1);
            encode(bytes, value);
        }
    }
}
pub(super) const fn status(value: FactStatus) -> u8 {
    match value {
        FactStatus::Recorded => 0,
        FactStatus::Withdrawn => 1,
    }
}
pub(super) fn procedural_time(bytes: &mut Vec<u8>, value: DeclaredProceduralTime) {
    bytes.push(match value.precision() {
        DeclaredProceduralPrecision::Unknown => 0,
        DeclaredProceduralPrecision::Date => 1,
        DeclaredProceduralPrecision::Minute => 2,
        DeclaredProceduralPrecision::Second => 3,
    });
    if let Some(date) = value.local_date() {
        civil_date(bytes, date.date());
        if let Some(hour) = value.local_hour() {
            bytes.push(hour);
        }
        if let Some(minute) = value.local_minute() {
            bytes.push(minute);
        }
        if let Some(second) = value.local_second() {
            bytes.push(second);
        }
        optional(bytes, value.offset().as_ref(), |bytes, offset| {
            bytes.extend_from_slice(&offset.whole_seconds().to_be_bytes());
        });
    }
}
pub(super) fn hearing_time(bytes: &mut Vec<u8>, value: DeclaredHearingResultTime) {
    bytes.push(match value.precision() {
        DeclaredHearingResultPrecision::Date => 0,
        DeclaredHearingResultPrecision::Instant => 1,
    });
    civil_date(bytes, value.local_date());
    if let Some(instant) = value.instant_value() {
        bytes.extend_from_slice(&[instant.hour(), instant.minute(), instant.second()]);
    }
    bytes.extend_from_slice(&value.offset().whole_seconds().to_be_bytes());
}
fn civil_date(bytes: &mut Vec<u8>, value: time::Date) {
    bytes.extend_from_slice(&(value.year() as u16).to_be_bytes());
    bytes.extend_from_slice(&[value.month() as u8, value.day()]);
}
