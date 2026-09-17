mod judicial_calendar_support;
use domain::judicial_calendars::*;
use judicial_calendar_support::*;

#[test]
fn civil_dates_are_exact_and_preserve_extremes_without_a_clock() {
    for text in ["0001-01-01", "1900-02-28", "2000-02-29", "9999-12-31"] {
        assert_eq!(date(text).to_string(), text);
    }
    for text in [
        "0000-01-01",
        "10000-01-01",
        "2026-2-01",
        "2026-02-30",
        "1900-02-29",
        "2026-04-31",
        " 2026-01-01",
        "2026-01-01Z",
        "2026-01-01T00:00:00Z",
    ] {
        assert!(text.parse::<CivilDate>().is_err(), "{text}");
    }
    assert_eq!(date("1970-01-01").days_since_epoch(), 0);
    assert_eq!(date("1969-12-31").days_since_epoch(), -1);
    assert_eq!(date("0001-01-01").days_since_epoch(), -719162);
    assert_eq!(date("9999-12-31").days_since_epoch(), 2932896);
}
#[test]
fn inclusive_coverage_is_bounded_and_crosses_years() {
    assert!(JudicialCalendarCoverage::new(date("9999-12-31"), date("9999-12-31")).is_ok());
    assert!(JudicialCalendarCoverage::new(date("2024-01-01"), date("2026-12-31")).is_ok());
    assert!(JudicialCalendarCoverage::new(date("2024-01-01"), date("2027-01-01")).is_err());
    assert!(JudicialCalendarCoverage::new(date("2026-01-02"), date("2026-01-01")).is_err());
}
#[test]
fn exceptions_override_complete_weekly_rules_without_fallback() {
    let exception = JudicialCalendarException::new(
        uuid::Uuid::nil(),
        date("2026-01-03"),
        date("2026-01-04"),
        JudicialCalendarRule::new(
            JudicialCalendarClassification::Unresolved,
            vec![],
            "Conditional source requires review",
        )
        .unwrap(),
    )
    .unwrap();
    let calendar = values(vec![exception]);
    let outside = calendar.classify(date("2025-12-31"));
    assert_eq!(outside.classification(), None);
    assert_eq!(outside.origin(), None);
    assert_eq!(outside.explanation(), None);
    assert!(outside.source_ids().is_empty());
    let weekday = calendar.classify(date("2026-01-02"));
    assert_eq!(
        weekday.classification(),
        Some(JudicialCalendarClassification::Countable)
    );
    assert_eq!(
        weekday.origin(),
        Some(JudicialCalendarDayOrigin::WeeklyPattern(5))
    );
    let special = calendar.classify(date("2026-01-03"));
    assert_eq!(
        special.classification(),
        Some(JudicialCalendarClassification::Unresolved)
    );
    assert_eq!(
        special.origin(),
        Some(JudicialCalendarDayOrigin::Exception(uuid::Uuid::nil()))
    );
    assert!(special.source_ids().is_empty());
    assert_eq!(
        calendar.classify(date("2026-01-11")).classification(),
        Some(JudicialCalendarClassification::Excluded)
    );
}
