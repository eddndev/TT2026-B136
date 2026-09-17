use domain::{
    deadline_arithmetic::{ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy},
    deadline_profiles::{DeadlineRuleBlock, DeadlineRuleTemplate, OrderedDeadlineUnit},
};
use std::num::NonZeroU32;

fn quantity(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

fn units_and_rules(value: u32) -> [(OrderedDeadlineUnit, ArithmeticRule); 3] {
    let quantity = quantity(value);
    [
        (
            OrderedDeadlineUnit::Days {
                inclusion: DayInclusion::AfterAnchor,
                basis: DayBasis::CalendarCountable,
                final_day: FinalDayPolicy::NextCountable,
            },
            ArithmeticRule::Days {
                quantity,
                inclusion: DayInclusion::AfterAnchor,
                basis: DayBasis::CalendarCountable,
                final_day: FinalDayPolicy::NextCountable,
            },
        ),
        (
            OrderedDeadlineUnit::CivilMonths {
                final_day: FinalDayPolicy::Preserve,
            },
            ArithmeticRule::CivilMonths {
                quantity,
                final_day: FinalDayPolicy::Preserve,
            },
        ),
        (
            OrderedDeadlineUnit::ElapsedHours,
            ArithmeticRule::ElapsedHours { quantity },
        ),
    ]
}

#[test]
fn fixed_rules_preserve_each_unit_and_quantity_without_ordered_input() {
    for value in [1, 7, u32::MAX] {
        for (_, rule) in units_and_rules(value) {
            let template = DeadlineRuleTemplate::Fixed(rule);
            assert_eq!(template.instantiate(None), Ok(rule));
        }
    }
}

#[test]
fn fixed_rules_reject_even_an_identical_ordered_quantity() {
    for (_, rule) in units_and_rules(7) {
        let template = DeadlineRuleTemplate::Fixed(rule);
        for supplied in [1, 7, u32::MAX] {
            assert_eq!(
                template.instantiate(Some(quantity(supplied))),
                Err(DeadlineRuleBlock::UnexpectedOrderedQuantity),
            );
        }
    }
}

#[test]
fn ordered_rules_without_a_ceiling_still_require_a_quantity() {
    for (unit, _) in units_and_rules(1) {
        let template = DeadlineRuleTemplate::Ordered {
            unit,
            maximum: None,
        };
        assert_eq!(
            template.instantiate(None),
            Err(DeadlineRuleBlock::MissingOrderedQuantity),
        );
    }
}

#[test]
fn a_ceiling_never_substitutes_for_a_missing_ordered_quantity() {
    for maximum in [1, 6, u32::MAX] {
        for (unit, _) in units_and_rules(1) {
            let template = DeadlineRuleTemplate::Ordered {
                unit,
                maximum: Some(quantity(maximum)),
            };
            assert_eq!(
                template.instantiate(None),
                Err(DeadlineRuleBlock::MissingOrderedQuantity),
            );
        }
    }
}

#[test]
fn an_ordered_quantity_below_the_ceiling_is_not_promoted_to_it() {
    for (unit, expected) in units_and_rules(2) {
        let template = DeadlineRuleTemplate::Ordered {
            unit,
            maximum: Some(quantity(6)),
        };
        assert_eq!(template.instantiate(Some(quantity(2))), Ok(expected));
    }
}

#[test]
fn an_ordered_quantity_equal_to_the_ceiling_is_accepted() {
    for maximum in [1, 6, u32::MAX] {
        for (unit, expected) in units_and_rules(maximum) {
            let template = DeadlineRuleTemplate::Ordered {
                unit,
                maximum: Some(quantity(maximum)),
            };
            assert_eq!(template.instantiate(Some(quantity(maximum))), Ok(expected));
        }
    }
}

#[test]
fn exceeding_the_ceiling_reports_both_exact_operands_without_clamping() {
    for (maximum, supplied) in [(1, 2), (6, 7), (u32::MAX - 1, u32::MAX)] {
        for (unit, _) in units_and_rules(1) {
            let template = DeadlineRuleTemplate::Ordered {
                unit,
                maximum: Some(quantity(maximum)),
            };
            assert_eq!(
                template.instantiate(Some(quantity(supplied))),
                Err(DeadlineRuleBlock::OrderedQuantityExceedsMaximum {
                    maximum: quantity(maximum),
                    supplied: quantity(supplied),
                }),
            );
        }
    }
}

#[test]
fn no_ceiling_preserves_the_full_nonzero_u32_domain_for_every_unit() {
    for supplied in [1, 72, u32::MAX] {
        for (unit, expected) in units_and_rules(supplied) {
            let template = DeadlineRuleTemplate::Ordered {
                unit,
                maximum: None,
            };
            assert_eq!(template.instantiate(Some(quantity(supplied))), Ok(expected));
        }
    }
}

#[test]
fn daily_templates_preserve_all_explicit_policy_combinations() {
    for inclusion in [DayInclusion::OnAnchor, DayInclusion::AfterAnchor] {
        for basis in [DayBasis::Natural, DayBasis::CalendarCountable] {
            for final_day in [FinalDayPolicy::Preserve, FinalDayPolicy::NextCountable] {
                let expected = ArithmeticRule::Days {
                    quantity: quantity(3),
                    inclusion,
                    basis,
                    final_day,
                };
                let ordered = DeadlineRuleTemplate::Ordered {
                    unit: OrderedDeadlineUnit::Days {
                        inclusion,
                        basis,
                        final_day,
                    },
                    maximum: Some(quantity(9)),
                };
                assert_eq!(ordered.instantiate(Some(quantity(3))), Ok(expected));
                assert_eq!(
                    DeadlineRuleTemplate::Fixed(expected).instantiate(None),
                    Ok(expected)
                );
            }
        }
    }
}

#[test]
fn monthly_templates_preserve_the_final_day_policy_and_month_unit() {
    for final_day in [FinalDayPolicy::Preserve, FinalDayPolicy::NextCountable] {
        let template = DeadlineRuleTemplate::Ordered {
            unit: OrderedDeadlineUnit::CivilMonths { final_day },
            maximum: Some(quantity(6)),
        };
        assert_eq!(
            template.instantiate(Some(quantity(1))),
            Ok(ArithmeticRule::CivilMonths {
                quantity: quantity(1),
                final_day,
            }),
        );
    }
}

#[test]
fn a_large_ceiling_is_only_compared_never_added_to_the_ordered_quantity() {
    for (unit, expected) in units_and_rules(u32::MAX - 1) {
        let template = DeadlineRuleTemplate::Ordered {
            unit,
            maximum: Some(quantity(u32::MAX)),
        };
        assert_eq!(
            template.instantiate(Some(quantity(u32::MAX - 1))),
            Ok(expected)
        );
    }
}
