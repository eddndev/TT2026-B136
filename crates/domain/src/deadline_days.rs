//! Pure civil-day counting over exact calendar values, without legal effects.

use crate::judicial_calendars::{
    CivilDate, JudicialCalendarClassification, JudicialCalendarDay, JudicialCalendarValues,
};
use std::num::NonZeroU32;

/// A civil candidate or the first reason the supplied values cannot produce one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CivilDayCountOutcome {
    Candidate { date: CivilDate },
    Unresolved { date: CivilDate },
    OutsideCoverage { date: CivilDate },
    DateRangeExhausted { after: CivilDate },
}

/// Exact calendar classification and count after inspecting that civil day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CivilDayCountStep {
    day: JudicialCalendarDay,
    accumulated: u32,
}
impl CivilDayCountStep {
    pub const fn day(&self) -> &JudicialCalendarDay {
        &self.day
    }
    pub const fn accumulated(&self) -> u32 {
        self.accumulated
    }
}

/// Mathematical result and contiguous trace; never an operational deadline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CivilDayCount {
    first_included: CivilDate,
    quantity: NonZeroU32,
    outcome: CivilDayCountOutcome,
    trace: Vec<CivilDayCountStep>,
}
impl CivilDayCount {
    pub const fn first_included(&self) -> CivilDate {
        self.first_included
    }
    pub const fn quantity(&self) -> NonZeroU32 {
        self.quantity
    }
    pub const fn outcome(&self) -> CivilDayCountOutcome {
        self.outcome
    }
    pub fn trace(&self) -> &[CivilDayCountStep] {
        &self.trace
    }
    pub fn accumulated(&self) -> u32 {
        self.trace.last().map_or(0, CivilDayCountStep::accumulated)
    }
}

/// Count from an included civil date using only the supplied calendar values.
///
/// Classification stops on the candidate or the first unresolved/outside day.
/// Valid coverage contains at most 1096 days, so the trace contains at most
/// 1097 steps including a possible first day outside coverage. Quantity never
/// determines allocation size. The caller must bind calendar identity and any
/// legal interpretation separately; see docs/deadline-day-counting.md.
pub fn count_calendar_days(
    calendar: &JudicialCalendarValues,
    first_included: CivilDate,
    quantity: NonZeroU32,
) -> CivilDayCount {
    let mut date = first_included;
    let mut accumulated = 0;
    let mut trace = Vec::new();
    let outcome = loop {
        let day = calendar.classify(date);
        let stop = match day.classification() {
            Some(JudicialCalendarClassification::Countable) => {
                accumulated += 1;
                (accumulated == quantity.get()).then_some(CivilDayCountOutcome::Candidate { date })
            }
            Some(JudicialCalendarClassification::Excluded) => None,
            Some(JudicialCalendarClassification::Unresolved) => {
                Some(CivilDayCountOutcome::Unresolved { date })
            }
            None => Some(CivilDayCountOutcome::OutsideCoverage { date }),
        };
        trace.push(CivilDayCountStep { day, accumulated });
        if let Some(outcome) = stop {
            break outcome;
        }
        let Some(next) = date
            .date()
            .next_day()
            .and_then(|next| CivilDate::from_date(next).ok())
        else {
            break CivilDayCountOutcome::DateRangeExhausted { after: date };
        };
        date = next;
    };
    CivilDayCount {
        first_included,
        quantity,
        outcome,
        trace,
    }
}
