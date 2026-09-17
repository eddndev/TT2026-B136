use crate::DomainError;
use std::{fmt, str::FromStr};
use time::{Date, Month};

/// Gregorian civil date without any clock, offset or time-zone conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilDate(Date);
impl CivilDate {
    pub fn from_date(value: Date) -> Result<Self, DomainError> {
        if !(1..=9999).contains(&value.year()) {
            return Err(invalid());
        }
        Ok(Self(value))
    }
    pub const fn date(self) -> Date {
        self.0
    }
    pub const fn days_since_epoch(self) -> i32 {
        self.0.to_julian_day() - 2440588
    }
    pub fn from_days_since_epoch(value: i32) -> Result<Self, DomainError> {
        if !(-719162..=2932896).contains(&value) {
            return Err(invalid());
        }
        Self::from_date(Date::from_julian_day(value + 2440588).map_err(|_| invalid())?)
    }
    pub const fn weekday(self) -> u8 {
        self.0.weekday().number_from_monday()
    }
}
impl FromStr for CivilDate {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let b = value.as_bytes();
        if b.len() != 10
            || b[4] != b'-'
            || b[7] != b'-'
            || b.iter()
                .enumerate()
                .any(|(i, c)| i != 4 && i != 7 && !c.is_ascii_digit())
        {
            return Err(invalid());
        }
        let year = value[..4].parse::<i32>().map_err(|_| invalid())?;
        let month = value[5..7].parse::<u8>().map_err(|_| invalid())?;
        let day = value[8..].parse::<u8>().map_err(|_| invalid())?;
        Self::from_date(
            Date::from_calendar_date(year, Month::try_from(month).map_err(|_| invalid())?, day)
                .map_err(|_| invalid())?,
        )
    }
}
impl fmt::Display for CivilDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}",
            self.0.year(),
            self.0.month() as u8,
            self.0.day()
        )
    }
}
fn invalid() -> DomainError {
    DomainError::InvalidJudicialCalendarValue("date")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudicialCalendarCoverage {
    from: CivilDate,
    through: CivilDate,
}
impl JudicialCalendarCoverage {
    pub fn new(from: CivilDate, through: CivilDate) -> Result<Self, DomainError> {
        let length = through.days_since_epoch() - from.days_since_epoch() + 1;
        if !(1..=1096).contains(&length) {
            return Err(DomainError::InvalidJudicialCalendarValue("coverage"));
        }
        Ok(Self { from, through })
    }
    pub const fn from(self) -> CivilDate {
        self.from
    }
    pub const fn through(self) -> CivilDate {
        self.through
    }
    pub fn contains(self, date: CivilDate) -> bool {
        self.from <= date && date <= self.through
    }
}
