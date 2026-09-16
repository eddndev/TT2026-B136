use ::time::{OffsetDateTime, UtcOffset};

use crate::DomainError;

/// Communicated whole-second instant and its explicit original minute offset.
#[derive(Debug, Clone, Copy)]
pub struct HearingTime(OffsetDateTime);

impl HearingTime {
    pub fn new(value: OffsetDateTime) -> Result<Self, DomainError> {
        let offset = value.offset().whole_seconds();
        let utc = value.checked_to_offset(UtcOffset::UTC);
        if value.nanosecond() != 0
            || offset % 60 != 0
            || offset.abs() > 14 * 60 * 60
            || !(1..=9999).contains(&value.year())
            || !utc.is_some_and(|utc| (1..=9999).contains(&utc.year()))
        {
            return Err(DomainError::InvalidHearingValue("scheduled_at"));
        }
        Ok(Self(value))
    }

    pub const fn value(self) -> OffsetDateTime {
        self.0
    }

    pub fn utc(self) -> OffsetDateTime {
        self.0.to_offset(UtcOffset::UTC)
    }
}

impl PartialEq for HearingTime {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.0.offset() == other.0.offset()
    }
}

impl Eq for HearingTime {}
