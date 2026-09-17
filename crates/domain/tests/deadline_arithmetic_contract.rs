mod deadline_days_support;
use deadline_days_support::*;
use domain::deadline_arithmetic::*;
use domain::procedural_time::DeclaredProceduralTime;

fn anchor(day: &str) -> DeclaredProceduralTime {
    DeclaredProceduralTime::date(date(day), None).unwrap()
}

#[test]
fn natural_day_inclusion_is_explicit_and_preserves_the_rule() {
    for (inclusion, expected) in [
        (DayInclusion::OnAnchor, "2026-01-31"),
        (DayInclusion::AfterAnchor, "2026-02-01"),
    ] {
        let rule = ArithmeticRule::Days {
            quantity: quantity(2),
            inclusion,
            basis: DayBasis::Natural,
            final_day: FinalDayPolicy::Preserve,
        };
        let input = anchor("2026-01-30");
        let result = evaluate_deadline_arithmetic(rule, input, None);
        assert_eq!(result.rule(), rule);
        assert_eq!(result.anchor(), input);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::CivilCandidate {
                date: date(expected)
            }
        );
    }
}

#[test]
fn civil_months_use_the_original_day_without_iterative_clamping() {
    let input = anchor("2026-01-31");
    let evaluate = |months| {
        evaluate_deadline_arithmetic(
            ArithmeticRule::CivilMonths {
                quantity: quantity(months),
                final_day: FinalDayPolicy::Preserve,
            },
            input,
            None,
        )
    };
    assert_eq!(
        evaluate(1).outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::MissingHomologousDay {
            year: 2026,
            month: 2,
            requested_day: 31
        })
    );
    assert_eq!(
        evaluate(2).outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-03-31")
        }
    );
}

#[test]
fn a_natural_candidate_can_be_adjusted_without_excluding_intermediate_days() {
    let calendar = calendar("2026-01-01", "2026-02-28", weekdays(), vec![]);
    let rule = ArithmeticRule::Days {
        quantity: quantity(2),
        inclusion: DayInclusion::AfterAnchor,
        basis: DayBasis::Natural,
        final_day: FinalDayPolicy::NextCountable,
    };
    let result = evaluate_deadline_arithmetic(rule, anchor("2026-01-30"), Some(&calendar));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-02-02")
        }
    );
    assert!(
        matches!(&result.trace()[0], ArithmeticTraceStep::NaturalDays { candidate: Some(day), .. } if *day == date("2026-02-01"))
    );
    assert!(
        matches!(&result.trace()[1], ArithmeticTraceStep::FinalDay(count) if count.trace().len() == 2)
    );
}

#[test]
fn calendar_counting_uses_the_supplied_values_from_the_derived_start() {
    let calendar = calendar("2026-01-01", "2026-02-28", weekdays(), vec![]);
    let rule = ArithmeticRule::Days {
        quantity: quantity(2),
        inclusion: DayInclusion::AfterAnchor,
        basis: DayBasis::CalendarCountable,
        final_day: FinalDayPolicy::Preserve,
    };
    let result = evaluate_deadline_arithmetic(rule, anchor("2026-01-30"), Some(&calendar));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-02-03")
        }
    );
    let ArithmeticTraceStep::CountedDays(count) = &result.trace()[0] else {
        panic!("calendar trace missing")
    };
    assert_eq!(count.first_included(), date("2026-01-31"));
    assert_eq!(count.trace().len(), 4);
}

#[test]
fn missing_prerequisites_are_not_filled_from_the_clock_or_calendar() {
    let rule = ArithmeticRule::CivilMonths {
        quantity: quantity(6),
        final_day: FinalDayPolicy::NextCountable,
    };
    let unknown = evaluate_deadline_arithmetic(rule, DeclaredProceduralTime::unknown(), None);
    assert_eq!(
        unknown.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::UnknownAnchor)
    );
    assert!(unknown.trace().is_empty());
    let missing = evaluate_deadline_arithmetic(rule, anchor("2026-01-01"), None);
    assert_eq!(
        missing.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::MissingCalendar)
    );
    assert!(missing.trace().is_empty());
}
