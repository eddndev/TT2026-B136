use application::hearings::{
    HearingAgendaCursor, HearingAgendaQuery, HearingHistoryQuery, HearingQuery, HearingStatusFilter,
};
use domain::hearings::{HearingId, HearingTime};
use time::{macros::datetime, Duration};

#[test]
fn page_sizes_and_history_boundaries_are_bounded() {
    for limit in [0, 101, u32::MAX] {
        assert!(HearingQuery::new(limit, None, HearingStatusFilter::All).is_err());
        assert!(HearingHistoryQuery::new(limit, None).is_err());
    }
    for limit in [1, 20, 100] {
        assert_eq!(
            HearingQuery::new(limit, None, HearingStatusFilter::All)
                .unwrap()
                .limit(),
            limit
        );
        assert_eq!(
            HearingHistoryQuery::new(limit, Some(1))
                .unwrap()
                .before_revision()
                .unwrap()
                .get(),
            1
        );
    }
    assert!(HearingHistoryQuery::new(20, Some(0)).is_err());
}

#[test]
fn agenda_interval_is_positive_and_at_most_366_exact_days() {
    let from = datetime!(2026-09-15 00:00 UTC);
    let until = from + Duration::days(366);
    assert!(HearingAgendaQuery::new(20, from, until, None, HearingStatusFilter::Scheduled).is_ok());
    for end in [
        from,
        from - Duration::seconds(1),
        until + Duration::seconds(1),
    ] {
        assert!(
            HearingAgendaQuery::new(20, from, end, None, HearingStatusFilter::Scheduled).is_err()
        );
    }
    assert!(HearingAgendaQuery::new(0, from, until, None, HearingStatusFilter::All).is_err());
    assert!(HearingAgendaQuery::new(101, from, until, None, HearingStatusFilter::All).is_err());
}

#[test]
fn agenda_requires_utc_whole_seconds_and_cursor_inside_half_open_interval() {
    let from = datetime!(2026-09-15 00:00 UTC);
    let until = from + Duration::days(1);
    for at in [from - Duration::seconds(1), until] {
        let cursor = HearingAgendaCursor {
            at: HearingTime::new(at).unwrap(),
            id: HearingId::new(),
        };
        assert!(
            HearingAgendaQuery::new(20, from, until, Some(cursor), HearingStatusFilter::All)
                .is_err()
        );
    }
    let cursor = HearingAgendaCursor {
        at: HearingTime::new(from).unwrap(),
        id: HearingId::new(),
    };
    assert_eq!(
        HearingAgendaQuery::new(20, from, until, Some(cursor), HearingStatusFilter::All)
            .unwrap()
            .after(),
        Some(cursor)
    );
    assert!(HearingAgendaQuery::new(
        20,
        from + Duration::nanoseconds(1),
        until,
        None,
        HearingStatusFilter::All
    )
    .is_err());
    let local = from.to_offset(time::UtcOffset::from_hms(-6, 0, 0).unwrap());
    assert!(HearingAgendaQuery::new(20, local, until, None, HearingStatusFilter::All).is_err());
}
