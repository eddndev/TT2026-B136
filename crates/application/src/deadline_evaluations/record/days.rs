use domain::{
    deadline_days::{CivilDayCount, CivilDayCountOutcome},
    judicial_calendars::{CivilDate, JudicialCalendarClassification, JudicialCalendarDayOrigin},
    typed_participants::Uuid,
};
use std::num::NonZeroU32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineDayCountRecord {
    pub(super) first_included: CivilDate,
    pub(super) quantity: NonZeroU32,
    pub(super) outcome: CivilDayCountOutcome,
    pub(super) trace: Vec<DeadlineDayStepRecord>,
}
impl DeadlineDayCountRecord {
    pub(super) fn capture(value: &CivilDayCount) -> Self {
        Self {
            first_included: value.first_included(),
            quantity: value.quantity(),
            outcome: value.outcome(),
            trace: value
                .trace()
                .iter()
                .map(|step| {
                    let day = step.day();
                    DeadlineDayStepRecord {
                        accumulated: step.accumulated(),
                        day: DeadlineCalendarDayRecord {
                            date: day.date(),
                            origin: day.origin(),
                            classification: day.classification(),
                            explanation: day.explanation().map(str::to_owned),
                            source_ids: day.source_ids().to_vec(),
                        },
                    }
                })
                .collect(),
        }
    }
    pub const fn first_included(&self) -> CivilDate {
        self.first_included
    }
    pub const fn quantity(&self) -> NonZeroU32 {
        self.quantity
    }
    pub const fn outcome(&self) -> CivilDayCountOutcome {
        self.outcome
    }
    pub fn trace(&self) -> &[DeadlineDayStepRecord] {
        &self.trace
    }
    pub fn accumulated(&self) -> u32 {
        self.trace.last().map_or(0, |step| step.accumulated)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineDayStepRecord {
    pub(super) day: DeadlineCalendarDayRecord,
    pub(super) accumulated: u32,
}
impl DeadlineDayStepRecord {
    pub const fn day(&self) -> &DeadlineCalendarDayRecord {
        &self.day
    }
    pub const fn accumulated(&self) -> u32 {
        self.accumulated
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineCalendarDayRecord {
    pub(super) date: CivilDate,
    pub(super) origin: Option<JudicialCalendarDayOrigin>,
    pub(super) classification: Option<JudicialCalendarClassification>,
    pub(super) explanation: Option<String>,
    pub(super) source_ids: Vec<Uuid>,
}
impl DeadlineCalendarDayRecord {
    pub const fn date(&self) -> CivilDate {
        self.date
    }
    pub const fn origin(&self) -> Option<JudicialCalendarDayOrigin> {
        self.origin
    }
    pub const fn classification(&self) -> Option<JudicialCalendarClassification> {
        self.classification
    }
    pub fn explanation(&self) -> Option<&str> {
        self.explanation.as_deref()
    }
    pub fn source_ids(&self) -> &[Uuid] {
        &self.source_ids
    }
}
