use super::{calendar, date, example, id, input, quantity};
use application::deadline_profiles::{
    DeadlineCompletionPolicy, DeadlineExampleExpected, DeadlineProfileDefinition,
};
use domain::{
    deadline_arithmetic::{
        ArithmeticBlock, ArithmeticOutcome, ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy,
    },
    deadline_profiles::{DeadlineRuleTemplate, OrderedDeadlineUnit},
    procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime},
};
use time::{macros::datetime, UtcOffset};

#[test]
fn a_wrong_candidate_or_outcome_kind_does_not_validate_the_example() {
    for expected in [
        ArithmeticOutcome::CivilCandidate {
            date: date("2026-01-08"),
        },
        ArithmeticOutcome::InstantCandidate {
            instant: datetime!(2026-01-07 00:00 UTC),
        },
        ArithmeticOutcome::Blocked(ArithmeticBlock::UnknownAnchor),
    ] {
        let mut input = input();
        input.examples[0].expected = DeadlineExampleExpected::Arithmetic(expected);
        assert!(DeadlineProfileDefinition::new(input).is_err());
    }
}

#[test]
fn a_corpus_of_correctly_blocked_examples_still_needs_a_successful_candidate() {
    let mut only_blocked = input();
    only_blocked.examples[0].anchor = DeclaredProceduralTime::unknown();
    only_blocked.examples[0].expected = DeadlineExampleExpected::Arithmetic(
        ArithmeticOutcome::Blocked(ArithmeticBlock::UnknownAnchor),
    );
    assert!(DeadlineProfileDefinition::new(only_blocked).is_err());
    let mut mixed = input();
    let mut blocked = example(3);
    blocked.anchor = DeclaredProceduralTime::unknown();
    blocked.expected = DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::Blocked(
        ArithmeticBlock::UnknownAnchor,
    ));
    mixed.examples.push(blocked);
    assert!(DeadlineProfileDefinition::new(mixed).is_ok());
}

#[test]
fn elapsed_hours_replay_the_explicit_offset_and_keep_precision_failures() {
    let mut input = input();
    input.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::ElapsedHours {
        quantity: quantity(2),
    });
    input.completion = DeadlineCompletionPolicy::ArithmeticInstant;
    input.examples[0].anchor = DeclaredProceduralTime::second(
        date("2026-01-06"),
        12,
        30,
        7,
        Some(UtcOffset::from_hms(2, 0, 0).unwrap()),
    )
    .unwrap();
    input.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate {
            instant: datetime!(2026-01-06 12:30:07 UTC),
        });
    let mut imprecise = example(3);
    imprecise.anchor = DeclaredProceduralTime::minute(date("2026-01-06"), 12, 30, None).unwrap();
    imprecise.expected = DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::Blocked(
        ArithmeticBlock::InsufficientPrecision {
            observed: DeclaredProceduralPrecision::Minute,
        },
    ));
    input.examples.push(imprecise);
    assert!(DeadlineProfileDefinition::new(input).is_ok());
}

#[test]
fn hourly_templates_reject_civil_completion_before_accepting_the_corpus() {
    for template in [
        DeadlineRuleTemplate::Fixed(ArithmeticRule::ElapsedHours {
            quantity: quantity(2),
        }),
        DeadlineRuleTemplate::Ordered {
            unit: OrderedDeadlineUnit::ElapsedHours,
            maximum: None,
        },
    ] {
        let mut input = input();
        input.template = template;
        input.examples[0].ordered_quantity = match template {
            DeadlineRuleTemplate::Ordered { .. } => Some(quantity(2)),
            _ => None,
        };
        input.examples[0].anchor =
            DeclaredProceduralTime::second(date("2026-01-06"), 0, 0, 0, Some(UtcOffset::UTC))
                .unwrap();
        input.examples[0].expected =
            DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate {
                instant: datetime!(2026-01-06 02:00 UTC),
            });
        assert!(DeadlineProfileDefinition::new(input).is_err());
    }
}

#[test]
fn countable_day_examples_replay_the_selected_calendar_and_not_a_weekday_guess() {
    let mut input = input();
    input.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::Days {
        quantity: quantity(2),
        inclusion: DayInclusion::OnAnchor,
        basis: DayBasis::CalendarCountable,
        final_day: FinalDayPolicy::Preserve,
    });
    input.examples[0].anchor = DeclaredProceduralTime::date(date("2026-01-09"), None).unwrap();
    input.examples[0].calendar = Some(calendar());
    input.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::CivilCandidate {
            date: date("2026-01-12"),
        });
    let mut missing_calendar = input.examples[0].clone();
    missing_calendar.id = id(3);
    missing_calendar.calendar = None;
    missing_calendar.expected = DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::Blocked(
        ArithmeticBlock::MissingCalendar,
    ));
    input.examples.push(missing_calendar);
    assert!(DeadlineProfileDefinition::new(input).is_ok());
}

#[test]
fn an_unused_calendar_is_retained_in_the_example_without_changing_arithmetic() {
    let mut input = input();
    let calendar = calendar();
    input.examples[0].calendar = Some(calendar.clone());
    let profile = DeadlineProfileDefinition::new(input).unwrap();
    assert_eq!(profile.examples()[0].calendar.as_ref(), Some(&calendar));
}

#[test]
fn synthetic_future_dates_are_not_rejected_by_an_implicit_clock() {
    let mut input = input();
    input.examples[0].anchor = DeclaredProceduralTime::date(date("9998-01-06"), None).unwrap();
    input.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::CivilCandidate {
            date: date("9998-01-07"),
        });
    assert!(DeadlineProfileDefinition::new(input).is_ok());
}
