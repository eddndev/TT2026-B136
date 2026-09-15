use ::time::{Date, OffsetDateTime, Time, UtcOffset};

use crate::DomainError;

/// Precision supplied by the declarant, never inferred from a clock value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredStagePrecision {
    Date,
    Instant,
}

#[derive(Debug, Clone, Copy)]
enum Declaration {
    Date(Date, UtcOffset),
    Instant(OffsetDateTime),
}

/// A local day or exact instant with its original explicit offset.
#[derive(Debug, Clone, Copy)]
pub struct DeclaredStageTime(Declaration);

impl DeclaredStageTime {
    pub fn date(date: Date, offset: UtcOffset) -> Result<Self, DomainError> {
        validate_offset(offset)?;
        utc_in_range(date.midnight().assume_offset(offset))?;
        utc_in_range(date.with_time(end_of_day()).assume_offset(offset))?;
        Ok(Self(Declaration::Date(date, offset)))
    }

    pub fn instant(value: OffsetDateTime) -> Result<Self, DomainError> {
        validate_offset(value.offset())?;
        utc_in_range(value)?;
        Ok(Self(Declaration::Instant(value)))
    }

    pub const fn precision(self) -> DeclaredStagePrecision {
        match self.0 {
            Declaration::Date(..) => DeclaredStagePrecision::Date,
            Declaration::Instant(_) => DeclaredStagePrecision::Instant,
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

    /// Inclusive UTC lower bound; a date retains the entire declared day.
    pub fn lower_bound(self) -> OffsetDateTime {
        let value = match self.0 {
            Declaration::Date(date, offset) => date.midnight().assume_offset(offset),
            Declaration::Instant(value) => value,
        };
        value.to_offset(UtcOffset::UTC)
    }

    /// Inclusive UTC upper bound, equal to the lower bound only for instants.
    pub fn upper_bound(self) -> OffsetDateTime {
        let value = match self.0 {
            Declaration::Date(date, offset) => date.with_time(end_of_day()).assume_offset(offset),
            Declaration::Instant(value) => value,
        };
        value.to_offset(UtcOffset::UTC)
    }
}

impl PartialEq for DeclaredStageTime {
    fn eq(&self, other: &Self) -> bool {
        self.precision() == other.precision()
            && self.offset() == other.offset()
            && self.local_date() == other.local_date()
            && self.instant_value() == other.instant_value()
    }
}
impl Eq for DeclaredStageTime {}

fn end_of_day() -> Time {
    Time::from_hms_nano(23, 59, 59, 999_999_999).expect("valid inclusive day boundary")
}

fn validate_offset(offset: UtcOffset) -> Result<(), DomainError> {
    let seconds = offset.whole_seconds();
    if seconds % 60 != 0 || seconds.abs() > 14 * 60 * 60 {
        return Err(DomainError::InvalidDeclaredStageTime);
    }
    Ok(())
}

pub(super) fn utc_in_range(value: OffsetDateTime) -> Result<OffsetDateTime, DomainError> {
    let utc = value
        .checked_to_offset(UtcOffset::UTC)
        .filter(|utc| (1..=9999).contains(&utc.year()))
        .filter(|_| (1..=9999).contains(&value.year()))
        .ok_or(DomainError::InvalidDeclaredStageTime)?;
    Ok(utc)
}
