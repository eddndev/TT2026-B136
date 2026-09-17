mod deadline_days_support;

use deadline_days_support::{calendar, date, exception, quantity, weekdays};
use domain::deadline_arithmetic::{
    evaluate_deadline_arithmetic, ArithmeticBlock, ArithmeticOutcome, ArithmeticRule,
    ArithmeticTraceStep, DayBasis, DayInclusion, FinalDayPolicy,
};
use domain::deadline_days::CivilDayCountOutcome;
use domain::judicial_calendars::{JudicialCalendarClassification::*, JudicialCalendarDayOrigin};
use domain::procedural_time::DeclaredProceduralTime;
use uuid::Uuid;

fn anchor(value: &str) -> DeclaredProceduralTime {
    DeclaredProceduralTime::date(date(value), None).unwrap()
}
fn days(
    value: u32,
    inclusion: DayInclusion,
    basis: DayBasis,
    final_day: FinalDayPolicy,
) -> ArithmeticRule {
    ArithmeticRule::Days {
        quantity: quantity(value),
        inclusion,
        basis,
        final_day,
    }
}
fn natural_adjusted() -> ArithmeticRule {
    days(
        2,
        DayInclusion::OnAnchor,
        DayBasis::Natural,
        FinalDayPolicy::NextCountable,
    )
}

#[test]
fn calendar_counting_derives_the_first_included_date_before_skipping_exclusions() {
    let cal = calendar("2026-01-01", "2026-01-31", weekdays(), vec![]);
    for (count, inclusion, first, expected) in [
        (1, DayInclusion::OnAnchor, "2026-01-09", "2026-01-09"),
        (2, DayInclusion::OnAnchor, "2026-01-09", "2026-01-12"),
        (1, DayInclusion::AfterAnchor, "2026-01-10", "2026-01-12"),
        (2, DayInclusion::AfterAnchor, "2026-01-10", "2026-01-13"),
    ] {
        let rule = days(
            count,
            inclusion,
            DayBasis::CalendarCountable,
            FinalDayPolicy::Preserve,
        );
        let result = evaluate_deadline_arithmetic(rule, anchor("2026-01-09"), Some(&cal));
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::CivilCandidate {
                date: date(expected)
            }
        );
        let [ArithmeticTraceStep::CountedDays(trace)] = result.trace() else {
            panic!("expected counting trace")
        };
        assert_eq!(trace.first_included(), date(first));
        assert_eq!(trace.quantity(), quantity(count));
        assert_eq!(trace.accumulated(), count);
        assert_eq!(
            trace.outcome(),
            CivilDayCountOutcome::Candidate {
                date: date(expected)
            }
        );
        assert_eq!(trace.trace().first().unwrap().day().date(), date(first));
        assert_eq!(trace.trace().last().unwrap().day().date(), date(expected));
    }
}

#[test]
fn natural_days_preserve_excluded_dates_while_calendar_days_skip_them() {
    let cal = calendar("2026-01-01", "2026-01-31", weekdays(), vec![]);
    for (basis, expected) in [
        (DayBasis::Natural, "2026-01-10"),
        (DayBasis::CalendarCountable, "2026-01-12"),
    ] {
        let result = evaluate_deadline_arithmetic(
            days(2, DayInclusion::OnAnchor, basis, FinalDayPolicy::Preserve),
            anchor("2026-01-09"),
            Some(&cal),
        );
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::CivilCandidate {
                date: date(expected)
            }
        );
    }
    let excluded = calendar("2026-01-01", "2026-01-31", [Excluded; 7], vec![]);
    let result = evaluate_deadline_arithmetic(
        days(
            2,
            DayInclusion::OnAnchor,
            DayBasis::Natural,
            FinalDayPolicy::Preserve,
        ),
        anchor("2026-01-09"),
        Some(&excluded),
    );
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-01-10")
        }
    );
}

#[test]
fn required_calendar_is_checked_before_any_arithmetic_trace() {
    let rules = [
        days(
            1,
            DayInclusion::OnAnchor,
            DayBasis::CalendarCountable,
            FinalDayPolicy::Preserve,
        ),
        natural_adjusted(),
        ArithmeticRule::CivilMonths {
            quantity: quantity(1),
            final_day: FinalDayPolicy::NextCountable,
        },
    ];
    for rule in rules {
        let result = evaluate_deadline_arithmetic(rule, anchor("2026-01-01"), None);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::Blocked(ArithmeticBlock::MissingCalendar)
        );
        assert!(result.trace().is_empty());
        let result = evaluate_deadline_arithmetic(rule, DeclaredProceduralTime::unknown(), None);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::Blocked(ArithmeticBlock::UnknownAnchor)
        );
        assert!(result.trace().is_empty());
    }
}

#[test]
fn calendar_counting_stops_at_unknown_classification_before_a_later_candidate() {
    let cal = calendar(
        "2026-01-01",
        "2026-01-31",
        weekdays(),
        vec![exception("2026-01-12", "2026-01-12", Unresolved)],
    );
    let rule = days(
        2,
        DayInclusion::OnAnchor,
        DayBasis::CalendarCountable,
        FinalDayPolicy::Preserve,
    );
    let result = evaluate_deadline_arithmetic(rule, anchor("2026-01-09"), Some(&cal));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::UnresolvedCalendarDate {
            date: date("2026-01-12")
        })
    );
    let [ArithmeticTraceStep::CountedDays(trace)] = result.trace() else {
        panic!("expected stopped counting")
    };
    assert_eq!(trace.accumulated(), 1);
    assert_eq!(trace.trace().len(), 4);
    assert_eq!(
        trace.trace().last().unwrap().day().classification(),
        Some(Unresolved)
    );
}

#[test]
fn calendar_counting_never_searches_a_later_coverage_for_an_outside_start() {
    let cal = calendar("2026-01-10", "2026-01-31", [Countable; 7], vec![]);
    let rule = days(
        1,
        DayInclusion::OnAnchor,
        DayBasis::CalendarCountable,
        FinalDayPolicy::Preserve,
    );
    let result = evaluate_deadline_arithmetic(rule, anchor("2026-01-09"), Some(&cal));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::OutsideCalendarCoverage {
            date: date("2026-01-09")
        })
    );
    let [ArithmeticTraceStep::CountedDays(trace)] = result.trace() else {
        panic!("expected blocked counting")
    };
    assert_eq!(trace.trace().len(), 1);
    assert_eq!(trace.accumulated(), 0);
    assert_eq!(trace.trace()[0].day().origin(), None);
}

#[test]
fn maximum_calendar_quantity_is_bounded_by_coverage_not_quantity() {
    let cal = calendar("2026-01-01", "2028-12-31", [Excluded; 7], vec![]);
    let rule = days(
        u32::MAX,
        DayInclusion::OnAnchor,
        DayBasis::CalendarCountable,
        FinalDayPolicy::Preserve,
    );
    let result = evaluate_deadline_arithmetic(rule, anchor("2026-01-01"), Some(&cal));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::OutsideCalendarCoverage {
            date: date("2029-01-01")
        })
    );
    let [ArithmeticTraceStep::CountedDays(trace)] = result.trace() else {
        panic!("expected bounded counting")
    };
    assert_eq!(trace.trace().len(), 1097);
    assert_eq!(trace.accumulated(), 0);
}

#[test]
fn countable_saturday_exception_preserves_its_source_and_avoids_monday_fallback() {
    let cal = calendar(
        "2026-01-01",
        "2026-01-31",
        weekdays(),
        vec![exception("2026-01-10", "2026-01-10", Countable)],
    );
    for rule in [
        natural_adjusted(),
        days(
            2,
            DayInclusion::OnAnchor,
            DayBasis::CalendarCountable,
            FinalDayPolicy::Preserve,
        ),
    ] {
        let result = evaluate_deadline_arithmetic(rule, anchor("2026-01-09"), Some(&cal));
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::CivilCandidate {
                date: date("2026-01-10")
            }
        );
        let trace = match result.trace().last().unwrap() {
            ArithmeticTraceStep::CountedDays(trace) | ArithmeticTraceStep::FinalDay(trace) => trace,
            _ => panic!("expected calendar classification"),
        };
        let saturday = trace.trace().last().unwrap();
        assert_eq!(
            saturday.day().origin(),
            Some(JudicialCalendarDayOrigin::Exception(Uuid::from_u128(99)))
        );
        assert_eq!(saturday.day().source_ids(), &[Uuid::from_u128(2)]);
        assert_eq!(
            saturday.day().explanation(),
            Some("Synthetic exception\nExact source")
        );
    }
}

#[test]
fn uncertainty_after_the_completed_candidate_does_not_invalidate_it() {
    let cal = calendar(
        "2026-01-09",
        "2026-01-10",
        weekdays(),
        vec![exception("2026-01-10", "2026-01-10", Unresolved)],
    );
    let rule = days(
        1,
        DayInclusion::OnAnchor,
        DayBasis::CalendarCountable,
        FinalDayPolicy::Preserve,
    );
    let result = evaluate_deadline_arithmetic(rule, anchor("2026-01-09"), Some(&cal));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-01-09")
        }
    );
    let [ArithmeticTraceStep::CountedDays(trace)] = result.trace() else {
        panic!("expected completed counting")
    };
    assert_eq!(trace.trace().len(), 1);
    assert_eq!(trace.trace()[0].day().source_ids(), &[Uuid::from_u128(1)]);
}
