mod deadline_days_support;

use deadline_days_support::{date, quantity};
use domain::deadline_arithmetic::{
    evaluate_deadline_arithmetic, ArithmeticBlock, ArithmeticOutcome, ArithmeticRule,
    ArithmeticTraceStep, DayBasis, DayInclusion, FinalDayPolicy,
};
use domain::procedural_time::DeclaredProceduralTime;
use time::UtcOffset;

fn anchor(value: &str) -> DeclaredProceduralTime {
    DeclaredProceduralTime::date(date(value), None).unwrap()
}
fn days(value: u32, inclusion: DayInclusion) -> ArithmeticRule {
    ArithmeticRule::Days {
        quantity: quantity(value),
        inclusion,
        basis: DayBasis::Natural,
        final_day: FinalDayPolicy::Preserve,
    }
}
fn months(value: u32) -> ArithmeticRule {
    ArithmeticRule::CivilMonths {
        quantity: quantity(value),
        final_day: FinalDayPolicy::Preserve,
    }
}

#[test]
fn natural_days_include_the_anchor_only_when_requested() {
    for (count, inclusion, first, expected) in [
        (1, DayInclusion::OnAnchor, "2026-01-09", "2026-01-09"),
        (2, DayInclusion::OnAnchor, "2026-01-09", "2026-01-10"),
        (1, DayInclusion::AfterAnchor, "2026-01-10", "2026-01-10"),
        (2, DayInclusion::AfterAnchor, "2026-01-10", "2026-01-11"),
    ] {
        let rule = days(count, inclusion);
        let source = anchor("2026-01-09");
        let result = evaluate_deadline_arithmetic(rule, source, None);
        assert_eq!(result.rule(), rule);
        assert_eq!(result.anchor(), source);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::CivilCandidate {
                date: date(expected)
            }
        );
        assert_eq!(
            result.trace(),
            &[ArithmeticTraceStep::NaturalDays {
                first_included: date(first),
                quantity: quantity(count),
                candidate: Some(date(expected)),
            }]
        );
    }
}

#[test]
fn natural_days_cross_leap_days_and_year_boundaries() {
    for (start, count, expected) in [
        ("2028-02-28", 2, "2028-02-29"),
        ("2028-02-28", 3, "2028-03-01"),
        ("2027-02-28", 2, "2027-03-01"),
        ("2026-12-31", 2, "2027-01-01"),
        ("0001-01-01", 1, "0001-01-01"),
        ("9999-12-31", 1, "9999-12-31"),
    ] {
        let result =
            evaluate_deadline_arithmetic(days(count, DayInclusion::OnAnchor), anchor(start), None);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::CivilCandidate {
                date: date(expected)
            }
        );
    }
}

#[test]
fn natural_days_do_not_clamp_or_wrap_after_the_last_supported_date() {
    for (count, inclusion) in [(2, DayInclusion::OnAnchor), (1, DayInclusion::AfterAnchor)] {
        let result =
            evaluate_deadline_arithmetic(days(count, inclusion), anchor("9999-12-31"), None);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::Blocked(ArithmeticBlock::DateRangeExhausted)
        );
    }
    let result =
        evaluate_deadline_arithmetic(days(2, DayInclusion::OnAnchor), anchor("9999-12-31"), None);
    assert_eq!(
        result.trace(),
        &[ArithmeticTraceStep::NaturalDays {
            first_included: date("9999-12-31"),
            quantity: quantity(2),
            candidate: None,
        }]
    );
}

#[test]
fn maximum_natural_quantity_retains_one_bounded_trace_step() {
    for inclusion in [DayInclusion::OnAnchor, DayInclusion::AfterAnchor] {
        let result =
            evaluate_deadline_arithmetic(days(u32::MAX, inclusion), anchor("2000-01-01"), None);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::Blocked(ArithmeticBlock::DateRangeExhausted)
        );
        let first = match inclusion {
            DayInclusion::OnAnchor => "2000-01-01",
            DayInclusion::AfterAnchor => "2000-01-02",
        };
        assert_eq!(
            result.trace(),
            &[ArithmeticTraceStep::NaturalDays {
                first_included: date(first),
                quantity: quantity(u32::MAX),
                candidate: None,
            }]
        );
    }
}

#[test]
fn civil_months_require_the_exact_homologous_day_without_clamping() {
    let source = anchor("2026-01-31");
    let result = evaluate_deadline_arithmetic(months(1), source, None);
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::MissingHomologousDay {
            year: 2026,
            month: 2,
            requested_day: 31,
        })
    );
    assert_eq!(
        result.trace(),
        &[ArithmeticTraceStep::CivilMonths {
            anchor: date("2026-01-31"),
            quantity: quantity(1),
            target_year: 2026,
            target_month: 2,
            requested_day: 31,
            candidate: None,
        }]
    );
    let result = evaluate_deadline_arithmetic(months(2), source, None);
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-03-31")
        }
    );
    assert_eq!(
        result.trace(),
        &[ArithmeticTraceStep::CivilMonths {
            anchor: date("2026-01-31"),
            quantity: quantity(2),
            target_year: 2026,
            target_month: 3,
            requested_day: 31,
            candidate: Some(date("2026-03-31")),
        }]
    );
}

#[test]
fn civil_months_distinguish_leap_year_homologues() {
    for (count, expected) in [
        (
            12,
            ArithmeticOutcome::Blocked(ArithmeticBlock::MissingHomologousDay {
                year: 2029,
                month: 2,
                requested_day: 29,
            }),
        ),
        (
            48,
            ArithmeticOutcome::CivilCandidate {
                date: date("2032-02-29"),
            },
        ),
    ] {
        let result = evaluate_deadline_arithmetic(months(count), anchor("2028-02-29"), None);
        assert_eq!(result.outcome(), &expected);
    }
    let result = evaluate_deadline_arithmetic(months(12), anchor("2096-02-29"), None);
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::MissingHomologousDay {
            year: 2097,
            month: 2,
            requested_day: 29,
        })
    );
    let result = evaluate_deadline_arithmetic(months(48), anchor("2096-02-29"), None);
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::MissingHomologousDay {
            year: 2100,
            month: 2,
            requested_day: 29,
        })
    );
}

#[test]
fn civil_months_cross_years_without_treating_months_as_thirty_days() {
    for (start, count, expected) in [
        ("2026-12-31", 1, "2027-01-31"),
        ("2026-12-31", 13, "2028-01-31"),
        ("0001-01-31", 2, "0001-03-31"),
        ("9999-11-30", 1, "9999-12-30"),
        ("2026-01-01", 2, "2026-03-01"),
    ] {
        let result = evaluate_deadline_arithmetic(months(count), anchor(start), None);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::CivilCandidate {
                date: date(expected)
            }
        );
    }
}

#[test]
fn civil_month_overflow_keeps_the_requested_target_year() {
    let result = evaluate_deadline_arithmetic(months(1), anchor("9999-12-31"), None);
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::DateRangeExhausted)
    );
    assert_eq!(
        result.trace(),
        &[ArithmeticTraceStep::CivilMonths {
            anchor: date("9999-12-31"),
            quantity: quantity(1),
            target_year: 10000,
            target_month: 1,
            requested_day: 31,
            candidate: None,
        }]
    );
}

#[test]
fn maximum_month_quantity_keeps_one_step_and_the_unrepresentable_target() {
    let result = evaluate_deadline_arithmetic(months(u32::MAX), anchor("2000-01-31"), None);
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::DateRangeExhausted)
    );
    assert_eq!(
        result.trace(),
        &[ArithmeticTraceStep::CivilMonths {
            anchor: date("2000-01-31"),
            quantity: quantity(u32::MAX),
            target_year: 357915941,
            target_month: 4,
            requested_day: 31,
            candidate: None,
        }]
    );
}

#[test]
fn unknown_anchor_never_creates_a_civil_date_or_trace() {
    for rule in [
        days(1, DayInclusion::OnAnchor),
        days(1, DayInclusion::AfterAnchor),
        months(1),
    ] {
        let result = evaluate_deadline_arithmetic(rule, DeclaredProceduralTime::unknown(), None);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::Blocked(ArithmeticBlock::UnknownAnchor)
        );
        assert!(result.trace().is_empty());
    }
}

#[test]
fn civil_rules_preserve_declared_offset_and_never_invent_a_clock_time() {
    for offset in [
        None,
        Some(UtcOffset::from_hms(14, 0, 0).unwrap()),
        Some(UtcOffset::from_hms(-14, 0, 0).unwrap()),
    ] {
        let source = DeclaredProceduralTime::date(date("2026-01-01"), offset).unwrap();
        for (rule, expected) in [
            (days(1, DayInclusion::OnAnchor), "2026-01-01"),
            (months(1), "2026-02-01"),
        ] {
            let result = evaluate_deadline_arithmetic(rule, source, None);
            assert_eq!(result.anchor(), source);
            assert!(result.anchor().instant_value().is_none());
            assert_eq!(
                result.outcome(),
                &ArithmeticOutcome::CivilCandidate {
                    date: date(expected)
                }
            );
        }
    }
}

#[test]
fn civil_rules_use_the_original_local_date_for_minute_and_second_anchors() {
    let east = Some(UtcOffset::from_hms(14, 0, 0).unwrap());
    let west = Some(UtcOffset::from_hms(-14, 0, 0).unwrap());
    let sources = [
        DeclaredProceduralTime::minute(date("2026-01-01"), 0, 15, east).unwrap(),
        DeclaredProceduralTime::second(date("2026-01-01"), 23, 59, 59, west).unwrap(),
    ];
    for source in sources {
        let result = evaluate_deadline_arithmetic(days(1, DayInclusion::OnAnchor), source, None);
        assert_eq!(result.anchor(), source);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::CivilCandidate {
                date: date("2026-01-01")
            }
        );
    }
}
