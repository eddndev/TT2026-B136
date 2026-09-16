mod deadline_days_support;
use deadline_days_support::*;
use domain::deadline_days::{count_calendar_days, CivilDayCountOutcome};
use domain::judicial_calendars::JudicialCalendarClassification as Class;

#[test]
fn unresolved_initial_or_later_day_blocks_without_discarding_progress() {
    for (first, length, accumulated) in [("2026-01-06", 3, 2), ("2026-01-08", 1, 0)] {
        let calendar = calendar(
            "2026-01-01",
            "2026-01-31",
            weekdays(),
            vec![exception("2026-01-08", "2026-01-08", Class::Unresolved)],
        );
        let result = count_calendar_days(&calendar, date(first), quantity(10));
        assert_eq!(
            result.outcome(),
            CivilDayCountOutcome::Unresolved {
                date: date("2026-01-08")
            }
        );
        assert_eq!(result.trace().len(), length);
        assert_eq!(result.first_included(), date(first));
        assert_eq!(result.quantity(), quantity(10));
        assert_eq!(result.accumulated(), accumulated);
        let last = result.trace().last().unwrap();
        assert_eq!(last.day(), &calendar.classify(date("2026-01-08")));
        assert_eq!(last.accumulated(), accumulated);
    }
}

#[test]
fn starting_outside_coverage_blocks_immediately_on_either_side() {
    let calendar = calendar("2026-01-06", "2026-01-20", weekdays(), vec![]);
    for first in ["0001-01-01", "2026-01-05", "2026-01-21", "9999-12-31"] {
        let result = count_calendar_days(&calendar, date(first), quantity(u32::MAX));
        assert_eq!(
            result.outcome(),
            CivilDayCountOutcome::OutsideCoverage { date: date(first) }
        );
        assert_eq!(result.trace().len(), 1);
        let step = &result.trace()[0];
        assert_eq!(step.day(), &calendar.classify(date(first)));
        assert_eq!(step.accumulated(), 0);
        assert_eq!(step.day().classification(), None);
        assert_eq!(step.day().origin(), None);
        assert_eq!(step.day().explanation(), None);
        assert!(step.day().source_ids().is_empty());
    }
}

#[test]
fn reaching_quantity_on_last_covered_day_does_not_add_outside_step() {
    let calendar = calendar("2026-01-06", "2026-01-07", weekdays(), vec![]);
    let result = count_calendar_days(&calendar, date("2026-01-06"), quantity(2));
    assert_eq!(
        result.outcome(),
        CivilDayCountOutcome::Candidate {
            date: date("2026-01-07")
        }
    );
    assert_eq!(result.trace().len(), 2);
    assert_eq!(result.trace()[1].accumulated(), 2);
}

#[test]
fn insufficient_coverage_includes_exactly_one_outside_step() {
    let calendar = calendar("2026-01-06", "2026-01-07", weekdays(), vec![]);
    let result = count_calendar_days(&calendar, date("2026-01-06"), quantity(3));
    assert_eq!(
        result.outcome(),
        CivilDayCountOutcome::OutsideCoverage {
            date: date("2026-01-08")
        }
    );
    assert_eq!(result.trace().len(), 3);
    assert_eq!(
        result.trace()[2].day(),
        &calendar.classify(date("2026-01-08"))
    );
    assert_eq!(result.trace()[2].accumulated(), 2);
}

#[test]
fn maximal_quantity_and_coverage_have_at_most_1097_steps() {
    for (classification, total) in [(Class::Countable, 1096), (Class::Excluded, 0)] {
        let calendar = calendar("2024-01-01", "2026-12-31", [classification; 7], vec![]);
        let result = count_calendar_days(&calendar, date("2024-01-01"), quantity(u32::MAX));
        assert_eq!(
            result.outcome(),
            CivilDayCountOutcome::OutsideCoverage {
                date: date("2027-01-01")
            }
        );
        assert_eq!(result.trace().len(), 1097);
        assert_eq!(result.trace()[1095].day().date(), date("2026-12-31"));
        assert_eq!(result.trace()[1095].accumulated(), total);
        assert_eq!(result.trace()[1096].accumulated(), total);
    }
}

#[test]
fn last_representable_day_can_be_a_candidate() {
    let calendar = calendar("9999-12-29", "9999-12-31", [Class::Countable; 7], vec![]);
    let result = count_calendar_days(&calendar, date("9999-12-29"), quantity(3));
    assert_eq!(
        result.outcome(),
        CivilDayCountOutcome::Candidate {
            date: date("9999-12-31")
        }
    );
    assert_eq!(result.trace().len(), 3);
    assert_eq!(result.trace()[2].accumulated(), 3);
}

#[test]
fn incomplete_count_at_maximum_year_returns_the_last_existing_date() {
    for (classification, total) in [(Class::Countable, 3), (Class::Excluded, 0)] {
        let calendar = calendar("9999-12-29", "9999-12-31", [classification; 7], vec![]);
        let result = count_calendar_days(&calendar, date("9999-12-29"), quantity(4));
        assert_eq!(
            result.outcome(),
            CivilDayCountOutcome::DateRangeExhausted {
                after: date("9999-12-31")
            }
        );
        assert_eq!(result.trace().len(), 3);
        assert_eq!(result.trace()[2].day().date(), date("9999-12-31"));
        assert_eq!(result.trace()[2].accumulated(), total);
    }
}

#[test]
fn unresolved_on_maximum_date_blocks_before_date_range_exhaustion() {
    let calendar = calendar("9999-12-31", "9999-12-31", [Class::Unresolved; 7], vec![]);
    let result = count_calendar_days(&calendar, date("9999-12-31"), quantity(1));
    assert_eq!(
        result.outcome(),
        CivilDayCountOutcome::Unresolved {
            date: date("9999-12-31")
        }
    );
    assert_eq!(result.trace().len(), 1);
    assert_eq!(result.trace()[0].accumulated(), 0);
}
