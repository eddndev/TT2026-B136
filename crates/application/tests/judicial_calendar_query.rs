use application::judicial_calendars::*;

#[test]
fn query_limits_and_date_range_are_bounded() {
    for limit in [0, 101, u32::MAX] {
        assert!(JudicialCalendarQuery::new(
            limit,
            None,
            JudicialCalendarStatusFilter::Published,
            None,
            None
        )
        .is_err());
    }
    assert!(
        JudicialCalendarQuery::new(100, None, JudicialCalendarStatusFilter::All, None, None)
            .is_ok()
    );
    assert!(JudicialCalendarHistoryQuery::new(21, None).is_err());
    assert!(JudicialCalendarHistoryQuery::new(1, Some(0)).is_err());
    assert!(JudicialCalendarHistoryQuery::new(20, Some(2)).is_ok());
    let end = "9999-12-31".parse::<CivilDate>().unwrap();
    assert!(JudicialCalendarDaysQuery::new(end, end).is_ok());
    assert!(JudicialCalendarDaysQuery::new(
        "2028-01-01".parse::<CivilDate>().unwrap(),
        "2028-03-03".parse::<CivilDate>().unwrap()
    )
    .is_err());
}
