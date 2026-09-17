mod deadline_days_support;

use deadline_days_support::{calendar, date, exception, quantity, weekdays};
use domain::deadline_arithmetic::{
    evaluate_deadline_arithmetic, ArithmeticBlock, ArithmeticOutcome, ArithmeticRule,
    ArithmeticTraceStep, DayBasis, DayInclusion, FinalDayPolicy,
};
use domain::deadline_days::CivilDayCountOutcome;
use domain::judicial_calendars::JudicialCalendarClassification::*;
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
fn final_day_adjustment_keeps_the_original_natural_candidate_and_its_calendar_trace() {
    let cal = calendar("2026-01-01", "2026-01-31", weekdays(), vec![]);
    let result = evaluate_deadline_arithmetic(natural_adjusted(), anchor("2026-01-09"), Some(&cal));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-01-12")
        }
    );
    let [ArithmeticTraceStep::NaturalDays { candidate, .. }, ArithmeticTraceStep::FinalDay(adjustment)] =
        result.trace()
    else {
        panic!("expected original and adjusted traces")
    };
    assert_eq!(*candidate, Some(date("2026-01-10")));
    assert_eq!(adjustment.first_included(), date("2026-01-10"));
    assert_eq!(adjustment.quantity(), quantity(1));
    assert_eq!(adjustment.trace().len(), 3);
    assert_eq!(adjustment.trace()[0].day().classification(), Some(Excluded));
    assert_eq!(adjustment.trace()[1].day().classification(), Some(Excluded));
    assert_eq!(
        adjustment.trace()[2].day().classification(),
        Some(Countable)
    );
}

#[test]
fn civil_month_final_adjustment_occurs_only_after_an_exact_homologue_exists() {
    let cal = calendar("2026-02-01", "2026-03-31", weekdays(), vec![]);
    let rule = ArithmeticRule::CivilMonths {
        quantity: quantity(1),
        final_day: FinalDayPolicy::NextCountable,
    };
    let result = evaluate_deadline_arithmetic(rule, anchor("2026-01-28"), Some(&cal));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-03-02")
        }
    );
    let [ArithmeticTraceStep::CivilMonths { candidate, .. }, ArithmeticTraceStep::FinalDay(adjustment)] =
        result.trace()
    else {
        panic!("expected month and adjustment traces")
    };
    assert_eq!(*candidate, Some(date("2026-02-28")));
    assert_eq!(adjustment.first_included(), date("2026-02-28"));
    let result = evaluate_deadline_arithmetic(rule, anchor("2026-01-31"), Some(&cal));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::MissingHomologousDay {
            year: 2026,
            month: 2,
            requested_day: 31
        })
    );
    assert!(matches!(
        result.trace(),
        [ArithmeticTraceStep::CivilMonths {
            candidate: None,
            ..
        }]
    ));
}

#[test]
fn unresolved_final_day_preserves_the_unadjusted_candidate_without_skipping_it() {
    let cal = calendar(
        "2026-01-01",
        "2026-01-31",
        weekdays(),
        vec![exception("2026-01-10", "2026-01-10", Unresolved)],
    );
    let result = evaluate_deadline_arithmetic(natural_adjusted(), anchor("2026-01-09"), Some(&cal));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::UnresolvedCalendarDate {
            date: date("2026-01-10")
        })
    );
    let [ArithmeticTraceStep::NaturalDays { candidate, .. }, ArithmeticTraceStep::FinalDay(adjustment)] =
        result.trace()
    else {
        panic!("expected original and blocked adjustment")
    };
    assert_eq!(*candidate, Some(date("2026-01-10")));
    assert_eq!(adjustment.trace().len(), 1);
    assert_eq!(
        adjustment.outcome(),
        CivilDayCountOutcome::Unresolved {
            date: date("2026-01-10")
        }
    );
    assert_eq!(
        adjustment.trace()[0].day().source_ids(),
        &[Uuid::from_u128(2)]
    );
}

#[test]
fn final_day_outside_coverage_keeps_the_excluded_days_and_missing_day() {
    let cal = calendar("2026-01-01", "2026-01-11", weekdays(), vec![]);
    let result = evaluate_deadline_arithmetic(natural_adjusted(), anchor("2026-01-09"), Some(&cal));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::OutsideCalendarCoverage {
            date: date("2026-01-12")
        })
    );
    let [ArithmeticTraceStep::NaturalDays { candidate, .. }, ArithmeticTraceStep::FinalDay(adjustment)] =
        result.trace()
    else {
        panic!("expected original and blocked adjustment")
    };
    assert_eq!(*candidate, Some(date("2026-01-10")));
    assert_eq!(adjustment.trace().len(), 3);
    let missing = adjustment.trace().last().unwrap().day();
    assert_eq!(missing.date(), date("2026-01-12"));
    assert_eq!(missing.classification(), None);
    assert!(missing.source_ids().is_empty());
}

#[test]
fn excluded_last_supported_date_blocks_adjustment_without_wrapping() {
    let cal = calendar("9999-12-31", "9999-12-31", [Excluded; 7], vec![]);
    let rule = days(
        1,
        DayInclusion::OnAnchor,
        DayBasis::Natural,
        FinalDayPolicy::NextCountable,
    );
    let result = evaluate_deadline_arithmetic(rule, anchor("9999-12-31"), Some(&cal));
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::DateRangeExhausted)
    );
    let [ArithmeticTraceStep::NaturalDays { candidate, .. }, ArithmeticTraceStep::FinalDay(adjustment)] =
        result.trace()
    else {
        panic!("expected original and exhausted adjustment")
    };
    assert_eq!(*candidate, Some(date("9999-12-31")));
    assert_eq!(adjustment.trace().len(), 1);
    assert_eq!(
        adjustment.outcome(),
        CivilDayCountOutcome::DateRangeExhausted {
            after: date("9999-12-31")
        }
    );
}
