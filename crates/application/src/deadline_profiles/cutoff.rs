use super::DeadlineProfileError;
use domain::{
    judicial_calendars::CivilDate, procedural_facts::FactLabel, typed_participants::Uuid,
};
use time::{Time, UtcOffset};

/// A declared fixed offset over a bounded interval, independent of the trigger offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineCivilCutoff {
    time: Time,
    offset: UtcOffset,
    from: CivilDate,
    through: CivilDate,
    channel: FactLabel,
    reference_id: Uuid,
}
impl DeadlineCivilCutoff {
    pub fn new(
        time: Time,
        offset: UtcOffset,
        from: CivilDate,
        through: CivilDate,
        channel: FactLabel,
        reference_id: Uuid,
    ) -> Result<Self, DeadlineProfileError> {
        let invalid = || DeadlineProfileError::Invalid("completion.cutoff");
        let days = i64::from(through.days_since_epoch()) - i64::from(from.days_since_epoch()) + 1;
        let seconds = offset.whole_seconds();
        if time.nanosecond() != 0
            || seconds % 60 != 0
            || seconds.abs() > 14 * 3600
            || !(1..=1096).contains(&days)
        {
            return Err(invalid());
        }
        for date in [from, through] {
            date.date()
                .with_time(time)
                .assume_offset(offset)
                .checked_to_offset(UtcOffset::UTC)
                .filter(|utc| (1..=9999).contains(&utc.year()))
                .ok_or_else(invalid)?;
        }
        Ok(Self {
            time,
            offset,
            from,
            through,
            channel,
            reference_id,
        })
    }
    pub const fn time(&self) -> Time {
        self.time
    }
    pub const fn offset(&self) -> UtcOffset {
        self.offset
    }
    pub const fn from(&self) -> CivilDate {
        self.from
    }
    pub const fn through(&self) -> CivilDate {
        self.through
    }
    pub fn channel(&self) -> &FactLabel {
        &self.channel
    }
    pub const fn reference_id(&self) -> Uuid {
        self.reference_id
    }
}
