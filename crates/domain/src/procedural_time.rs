//! Precision and original components of a declared procedural time.

use crate::{judicial_calendars::CivilDate, DomainError};
use time::{OffsetDateTime, Time, UtcOffset};

/// Temporal granularity expressly declared, independent of the capture clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredProceduralPrecision {
    Unknown,
    Date,
    Minute,
    Second,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Declaration {
    Unknown,
    Date(CivilDate, Option<UtcOffset>),
    Minute(CivilDate, Time, Option<UtcOffset>),
    Second(CivilDate, Time, Option<UtcOffset>),
}

/// Preserves missing components and a declared fixed offset without inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclaredProceduralTime(Declaration);

impl DeclaredProceduralTime {
    pub const fn unknown() -> Self {
        Self(Declaration::Unknown)
    }

    pub fn date(date: CivilDate, offset: Option<UtcOffset>) -> Result<Self, DomainError> {
        let end = Time::from_hms_nano(23, 59, 59, 999_999_999).map_err(|_| invalid())?;
        validate_interval(date, Time::MIDNIGHT, end, offset)?;
        Ok(Self(Declaration::Date(date, offset)))
    }

    pub fn minute(
        date: CivilDate,
        hour: u8,
        minute: u8,
        offset: Option<UtcOffset>,
    ) -> Result<Self, DomainError> {
        let start = Time::from_hms(hour, minute, 0).map_err(|_| invalid())?;
        let end = Time::from_hms_nano(hour, minute, 59, 999_999_999).map_err(|_| invalid())?;
        validate_interval(date, start, end, offset)?;
        Ok(Self(Declaration::Minute(date, start, offset)))
    }

    pub fn second(
        date: CivilDate,
        hour: u8,
        minute: u8,
        second: u8,
        offset: Option<UtcOffset>,
    ) -> Result<Self, DomainError> {
        let time = Time::from_hms(hour, minute, second).map_err(|_| invalid())?;
        validate_interval(date, time, time, offset)?;
        Ok(Self(Declaration::Second(date, time, offset)))
    }

    pub const fn precision(self) -> DeclaredProceduralPrecision {
        match self.0 {
            Declaration::Unknown => DeclaredProceduralPrecision::Unknown,
            Declaration::Date(..) => DeclaredProceduralPrecision::Date,
            Declaration::Minute(..) => DeclaredProceduralPrecision::Minute,
            Declaration::Second(..) => DeclaredProceduralPrecision::Second,
        }
    }

    pub const fn local_date(self) -> Option<CivilDate> {
        match self.0 {
            Declaration::Unknown => None,
            Declaration::Date(date, _)
            | Declaration::Minute(date, ..)
            | Declaration::Second(date, ..) => Some(date),
        }
    }

    pub const fn local_hour(self) -> Option<u8> {
        match self.0 {
            Declaration::Minute(_, time, _) | Declaration::Second(_, time, _) => Some(time.hour()),
            _ => None,
        }
    }

    pub const fn local_minute(self) -> Option<u8> {
        match self.0 {
            Declaration::Minute(_, time, _) | Declaration::Second(_, time, _) => {
                Some(time.minute())
            }
            _ => None,
        }
    }

    pub const fn local_second(self) -> Option<u8> {
        match self.0 {
            Declaration::Second(_, time, _) => Some(time.second()),
            _ => None,
        }
    }

    pub const fn offset(self) -> Option<UtcOffset> {
        match self.0 {
            Declaration::Unknown => None,
            Declaration::Date(_, offset)
            | Declaration::Minute(_, _, offset)
            | Declaration::Second(_, _, offset) => offset,
        }
    }

    /// Only an explicitly declared second and offset identify a modeled instant.
    pub fn instant_value(self) -> Option<OffsetDateTime> {
        match self.0 {
            Declaration::Second(date, time, Some(offset)) => {
                Some(date.date().with_time(time).assume_offset(offset))
            }
            _ => None,
        }
    }
}

fn invalid() -> DomainError {
    DomainError::InvalidDeclaredProceduralTime
}

// Endpoints only validate representability; they are not declared event times.
fn validate_interval(
    date: CivilDate,
    start: Time,
    end: Time,
    offset: Option<UtcOffset>,
) -> Result<(), DomainError> {
    let Some(offset) = offset else {
        return Ok(());
    };
    let seconds = offset.whole_seconds();
    if seconds % 60 != 0 || seconds.abs() > 14 * 3600 {
        return Err(invalid());
    }
    for time in [start, end] {
        date.date()
            .with_time(time)
            .assume_offset(offset)
            .checked_to_offset(UtcOffset::UTC)
            .filter(|utc| (1..=9999).contains(&utc.year()))
            .ok_or_else(invalid)?;
    }
    Ok(())
}
