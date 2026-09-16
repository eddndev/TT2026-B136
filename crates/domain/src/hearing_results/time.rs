use crate::DomainError;
use ::time::{Date, OffsetDateTime, Time, UtcOffset};

/// Precision reported by the operator; it is never inferred from the capture clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredHearingResultPrecision {
    Date,
    Instant,
}

#[derive(Debug, Clone, Copy)]
enum Declaration {
    Date(Date, UtcOffset),
    Instant(OffsetDateTime),
}

/// An entire local day or whole-second instant with its original minute offset.
#[derive(Debug, Clone, Copy)]
pub struct DeclaredHearingResultTime(Declaration);
impl DeclaredHearingResultTime {
    pub fn date(date: Date, offset: UtcOffset) -> Result<Self, DomainError> {
        validate_offset(offset)?;
        utc_in_range(date.midnight().assume_offset(offset))?;
        utc_in_range(date.with_time(end_of_day()).assume_offset(offset))?;
        Ok(Self(Declaration::Date(date, offset)))
    }
    pub fn instant(value: OffsetDateTime) -> Result<Self, DomainError> {
        validate_offset(value.offset())?;
        if value.nanosecond() != 0 {
            return Err(invalid());
        }
        utc_in_range(value)?;
        Ok(Self(Declaration::Instant(value)))
    }
    pub const fn precision(self) -> DeclaredHearingResultPrecision {
        match self.0 {
            Declaration::Date(..) => DeclaredHearingResultPrecision::Date,
            Declaration::Instant(_) => DeclaredHearingResultPrecision::Instant,
        }
    }
    pub const fn local_date(self) -> Date {
        match self.0 {
            Declaration::Date(date, _) => date,
            Declaration::Instant(value) => value.date(),
        }
    }
    pub const fn offset(self) -> UtcOffset {
        match self.0 {
            Declaration::Date(_, offset) => offset,
            Declaration::Instant(value) => value.offset(),
        }
    }
    pub const fn instant_value(self) -> Option<OffsetDateTime> {
        match self.0 {
            Declaration::Date(..) => None,
            Declaration::Instant(value) => Some(value),
        }
    }
    /// Inclusive bound for comparisons; it does not assert a known time of day.
    pub fn lower_bound(self) -> OffsetDateTime {
        match self.0 {
            Declaration::Date(date, offset) => date.midnight().assume_offset(offset),
            Declaration::Instant(value) => value,
        }
        .to_offset(UtcOffset::UTC)
    }
    /// Inclusive end of the declared day, or the exact instant for instant precision.
    pub fn upper_bound(self) -> OffsetDateTime {
        match self.0 {
            Declaration::Date(date, offset) => date.with_time(end_of_day()).assume_offset(offset),
            Declaration::Instant(value) => value,
        }
        .to_offset(UtcOffset::UTC)
    }
}
impl PartialEq for DeclaredHearingResultTime {
    fn eq(&self, other: &Self) -> bool {
        self.precision() == other.precision()
            && self.offset() == other.offset()
            && self.local_date() == other.local_date()
            && self.instant_value() == other.instant_value()
    }
}
impl Eq for DeclaredHearingResultTime {}
fn end_of_day() -> Time {
    Time::from_hms_nano(23, 59, 59, 999_999_999).expect("valid final nanosecond of a day")
}
fn invalid() -> DomainError {
    DomainError::InvalidHearingResultValue("event_time")
}
fn validate_offset(offset: UtcOffset) -> Result<(), DomainError> {
    let seconds = offset.whole_seconds();
    if seconds % 60 != 0 || seconds.abs() > 14 * 3600 {
        return Err(invalid());
    }
    Ok(())
}
fn utc_in_range(value: OffsetDateTime) -> Result<OffsetDateTime, DomainError> {
    value
        .checked_to_offset(UtcOffset::UTC)
        .filter(|utc| (1..=9999).contains(&utc.year()) && (1..=9999).contains(&value.year()))
        .ok_or_else(invalid)
}
