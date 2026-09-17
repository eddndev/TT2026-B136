use super::{ArithmeticBlock, ArithmeticOutcome, ArithmeticTraceStep};
use crate::judicial_calendars::CivilDate;
use std::num::NonZeroU32;
use time::{Date, Month};

pub(super) fn natural_days(
    first_included: CivilDate,
    quantity: NonZeroU32,
) -> (ArithmeticOutcome, ArithmeticTraceStep) {
    let ordinal = i64::from(first_included.days_since_epoch()) + i64::from(quantity.get()) - 1;
    let candidate = i32::try_from(ordinal)
        .ok()
        .and_then(|value| CivilDate::from_days_since_epoch(value).ok());
    let outcome = candidate.map_or(
        ArithmeticOutcome::Blocked(ArithmeticBlock::DateRangeExhausted),
        |date| ArithmeticOutcome::CivilCandidate { date },
    );
    (
        outcome,
        ArithmeticTraceStep::NaturalDays {
            first_included,
            quantity,
            candidate,
        },
    )
}

pub(super) fn civil_months(
    anchor: CivilDate,
    quantity: NonZeroU32,
) -> (ArithmeticOutcome, ArithmeticTraceStep) {
    // At most u32::MAX months plus eleven; the resulting year fits in u32.
    let shifted = u64::from(anchor.date().month() as u8 - 1) + u64::from(quantity.get());
    let target_year = anchor.date().year() as u32 + (shifted / 12) as u32;
    let target_month = (shifted % 12 + 1) as u8;
    let requested_day = anchor.date().day();
    let candidate = if target_year <= 9999 {
        Month::try_from(target_month)
            .ok()
            .and_then(|month| {
                Date::from_calendar_date(target_year as i32, month, requested_day).ok()
            })
            .and_then(|date| CivilDate::from_date(date).ok())
    } else {
        None
    };
    let outcome = match candidate {
        Some(date) => ArithmeticOutcome::CivilCandidate { date },
        None if target_year > 9999 => {
            ArithmeticOutcome::Blocked(ArithmeticBlock::DateRangeExhausted)
        }
        None => ArithmeticOutcome::Blocked(ArithmeticBlock::MissingHomologousDay {
            year: target_year,
            month: target_month,
            requested_day,
        }),
    };
    (
        outcome,
        ArithmeticTraceStep::CivilMonths {
            anchor,
            quantity,
            target_year,
            target_month,
            requested_day,
            candidate,
        },
    )
}
