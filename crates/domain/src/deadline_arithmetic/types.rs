use crate::{
    deadline_days::CivilDayCount,
    judicial_calendars::CivilDate,
    procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime},
};
use std::num::NonZeroU32;
use time::OffsetDateTime;

/// Explicit inclusion of the anchor's civil date for a daily count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayInclusion {
    OnAnchor,
    AfterAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayBasis {
    Natural,
    CalendarCountable,
}

/// Applied only to a civil candidate, never to an elapsed-hours instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalDayPolicy {
    Preserve,
    NextCountable,
}

/// Mathematical operands; not a legal profile or a finding of applicability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticRule {
    Days {
        quantity: NonZeroU32,
        inclusion: DayInclusion,
        basis: DayBasis,
        final_day: FinalDayPolicy,
    },
    CivilMonths {
        quantity: NonZeroU32,
        final_day: FinalDayPolicy,
    },
    ElapsedHours {
        quantity: NonZeroU32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticBlock {
    UnknownAnchor,
    InsufficientPrecision {
        observed: DeclaredProceduralPrecision,
    },
    MissingOffset,
    MissingCalendar,
    DateRangeExhausted,
    MissingHomologousDay {
        year: u32,
        month: u8,
        requested_day: u8,
    },
    UnresolvedCalendarDate {
        date: CivilDate,
    },
    OutsideCalendarCoverage {
        date: CivilDate,
    },
}

/// A calculated candidate does not establish an operational legal deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOutcome {
    CivilCandidate { date: CivilDate },
    InstantCandidate { instant: OffsetDateTime },
    Blocked(ArithmeticBlock),
}

/// Operands and intermediate results, including the calendar's exact evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArithmeticTraceStep {
    NaturalDays {
        first_included: CivilDate,
        quantity: NonZeroU32,
        candidate: Option<CivilDate>,
    },
    CivilMonths {
        anchor: CivilDate,
        quantity: NonZeroU32,
        target_year: u32,
        target_month: u8,
        requested_day: u8,
        candidate: Option<CivilDate>,
    },
    ElapsedHours {
        start: OffsetDateTime,
        quantity: NonZeroU32,
        candidate: Option<OffsetDateTime>,
    },
    CountedDays(CivilDayCount),
    FinalDay(CivilDayCount),
}

/// Preserves the declaration separately from derived dates or UTC instants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineArithmetic {
    pub(super) rule: ArithmeticRule,
    pub(super) anchor: DeclaredProceduralTime,
    pub(super) outcome: ArithmeticOutcome,
    pub(super) trace: Vec<ArithmeticTraceStep>,
}
impl DeadlineArithmetic {
    pub const fn rule(&self) -> ArithmeticRule {
        self.rule
    }
    pub const fn anchor(&self) -> DeclaredProceduralTime {
        self.anchor
    }
    pub const fn outcome(&self) -> &ArithmeticOutcome {
        &self.outcome
    }
    pub fn trace(&self) -> &[ArithmeticTraceStep] {
        &self.trace
    }
}
