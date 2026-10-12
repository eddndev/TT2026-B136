use super::{DeadlineCivilCutoff, DeadlineProfileError};
use domain::{
    judicial_calendars::CivilDate, procedural_facts::FactLabel, typed_participants::Uuid,
};
use time::{OffsetDateTime, Time, UtcOffset};

/// Declared channel hours, indexed Monday through Sunday; None means no coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineWeeklyCivilCutoff {
    times: [Option<Time>; 7],
    common: DeadlineCivilCutoff,
}

impl DeadlineWeeklyCivilCutoff {
    pub fn new(
        times: [Option<Time>; 7],
        offset: UtcOffset,
        from: CivilDate,
        through: CivilDate,
        channel: FactLabel,
        reference_id: Uuid,
    ) -> Result<Self, DeadlineProfileError> {
        let mut common = None;
        for time in times.iter().flatten() {
            // Validate every hour against the same offset, bounds and precision.
            common = Some(DeadlineCivilCutoff::new(
                *time,
                offset,
                from,
                through,
                channel.clone(),
                reference_id,
            )?);
        }
        Ok(Self {
            times,
            common: common.ok_or(DeadlineProfileError::Invalid("completion.weekly_cutoff"))?,
        })
    }
    pub const fn times(&self) -> &[Option<Time>; 7] {
        &self.times
    }
    pub const fn offset(&self) -> UtcOffset {
        self.common.offset()
    }
    pub const fn from(&self) -> CivilDate {
        self.common.from()
    }
    pub const fn through(&self) -> CivilDate {
        self.common.through()
    }
    pub fn channel(&self) -> &FactLabel {
        self.common.channel()
    }
    pub const fn reference_id(&self) -> Uuid {
        self.common.reference_id()
    }
    pub fn at(&self, date: CivilDate) -> Option<OffsetDateTime> {
        if date < self.from() || date > self.through() {
            return None;
        }
        let time = self.times[usize::from(date.date().weekday().number_days_from_monday())]?;
        Some(
            date.date()
                .with_time(time)
                .assume_offset(self.offset())
                .to_offset(UtcOffset::UTC),
        )
    }
}
