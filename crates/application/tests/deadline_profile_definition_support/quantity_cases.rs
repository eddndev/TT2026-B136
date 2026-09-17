use super::{date, example, id, input, quantity};
use application::deadline_profiles::{
    DeadlineCompletionPolicy, DeadlineExampleExpected, DeadlineProfileDefinition,
};
use domain::{
    deadline_arithmetic::{
        ArithmeticBlock, ArithmeticOutcome, ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy,
    },
    deadline_profiles::{DeadlineRuleBlock, DeadlineRuleTemplate, OrderedDeadlineUnit},
    procedural_time::DeclaredProceduralTime,
};
use time::{macros::datetime, UtcOffset};

#[test]
fn ordered_examples_bind_the_real_quantity_and_verify_rule_block_operands() {
    let mut input = input();
    input.template = DeadlineRuleTemplate::Ordered {
        unit: OrderedDeadlineUnit::Days {
            inclusion: DayInclusion::OnAnchor,
            basis: DayBasis::Natural,
            final_day: FinalDayPolicy::Preserve,
        },
        maximum: Some(quantity(6)),
    };
    input.examples[0].ordered_quantity = Some(quantity(2));
    let mut missing = example(3);
    missing.expected =
        DeadlineExampleExpected::RuleBlocked(DeadlineRuleBlock::MissingOrderedQuantity);
    let mut excess = example(4);
    excess.ordered_quantity = Some(quantity(7));
    excess.expected =
        DeadlineExampleExpected::RuleBlocked(DeadlineRuleBlock::OrderedQuantityExceedsMaximum {
            maximum: quantity(6),
            supplied: quantity(7),
        });
    input.examples.extend([missing, excess]);
    let profile = DeadlineProfileDefinition::new(input).unwrap();
    assert_eq!(profile.examples()[0].ordered_quantity, Some(quantity(2)));
    assert_eq!(profile.examples().len(), 3);
}

#[test]
fn a_mismatched_rule_block_is_rejected_even_with_a_valid_positive_example() {
    let mut input = input();
    let mut invalid = example(3);
    invalid.ordered_quantity = Some(quantity(2));
    invalid.expected =
        DeadlineExampleExpected::RuleBlocked(DeadlineRuleBlock::MissingOrderedQuantity);
    input.examples.push(invalid);
    assert!(DeadlineProfileDefinition::new(input).is_err());
}

#[test]
fn a_fixed_rule_can_include_a_negative_example_for_unexpected_quantity() {
    let mut input = input();
    let mut negative = example(3);
    negative.ordered_quantity = Some(quantity(2));
    negative.expected =
        DeadlineExampleExpected::RuleBlocked(DeadlineRuleBlock::UnexpectedOrderedQuantity);
    input.examples.push(negative);
    assert!(DeadlineProfileDefinition::new(input).is_ok());
}

#[test]
fn monthly_examples_check_the_homologue_instead_of_converting_months_to_days() {
    let mut input = input();
    input.template = DeadlineRuleTemplate::Ordered {
        unit: OrderedDeadlineUnit::CivilMonths {
            final_day: FinalDayPolicy::Preserve,
        },
        maximum: Some(quantity(2)),
    };
    input.examples[0].anchor = DeclaredProceduralTime::date(date("2026-01-31"), None).unwrap();
    input.examples[0].ordered_quantity = Some(quantity(2));
    input.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::CivilCandidate {
            date: date("2026-03-31"),
        });
    let mut missing = input.examples[0].clone();
    missing.id = id(3);
    missing.ordered_quantity = Some(quantity(1));
    missing.expected = DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::Blocked(
        ArithmeticBlock::MissingHomologousDay {
            year: 2026,
            month: 2,
            requested_day: 31,
        },
    ));
    input.examples.push(missing);
    assert!(DeadlineProfileDefinition::new(input).is_ok());
}

#[test]
fn the_full_hour_quantity_range_can_include_an_expected_overflow() {
    let mut input = input();
    input.template = DeadlineRuleTemplate::Ordered {
        unit: OrderedDeadlineUnit::ElapsedHours,
        maximum: Some(quantity(u32::MAX)),
    };
    input.completion = DeadlineCompletionPolicy::ArithmeticInstant;
    input.examples[0].anchor =
        DeclaredProceduralTime::second(date("2026-01-06"), 0, 0, 0, Some(UtcOffset::UTC)).unwrap();
    input.examples[0].ordered_quantity = Some(quantity(1));
    input.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate {
            instant: datetime!(2026-01-06 01:00 UTC),
        });
    let mut overflow = input.examples[0].clone();
    overflow.id = id(3);
    overflow.ordered_quantity = Some(quantity(u32::MAX));
    overflow.expected = DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::Blocked(
        ArithmeticBlock::DateRangeExhausted,
    ));
    input.examples.push(overflow);
    assert!(DeadlineProfileDefinition::new(input).is_ok());
}

#[test]
fn monthly_templates_reject_instant_completion_with_an_otherwise_valid_corpus() {
    for template in [
        DeadlineRuleTemplate::Fixed(ArithmeticRule::CivilMonths {
            quantity: quantity(1),
            final_day: FinalDayPolicy::Preserve,
        }),
        DeadlineRuleTemplate::Ordered {
            unit: OrderedDeadlineUnit::CivilMonths {
                final_day: FinalDayPolicy::Preserve,
            },
            maximum: None,
        },
    ] {
        let mut input = input();
        input.template = template;
        input.completion = DeadlineCompletionPolicy::ArithmeticInstant;
        input.examples[0].ordered_quantity = match template {
            DeadlineRuleTemplate::Ordered { .. } => Some(quantity(1)),
            _ => None,
        };
        input.examples[0].expected =
            DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::CivilCandidate {
                date: date("2026-02-06"),
            });
        assert!(DeadlineProfileDefinition::new(input).is_err());
    }
}

#[test]
fn exceeding_the_maximum_must_match_both_operands_in_the_expected_block() {
    for (maximum, supplied) in [(5, 7), (6, 8)] {
        let mut input = input();
        input.template = DeadlineRuleTemplate::Ordered {
            unit: OrderedDeadlineUnit::Days {
                inclusion: DayInclusion::OnAnchor,
                basis: DayBasis::Natural,
                final_day: FinalDayPolicy::Preserve,
            },
            maximum: Some(quantity(6)),
        };
        input.examples[0].ordered_quantity = Some(quantity(2));
        let mut excess = example(3);
        excess.ordered_quantity = Some(quantity(7));
        excess.expected = DeadlineExampleExpected::RuleBlocked(
            DeadlineRuleBlock::OrderedQuantityExceedsMaximum {
                maximum: quantity(maximum),
                supplied: quantity(supplied),
            },
        );
        input.examples.push(excess);
        assert!(DeadlineProfileDefinition::new(input).is_err());
    }
}
