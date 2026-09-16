mod deadline_days_support;
use deadline_days_support::*;
use domain::deadline_days::{count_calendar_days, CivilDayCountOutcome};
use domain::judicial_calendars::{
    JudicialCalendarClassification as Class, JudicialCalendarDayOrigin,
};
use uuid::Uuid;

#[test]
fn included_first_day_and_weekly_trace_match_the_manual_candidate() {
    let calendar = calendar("2026-01-01", "2026-01-31", weekdays(), vec![]);
    let result = count_calendar_days(&calendar, date("2026-01-06"), quantity(10));
    assert_eq!(
        result.outcome(),
        CivilDayCountOutcome::Candidate {
            date: date("2026-01-19")
        }
    );
    assert_eq!(result.first_included(), date("2026-01-06"));
    assert_eq!(result.quantity(), quantity(10));
    assert_eq!(result.accumulated(), 10);
    let expected = [
        ("2026-01-06", 1),
        ("2026-01-07", 2),
        ("2026-01-08", 3),
        ("2026-01-09", 4),
        ("2026-01-10", 4),
        ("2026-01-11", 4),
        ("2026-01-12", 5),
        ("2026-01-13", 6),
        ("2026-01-14", 7),
        ("2026-01-15", 8),
        ("2026-01-16", 9),
        ("2026-01-17", 9),
        ("2026-01-18", 9),
        ("2026-01-19", 10),
    ];
    assert_eq!(result.trace().len(), expected.len());
    for (step, (day, total)) in result.trace().iter().zip(expected) {
        assert_eq!(step.day(), &calendar.classify(date(day)));
        assert_eq!(step.accumulated(), total);
        assert_eq!(step.day().source_ids(), &[Uuid::from_u128(1)]);
    }
}

#[test]
fn excluded_first_day_is_in_the_trace_without_incrementing() {
    let calendar = calendar("2026-01-01", "2026-01-31", weekdays(), vec![]);
    let result = count_calendar_days(&calendar, date("2026-01-10"), quantity(10));
    assert_eq!(
        result.outcome(),
        CivilDayCountOutcome::Candidate {
            date: date("2026-01-23")
        }
    );
    assert_eq!(result.trace().len(), 14);
    assert_eq!(result.trace()[0].day().date(), date("2026-01-10"));
    assert_eq!(result.trace()[0].accumulated(), 0);
    assert_eq!(result.trace()[1].accumulated(), 0);
    assert_eq!(result.trace()[2].accumulated(), 1);
}

#[test]
fn exception_replaces_weekly_rule_and_preserves_exact_provenance() {
    let calendar = calendar(
        "2026-01-01",
        "2026-01-31",
        weekdays(),
        vec![exception("2026-01-14", "2026-01-14", Class::Excluded)],
    );
    let result = count_calendar_days(&calendar, date("2026-01-10"), quantity(10));
    assert_eq!(
        result.outcome(),
        CivilDayCountOutcome::Candidate {
            date: date("2026-01-26")
        }
    );
    assert_eq!(result.trace().len(), 17);
    let step = &result.trace()[4];
    assert_eq!(step.day(), &calendar.classify(date("2026-01-14")));
    assert_eq!(
        step.day().origin(),
        Some(JudicialCalendarDayOrigin::Exception(Uuid::from_u128(99)))
    );
    assert_eq!(step.day().classification(), Some(Class::Excluded));
    assert_eq!(
        step.day().explanation(),
        Some("Synthetic exception\nExact source")
    );
    assert_eq!(step.day().source_ids(), &[Uuid::from_u128(2)]);
    assert_eq!(step.accumulated(), 2);
    assert_eq!(result.trace().last().unwrap().accumulated(), 10);
}

#[test]
fn leap_day_is_counted_explicitly_in_the_manual_february_trace() {
    let calendar = calendar("2028-02-01", "2028-03-31", weekdays(), vec![]);
    let result = count_calendar_days(&calendar, date("2028-02-22"), quantity(10));
    assert_eq!(
        result.outcome(),
        CivilDayCountOutcome::Candidate {
            date: date("2028-03-06")
        }
    );
    let dates = [
        ("2028-02-22", 1),
        ("2028-02-23", 2),
        ("2028-02-24", 3),
        ("2028-02-25", 4),
        ("2028-02-26", 4),
        ("2028-02-27", 4),
        ("2028-02-28", 5),
        ("2028-02-29", 6),
        ("2028-03-01", 7),
        ("2028-03-02", 8),
        ("2028-03-03", 9),
        ("2028-03-04", 9),
        ("2028-03-05", 9),
        ("2028-03-06", 10),
    ];
    assert_eq!(result.trace().len(), dates.len());
    for (step, (day, total)) in result.trace().iter().zip(dates) {
        assert_eq!(step.day().date(), date(day));
        assert_eq!(step.accumulated(), total);
    }
}

#[test]
fn a_declared_countable_saturday_is_counted_without_a_legal_profile() {
    let mut weekly = weekdays();
    weekly[5] = Class::Countable;
    let calendar = calendar("2026-01-01", "2026-01-31", weekly, vec![]);
    let result = count_calendar_days(&calendar, date("2026-01-10"), quantity(1));
    assert_eq!(
        result.outcome(),
        CivilDayCountOutcome::Candidate {
            date: date("2026-01-10")
        }
    );
    assert_eq!(result.trace().len(), 1);
    assert_eq!(
        result.trace()[0].day().origin(),
        Some(JudicialCalendarDayOrigin::WeeklyPattern(6))
    );
    assert_eq!(result.trace()[0].accumulated(), 1);
}

#[test]
fn reaching_the_candidate_does_not_read_later_unresolved_days() {
    let calendar = calendar(
        "2026-01-01",
        "2026-01-31",
        weekdays(),
        vec![exception("2026-01-07", "2026-01-07", Class::Unresolved)],
    );
    let result = count_calendar_days(&calendar, date("2026-01-06"), quantity(1));
    assert_eq!(
        result.outcome(),
        CivilDayCountOutcome::Candidate {
            date: date("2026-01-06")
        }
    );
    assert_eq!(result.trace().len(), 1);
}

#[test]
fn exact_values_remain_unchanged_and_repeat_the_same_trace() {
    let calendar = calendar("0001-01-01", "0001-01-07", [Class::Countable; 7], vec![]);
    let before = calendar.clone();
    let first = count_calendar_days(&calendar, date("0001-01-01"), quantity(2));
    let second = count_calendar_days(&calendar, date("0001-01-01"), quantity(2));
    assert_eq!(
        first.outcome(),
        CivilDayCountOutcome::Candidate {
            date: date("0001-01-02")
        }
    );
    assert_eq!(first, second);
    assert_eq!(calendar, before);
}
