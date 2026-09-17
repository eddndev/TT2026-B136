use super::super::{
    invalid, DeadlineArithmeticRecord, DeadlineDayCountRecord, DeadlineTraceRecord,
};
use crate::ApplicationError;
use domain::{
    deadline_arithmetic::{
        ArithmeticBlock, ArithmeticOutcome, ArithmeticRule, DayBasis, FinalDayPolicy,
    },
    deadline_days::CivilDayCountOutcome,
    judicial_calendars::CivilDate,
};

/// Compares repeated operands and terminal results without calculating a new date.
/// Historical candidates may differ from the current arithmetic implementation,
/// but one captured result cannot contradict its own trace.
pub(super) fn validate(value: &DeadlineArithmeticRecord) -> Result<(), ApplicationError> {
    let Some(first) = value.trace.first() else {
        return if matches!(value.outcome, ArithmeticOutcome::Blocked(_)) {
            // Missing input and early civil overflow legitimately have no step.
            Ok(())
        } else {
            Err(invalid("successful arithmetic has no terminal trace"))
        };
    };
    let same_operands = match (value.rule, first) {
        (
            ArithmeticRule::Days {
                quantity,
                basis: DayBasis::Natural,
                ..
            },
            DeadlineTraceRecord::NaturalDays {
                quantity: captured, ..
            },
        )
        | (
            ArithmeticRule::CivilMonths { quantity, .. },
            DeadlineTraceRecord::CivilMonths {
                quantity: captured, ..
            },
        )
        | (
            ArithmeticRule::ElapsedHours { quantity },
            DeadlineTraceRecord::ElapsedHours {
                quantity: captured, ..
            },
        ) => quantity == *captured,
        (
            ArithmeticRule::Days {
                quantity,
                basis: DayBasis::CalendarCountable,
                ..
            },
            DeadlineTraceRecord::CountedDays(count),
        ) => quantity == count.quantity,
        _ => false,
    };
    if !same_operands {
        return Err(invalid("trace family or quantity differs from its rule"));
    }
    let next_countable = matches!(
        value.rule,
        ArithmeticRule::Days {
            final_day: FinalDayPolicy::NextCountable,
            ..
        } | ArithmeticRule::CivilMonths {
            final_day: FinalDayPolicy::NextCountable,
            ..
        }
    );
    let coherent = match value.trace.as_slice() {
        [first] => {
            terminal_matches(first, value.outcome)
                && !(next_countable && civil_candidate(first).is_some())
        }
        [first, DeadlineTraceRecord::FinalDay(last)] => {
            next_countable
                && last.quantity.get() == 1
                && civil_candidate(first) == Some(last.first_included)
                && calendar_outcome(last) == value.outcome
        }
        _ => false,
    };
    if !coherent {
        return Err(invalid("terminal trace and arithmetic result disagree"));
    }
    Ok(())
}

fn civil_candidate(step: &DeadlineTraceRecord) -> Option<CivilDate> {
    match step {
        DeadlineTraceRecord::NaturalDays { candidate, .. }
        | DeadlineTraceRecord::CivilMonths { candidate, .. } => *candidate,
        DeadlineTraceRecord::CountedDays(count) => match count.outcome {
            CivilDayCountOutcome::Candidate { date } => Some(date),
            _ => None,
        },
        _ => None,
    }
}
fn terminal_matches(step: &DeadlineTraceRecord, outcome: ArithmeticOutcome) -> bool {
    match (step, outcome) {
        (
            DeadlineTraceRecord::NaturalDays {
                candidate: Some(candidate),
                ..
            }
            | DeadlineTraceRecord::CivilMonths {
                candidate: Some(candidate),
                ..
            },
            ArithmeticOutcome::CivilCandidate { date },
        ) => *candidate == date,
        (
            DeadlineTraceRecord::ElapsedHours {
                candidate: Some(candidate),
                ..
            },
            ArithmeticOutcome::InstantCandidate { instant },
        ) => *candidate == instant,
        (
            DeadlineTraceRecord::NaturalDays {
                candidate: None, ..
            }
            | DeadlineTraceRecord::CivilMonths {
                candidate: None, ..
            }
            | DeadlineTraceRecord::ElapsedHours {
                candidate: None, ..
            },
            ArithmeticOutcome::Blocked(ArithmeticBlock::DateRangeExhausted),
        ) => true,
        (
            DeadlineTraceRecord::CivilMonths {
                candidate: None,
                target_year,
                target_month,
                requested_day,
                ..
            },
            ArithmeticOutcome::Blocked(ArithmeticBlock::MissingHomologousDay {
                year,
                month,
                requested_day: day,
            }),
        ) => *target_year == year && *target_month == month && *requested_day == day,
        (DeadlineTraceRecord::CountedDays(count), outcome) => calendar_outcome(count) == outcome,
        _ => false,
    }
}
fn calendar_outcome(count: &DeadlineDayCountRecord) -> ArithmeticOutcome {
    match count.outcome {
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
