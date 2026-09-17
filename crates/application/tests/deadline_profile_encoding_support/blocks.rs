use super::deadline_profile_encoding_support::*;
use application::deadline_profiles::*;
use domain::{
    deadline_arithmetic::{
        ArithmeticBlock, ArithmeticOutcome, ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy,
    },
    deadline_profiles::{DeadlineRuleBlock, DeadlineRuleTemplate, OrderedDeadlineUnit},
    judicial_calendars::JudicialCalendarClassification,
    procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime},
};
use time::UtcOffset;

fn anchor(value: &str) -> DeclaredProceduralTime {
    DeclaredProceduralTime::date(date(value), None).unwrap()
}
fn hourly() -> DeadlineProfileDefinitionInput {
    let mut values = input();
    values.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::ElapsedHours {
        quantity: quantity(1),
    });
    values.completion = DeadlineCompletionPolicy::ArithmeticInstant;
    values.examples[0].anchor =
        DeclaredProceduralTime::second(date("2026-01-06"), 0, 0, 0, Some(UtcOffset::UTC)).unwrap();
    recalculate(&mut values);
    values
}

#[test]
fn hourly_corpus_preserves_unknown_precision_missing_offset_and_overflow_blocks() {
    let mut values = hourly();
    for (at, block) in [
        (
            DeclaredProceduralTime::unknown(),
            ArithmeticBlock::UnknownAnchor,
        ),
        (
            anchor("2026-01-06"),
            ArithmeticBlock::InsufficientPrecision {
                observed: DeclaredProceduralPrecision::Date,
            },
        ),
        (
            DeclaredProceduralTime::minute(date("2026-01-06"), 0, 0, Some(UtcOffset::UTC)).unwrap(),
            ArithmeticBlock::InsufficientPrecision {
                observed: DeclaredProceduralPrecision::Minute,
            },
        ),
        (
            DeclaredProceduralTime::second(date("2026-01-06"), 0, 0, 0, None).unwrap(),
            ArithmeticBlock::MissingOffset,
        ),
        (
            DeclaredProceduralTime::second(date("9999-12-31"), 23, 59, 59, Some(UtcOffset::UTC))
                .unwrap(),
            ArithmeticBlock::DateRangeExhausted,
        ),
    ] {
        let mut example = values.examples[0].clone();
        example.id = id(values.examples.len() as u128);
        example.anchor = at;
        example.expected = DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::Blocked(block));
        values.examples.push(example);
    }
    roundtrip(values);
}

#[test]
fn monthly_corpus_preserves_missing_homologous_day_payload_and_year_exhaustion() {
    let mut values = input();
    for (at, block) in [
        (
            "2026-01-31",
            ArithmeticBlock::MissingHomologousDay {
                year: 2026,
                month: 2,
                requested_day: 31,
            },
        ),
        ("9999-12-01", ArithmeticBlock::DateRangeExhausted),
    ] {
        let mut example = values.examples[0].clone();
        example.id = id(values.examples.len() as u128);
        example.anchor = anchor(at);
        example.expected = DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::Blocked(block));
        values.examples.push(example);
    }
    roundtrip(values);
}

#[test]
fn countable_day_corpus_preserves_missing_unresolved_and_outside_coverage_dates() {
    let mut values = input();
    values.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::Days {
        quantity: quantity(1),
        inclusion: DayInclusion::OnAnchor,
        basis: DayBasis::CalendarCountable,
        final_day: FinalDayPolicy::Preserve,
    });
    values.examples[0].anchor = anchor("2026-01-06");
    values.examples[0].calendar = Some(calendar(JudicialCalendarClassification::Countable));
    recalculate(&mut values);
    for (at, calendar, block) in [
        ("2026-01-06", None, ArithmeticBlock::MissingCalendar),
        (
            "2026-01-06",
            Some(calendar(JudicialCalendarClassification::Unresolved)),
            ArithmeticBlock::UnresolvedCalendarDate {
                date: date("2026-01-06"),
            },
        ),
        (
            "2027-01-06",
            Some(calendar(JudicialCalendarClassification::Countable)),
            ArithmeticBlock::OutsideCalendarCoverage {
                date: date("2027-01-06"),
            },
        ),
    ] {
        let mut example = values.examples[0].clone();
        example.id = id(values.examples.len() as u128);
        example.anchor = anchor(at);
        example.calendar = calendar;
        example.expected = DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::Blocked(block));
        values.examples.push(example);
    }
    roundtrip(values);
}

#[test]
fn ordered_corpus_keeps_missing_and_excess_operands_distinct_from_fixed_unexpected_quantity() {
    let mut ordered = input();
    ordered.template = DeadlineRuleTemplate::Ordered {
        unit: OrderedDeadlineUnit::CivilMonths {
            final_day: FinalDayPolicy::Preserve,
        },
        maximum: Some(quantity(6)),
    };
    ordered.examples[0].ordered_quantity = Some(quantity(1));
    for (supplied, block) in [
        (None, DeadlineRuleBlock::MissingOrderedQuantity),
        (
            Some(quantity(7)),
            DeadlineRuleBlock::OrderedQuantityExceedsMaximum {
                maximum: quantity(6),
                supplied: quantity(7),
            },
        ),
    ] {
        let mut example = ordered.examples[0].clone();
        example.id = id(ordered.examples.len() as u128);
        example.ordered_quantity = supplied;
        example.expected = DeadlineExampleExpected::RuleBlocked(block);
        ordered.examples.push(example);
    }
    roundtrip(ordered);
    let mut fixed = input();
    let mut unexpected = fixed.examples[0].clone();
    unexpected.id = id(1);
    unexpected.ordered_quantity = Some(quantity(1));
    unexpected.expected =
        DeadlineExampleExpected::RuleBlocked(DeadlineRuleBlock::UnexpectedOrderedQuantity);
    fixed.examples.push(unexpected);
    roundtrip(fixed);
}
