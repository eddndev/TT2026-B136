mod deadline_days_support;

use deadline_days_support::{calendar, date, quantity};
use domain::{
    deadline_arithmetic::{
        evaluate_deadline_arithmetic, ArithmeticBlock, ArithmeticOutcome, ArithmeticRule,
        ArithmeticTraceStep,
    },
    judicial_calendars::JudicialCalendarClassification,
    procedural_time::{
        DeclaredProceduralPrecision as Precision, DeclaredProceduralTime as Declared,
    },
};
use time::{macros::datetime, OffsetDateTime, UtcOffset};

fn second(day: &str, hour: u8, minute: u8, second: u8, offset: i32) -> Declared {
    Declared::second(
        date(day),
        hour,
        minute,
        second,
        Some(UtcOffset::from_whole_seconds(offset).unwrap()),
    )
    .unwrap()
}

fn assert_candidate(
    anchor: Declared,
    hours: u32,
    expected_start: OffsetDateTime,
    expected_end: OffsetDateTime,
) {
    let quantity = quantity(hours);
    let rule = ArithmeticRule::ElapsedHours { quantity };
    let result = evaluate_deadline_arithmetic(rule, anchor, None);
    assert_eq!(result.rule(), rule);
    assert_eq!(result.anchor(), anchor);
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::InstantCandidate {
            instant: expected_end,
        }
    );
    assert_eq!(
        result.trace(),
        &[ArithmeticTraceStep::ElapsedHours {
            start: expected_start,
            quantity,
            candidate: Some(expected_end),
        }]
    );
    let ArithmeticOutcome::InstantCandidate { instant } = result.outcome() else {
        panic!("expected an instant candidate");
    };
    assert_eq!(instant.offset(), UtcOffset::UTC);
    assert_eq!(instant.nanosecond(), 0);
    let ArithmeticTraceStep::ElapsedHours {
        start,
        candidate: Some(end),
        ..
    } = &result.trace()[0]
    else {
        panic!("expected an elapsed-hours trace");
    };
    assert_eq!(start.offset(), UtcOffset::UTC);
    assert_eq!(end.offset(), UtcOffset::UTC);
    assert_eq!((*end - *start).whole_seconds(), i64::from(hours) * 3600);
}

#[test]
fn unknown_anchor_blocks_without_a_fabricated_start_or_trace() {
    let anchor = Declared::unknown();
    let rule = ArithmeticRule::ElapsedHours {
        quantity: quantity(1),
    };
    let result = evaluate_deadline_arithmetic(rule, anchor, None);
    assert_eq!(result.anchor(), anchor);
    assert_eq!(result.rule(), rule);
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::UnknownAnchor)
    );
    assert!(result.trace().is_empty());
}

#[test]
fn date_and_minute_block_for_precision_before_considering_missing_offset() {
    for offset in [None, Some(UtcOffset::UTC)] {
        for (anchor, observed) in [
            (
                Declared::date(date("2026-01-01"), offset).unwrap(),
                Precision::Date,
            ),
            (
                Declared::minute(date("2026-01-01"), 12, 34, offset).unwrap(),
                Precision::Minute,
            ),
        ] {
            let result = evaluate_deadline_arithmetic(
                ArithmeticRule::ElapsedHours {
                    quantity: quantity(2),
                },
                anchor,
                None,
            );
            assert_eq!(result.anchor(), anchor);
            assert_eq!(anchor.local_second(), None);
            assert_eq!(
                result.outcome(),
                &ArithmeticOutcome::Blocked(ArithmeticBlock::InsufficientPrecision { observed })
            );
            assert!(result.trace().is_empty());
        }
    }
}

#[test]
fn local_second_without_offset_blocks_instead_of_assuming_utc() {
    let anchor = Declared::second(date("2026-01-01"), 12, 34, 0, None).unwrap();
    let result = evaluate_deadline_arithmetic(
        ArithmeticRule::ElapsedHours {
            quantity: quantity(1),
        },
        anchor,
        None,
    );
    assert_eq!(result.anchor(), anchor);
    assert_eq!(
        result.outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::MissingOffset)
    );
    assert!(result.trace().is_empty());
}

#[test]
fn utc_hours_cross_midnight_and_preserve_seconds() {
    assert_candidate(
        second("2026-12-31", 23, 45, 17, 0),
        2,
        datetime!(2026-12-31 23:45:17 UTC),
        datetime!(2027-01-01 01:45:17 UTC),
    );
}

#[test]
fn positive_fixed_offset_is_normalized_before_the_duration_is_added() {
    assert_candidate(
        second("2026-01-31", 23, 15, 37, 9000),
        10,
        datetime!(2026-01-31 20:45:37 UTC),
        datetime!(2026-02-01 06:45:37 UTC),
    );
}

#[test]
fn negative_offset_crosses_leap_day_by_exact_elapsed_seconds() {
    assert_candidate(
        second("2028-02-28", 23, 30, 1, -21600),
        24,
        datetime!(2028-02-29 05:30:01 UTC),
        datetime!(2028-03-01 05:30:01 UTC),
    );
    assert_candidate(
        second("2028-02-28", 23, 59, 59, 0),
        1,
        datetime!(2028-02-28 23:59:59 UTC),
        datetime!(2028-02-29 00:59:59 UTC),
    );
}

#[test]
fn equivalent_instants_keep_distinct_declarations_and_equal_utc_candidates() {
    let local = second("2026-01-01", 0, 0, 7, 50400);
    let utc = second("2025-12-31", 10, 0, 7, 0);
    assert_ne!(local, utc);
    for anchor in [local, utc] {
        assert_candidate(
            anchor,
            3,
            datetime!(2025-12-31 10:00:07 UTC),
            datetime!(2025-12-31 13:00:07 UTC),
        );
    }
}

#[test]
fn elapsed_hours_ignore_excluded_unresolved_and_uncovered_civil_dates() {
    let anchor = second("2026-01-01", 23, 15, 0, -21600);
    let rule = ArithmeticRule::ElapsedHours {
        quantity: quantity(24),
    };
    let expected = evaluate_deadline_arithmetic(rule, anchor, None);
    for classification in [
        JudicialCalendarClassification::Excluded,
        JudicialCalendarClassification::Unresolved,
    ] {
        let values = calendar("2026-01-01", "2026-01-01", [classification; 7], vec![]);
        let result = evaluate_deadline_arithmetic(rule, anchor, Some(&values));
        assert_eq!(result.outcome(), expected.outcome());
        assert_eq!(result.trace(), expected.trace());
        assert_eq!(result.anchor(), anchor);
    }
    assert_candidate(
        anchor,
        24,
        datetime!(2026-01-02 05:15:00 UTC),
        datetime!(2026-01-03 05:15:00 UTC),
    );
}

#[test]
fn both_utc_year_boundaries_allow_representable_candidates() {
    assert_candidate(
        second("0001-01-01", 14, 0, 0, 50400),
        1,
        datetime!(0001-01-01 00:00:00 UTC),
        datetime!(0001-01-01 01:00:00 UTC),
    );
    assert_candidate(
        second("9999-12-31", 22, 59, 59, 0),
        1,
        datetime!(9999-12-31 22:59:59 UTC),
        datetime!(9999-12-31 23:59:59 UTC),
    );
}

#[test]
fn valid_utc_candidate_does_not_require_a_future_projection_at_the_original_offset() {
    assert_candidate(
        second("9999-12-31", 23, 30, 0, 50400),
        2,
        datetime!(9999-12-31 09:30:00 UTC),
        datetime!(9999-12-31 11:30:00 UTC),
    );
}

#[test]
fn year_overflow_and_maximum_quantity_leave_one_trace_step_without_candidate() {
    for (anchor, hours, expected_start) in [
        (
            second("9999-12-31", 23, 0, 0, 0),
            1,
            datetime!(9999-12-31 23:00:00 UTC),
        ),
        (
            second("2026-01-01", 10, 34, 56, 20700),
            u32::MAX,
            datetime!(2026-01-01 04:49:56 UTC),
        ),
    ] {
        let quantity = quantity(hours);
        let result =
            evaluate_deadline_arithmetic(ArithmeticRule::ElapsedHours { quantity }, anchor, None);
        assert_eq!(result.anchor(), anchor);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::Blocked(ArithmeticBlock::DateRangeExhausted)
        );
        assert_eq!(
            result.trace(),
            &[ArithmeticTraceStep::ElapsedHours {
                start: expected_start,
                quantity,
                candidate: None,
            }]
        );
        let ArithmeticTraceStep::ElapsedHours { start, .. } = &result.trace()[0] else {
            panic!("expected one bounded elapsed-hours trace");
        };
        assert_eq!(start.offset(), UtcOffset::UTC);
    }
}

#[test]
fn repeated_evaluation_preserves_the_complete_original_declaration() {
    let anchor = second("2026-01-01", 10, 20, 30, 20700);
    let original = anchor;
    let rule = ArithmeticRule::ElapsedHours {
        quantity: quantity(17),
    };
    let first = evaluate_deadline_arithmetic(rule, anchor, None);
    let second = evaluate_deadline_arithmetic(rule, anchor, None);
    assert_eq!(first.outcome(), second.outcome());
    assert_eq!(first.trace(), second.trace());
    assert_eq!(anchor, original);
    assert_eq!(first.anchor(), original);
    assert_eq!(first.anchor().local_hour(), Some(10));
    assert_eq!(first.anchor().local_minute(), Some(20));
    assert_eq!(first.anchor().local_second(), Some(30));
    assert_eq!(first.anchor().offset().unwrap().whole_seconds(), 20700);
    assert_candidate(
        original,
        17,
        datetime!(2026-01-01 04:35:30 UTC),
        datetime!(2026-01-01 21:35:30 UTC),
    );
}
