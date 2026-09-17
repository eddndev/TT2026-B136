//! Explicit temporal arithmetic without selecting a legal rule or trigger.

mod civil;
mod hours;
mod types;

use crate::{
    deadline_days::{count_calendar_days, CivilDayCountOutcome},
    judicial_calendars::{CivilDate, JudicialCalendarValues},
    procedural_time::DeclaredProceduralTime,
};
use std::num::NonZeroU32;
pub use types::{
    ArithmeticBlock, ArithmeticOutcome, ArithmeticRule, ArithmeticTraceStep, DayBasis,
    DayInclusion, DeadlineArithmetic, FinalDayPolicy,
};

/// Derive a mathematical candidate or a typed block from explicit operands.
///
/// The original declaration is preserved. No clock, legal inference, database,
/// or time-zone lookup is used. See docs/deadline-arithmetic.md.
pub fn evaluate_deadline_arithmetic(
    rule: ArithmeticRule,
    anchor: DeclaredProceduralTime,
    calendar: Option<&JudicialCalendarValues>,
) -> DeadlineArithmetic {
    let (outcome, trace) = match rule {
        ArithmeticRule::ElapsedHours { quantity } => hours::elapsed_hours(anchor, quantity),
        _ => evaluate_civil(rule, anchor, calendar),
    };
    DeadlineArithmetic {
        rule,
        anchor,
        outcome,
        trace,
    }
}

fn evaluate_civil(
    rule: ArithmeticRule,
    anchor: DeclaredProceduralTime,
    calendar: Option<&JudicialCalendarValues>,
) -> (ArithmeticOutcome, Vec<ArithmeticTraceStep>) {
    let blocked = |cause| (ArithmeticOutcome::Blocked(cause), vec![]);
    let Some(date) = anchor.local_date() else {
        return blocked(ArithmeticBlock::UnknownAnchor);
    };
    let final_day = match rule {
        ArithmeticRule::Days { final_day, .. } | ArithmeticRule::CivilMonths { final_day, .. } => {
            final_day
        }
        ArithmeticRule::ElapsedHours { .. } => unreachable!("hours are evaluated separately"),
    };
    let needs_calendar = final_day == FinalDayPolicy::NextCountable
        || matches!(
            rule,
            ArithmeticRule::Days {
                basis: DayBasis::CalendarCountable,
                ..
            }
        );
    if needs_calendar && calendar.is_none() {
        return blocked(ArithmeticBlock::MissingCalendar);
    }
    let (outcome, step) = match rule {
        ArithmeticRule::Days {
            quantity,
            inclusion,
            basis,
            ..
        } => {
            let first = match inclusion {
                DayInclusion::OnAnchor => date,
                DayInclusion::AfterAnchor => {
                    let Some(next) = date
                        .date()
                        .next_day()
                        .and_then(|value| CivilDate::from_date(value).ok())
                    else {
                        return blocked(ArithmeticBlock::DateRangeExhausted);
                    };
                    next
                }
            };
            match basis {
                DayBasis::Natural => civil::natural_days(first, quantity),
                DayBasis::CalendarCountable => {
                    let count = count_calendar_days(
                        calendar.expect("calendar was checked"),
                        first,
                        quantity,
                    );
                    (
                        calendar_outcome(count.outcome()),
                        ArithmeticTraceStep::CountedDays(count),
                    )
                }
            }
        }
        ArithmeticRule::CivilMonths { quantity, .. } => civil::civil_months(date, quantity),
        ArithmeticRule::ElapsedHours { .. } => unreachable!("hours are evaluated separately"),
    };
    let mut trace = vec![step];
    if let ArithmeticOutcome::CivilCandidate { date } = outcome {
        if final_day == FinalDayPolicy::NextCountable {
            let count = count_calendar_days(
                calendar.expect("calendar was checked"),
                date,
                NonZeroU32::MIN,
            );
            let outcome = calendar_outcome(count.outcome());
            trace.push(ArithmeticTraceStep::FinalDay(count));
            return (outcome, trace);
        }
    }
    (outcome, trace)
}

fn calendar_outcome(outcome: CivilDayCountOutcome) -> ArithmeticOutcome {
    match outcome {
        CivilDayCountOutcome::Candidate { date } => ArithmeticOutcome::CivilCandidate { date },
        CivilDayCountOutcome::Unresolved { date } => {
            ArithmeticOutcome::Blocked(ArithmeticBlock::UnresolvedCalendarDate { date })
        }
        CivilDayCountOutcome::OutsideCoverage { date } => {
            ArithmeticOutcome::Blocked(ArithmeticBlock::OutsideCalendarCoverage { date })
        }
        CivilDayCountOutcome::DateRangeExhausted { .. } => {
            ArithmeticOutcome::Blocked(ArithmeticBlock::DateRangeExhausted)
        }
    }
}
