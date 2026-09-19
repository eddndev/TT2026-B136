use domain::alerts::{AlertAnticipations, AlertLeadHours, AlertTimingError, AlertWindow};
use time::{Date, Duration, Month, OffsetDateTime, UtcOffset};

fn due() -> OffsetDateTime {
    Date::from_calendar_date(2026, Month::March, 2)
        .unwrap()
        .with_hms_nano(17, 30, 0, 123_456_789)
        .unwrap()
        .assume_offset(UtcOffset::from_hms(-6, 0, 0).unwrap())
}

#[test]
fn leads_accept_configurable_hours_and_reject_unbounded_values() {
    for hours in [1, 24, 48, 720] {
        assert_eq!(AlertLeadHours::new(hours).unwrap().get(), hours);
    }
    for hours in [0, 721, u16::MAX] {
        assert_eq!(
            AlertLeadHours::new(hours),
            Err(AlertTimingError::InvalidLeadHours)
        );
    }
}

#[test]
fn defaults_schedule_separate_forty_eight_and_twenty_four_hour_notices() {
    assert_eq!(
        AlertAnticipations::default()
            .hours()
            .iter()
            .map(|lead| lead.get())
            .collect::<Vec<_>>(),
        [48, 24]
    );
}

#[test]
fn rules_are_canonical_and_an_empty_list_disables_upcoming_notices() {
    let values = [24, 72, 48]
        .into_iter()
        .map(|value| AlertLeadHours::new(value).unwrap())
        .collect();
    let rules = AlertAnticipations::new(values).unwrap();
    assert_eq!(
        rules
            .hours()
            .iter()
            .map(|lead| lead.get())
            .collect::<Vec<_>>(),
        [72, 48, 24]
    );
    assert!(AlertAnticipations::new(Vec::new())
        .unwrap()
        .hours()
        .is_empty());
}

#[test]
fn duplicate_or_excess_rules_are_rejected_instead_of_silently_discarded() {
    let lead = AlertLeadHours::new(24).unwrap();
    assert_eq!(
        AlertAnticipations::new(vec![lead, lead]),
        Err(AlertTimingError::InvalidAnticipations)
    );
    let rules = (1..=9)
        .map(|value| AlertLeadHours::new(value).unwrap())
        .collect();
    assert_eq!(
        AlertAnticipations::new(rules),
        Err(AlertTimingError::InvalidAnticipations)
    );
}

#[test]
fn reminder_windows_use_elapsed_hours_across_weekends_and_preserve_nanoseconds() {
    let activity = due();
    let window = AlertWindow::upcoming(activity, AlertLeadHours::new(48).unwrap()).unwrap();
    assert_eq!(window.starts_at(), activity - Duration::hours(48));
    assert_eq!(window.starts_at().offset(), UtcOffset::UTC);
    assert_eq!(window.starts_at().nanosecond(), 123_456_789);
    assert_eq!(window.starts_at().date().day(), 28);
    assert_eq!(window.ends_at(), Some(activity.to_offset(UtcOffset::UTC)));
}

#[test]
fn a_late_scheduler_can_deliver_inside_the_window_but_not_at_or_after_due() {
    let window = AlertWindow::upcoming(due(), AlertLeadHours::new(24).unwrap()).unwrap();
    assert!(!window.contains(window.starts_at() - Duration::nanoseconds(1)));
    assert!(window.contains(window.starts_at()));
    assert!(window.contains(due() - Duration::hours(3)));
    assert!(window.contains(due() - Duration::nanoseconds(1)));
    assert!(!window.contains(due()));
    assert!(!window.contains(due() + Duration::nanoseconds(1)));
}

#[test]
fn overdue_starts_at_the_exact_due_and_has_no_fabricated_expiry() {
    let window = AlertWindow::overdue(due()).unwrap();
    assert_eq!(window.starts_at(), due().to_offset(UtcOffset::UTC));
    assert_eq!(window.ends_at(), None);
    assert!(!window.contains(due() - Duration::nanoseconds(1)));
    assert!(window.contains(due()));
    assert!(window.contains(due() + Duration::days(30)));
}

#[test]
fn timing_rejects_unrepresentable_utc_years_and_subtraction_below_year_one() {
    let first = Date::from_calendar_date(1, Month::January, 1)
        .unwrap()
        .midnight()
        .assume_utc();
    assert_eq!(
        AlertWindow::upcoming(first, AlertLeadHours::new(1).unwrap()),
        Err(AlertTimingError::UnsupportedTime)
    );
    let outside = first - Duration::nanoseconds(1);
    assert_eq!(
        AlertWindow::overdue(outside),
        Err(AlertTimingError::UnsupportedTime)
    );
    assert!(AlertWindow::overdue(first).is_ok());
    assert_eq!(
        AlertWindow::overdue(first.to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()))
            .unwrap()
            .starts_at(),
        first
    );
}
