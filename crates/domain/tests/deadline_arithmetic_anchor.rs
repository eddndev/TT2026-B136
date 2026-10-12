use crate::deadline_days_support::{calendar, date};
use domain::{
    deadline_arithmetic::{
        evaluate_deadline_arithmetic, ArithmeticBlock, ArithmeticOutcome, ArithmeticRule,
        ArithmeticTraceStep,
    },
    judicial_calendars::JudicialCalendarClassification::Excluded,
    procedural_time::{
        DeclaredProceduralPrecision as Precision, DeclaredProceduralTime as Declared,
    },
};
use time::UtcOffset;

#[test]
fn exact_anchor_is_not_shifted_to_another_day_or_cutoff() {
    let offset = UtcOffset::from_hms(-6, 0, 0).unwrap();
    let anchor = Declared::second(date("2026-10-11"), 13, 27, 42, Some(offset)).unwrap();
    let excluded = calendar("2026-10-11", "2026-10-11", [Excluded; 7], vec![]);
    for selected in [None, Some(&excluded)] {
        let result = evaluate_deadline_arithmetic(ArithmeticRule::AtAnchor, anchor, selected);
        let expected = anchor.instant_value().unwrap();
        assert_eq!(result.anchor(), anchor);
        assert_eq!(result.rule(), ArithmeticRule::AtAnchor);
        assert_eq!(
            result.outcome(),
            &ArithmeticOutcome::InstantCandidate { instant: expected }
        );
        assert_eq!(
            result.trace(),
            &[ArithmeticTraceStep::AtAnchor { instant: expected }]
        );
        let ArithmeticOutcome::InstantCandidate { instant } = result.outcome() else {
            panic!("expected exact declared instant");
        };
        assert_eq!(instant.offset(), offset);
    }
}

#[test]
fn incomplete_anchor_never_supplies_time_precision_or_offset() {
    for (anchor, expected) in [
        (Declared::unknown(), ArithmeticBlock::UnknownAnchor),
        (
            Declared::date(date("2026-10-11"), None).unwrap(),
            ArithmeticBlock::InsufficientPrecision {
                observed: Precision::Date,
            },
        ),
        (
            Declared::minute(date("2026-10-11"), 13, 27, Some(UtcOffset::UTC)).unwrap(),
            ArithmeticBlock::InsufficientPrecision {
                observed: Precision::Minute,
            },
        ),
        (
            Declared::second(date("2026-10-11"), 13, 27, 42, None).unwrap(),
            ArithmeticBlock::MissingOffset,
        ),
    ] {
        let result = evaluate_deadline_arithmetic(ArithmeticRule::AtAnchor, anchor, None);
        assert_eq!(result.outcome(), &ArithmeticOutcome::Blocked(expected));
        assert!(result.trace().is_empty());
    }
}
