use application::{agenda::*, hearings::HearingStatusFilter};
use time::{macros::datetime, Duration, OffsetDateTime, UtcOffset};
use uuid::Uuid;

fn query(
    from: OffsetDateTime,
    until: OffsetDateTime,
) -> Result<AgendaQuery, application::ApplicationError> {
    AgendaQuery::new(
        20,
        from,
        until,
        AgendaKind::All,
        HearingStatusFilter::Scheduled,
        None,
    )
}

#[test]
fn periods_and_manual_range_preserve_exact_query_values() {
    let from = datetime!(2026-01-01 00:00 UTC);
    for days in [1, 7, 28, 29, 30, 31, 366] {
        let until = from + Duration::days(days);
        let q = query(from, until).unwrap();
        assert_eq!((q.from(), q.until(), q.limit()), (from, until, 20));
        assert_eq!(q.kind(), AgendaKind::All);
        assert_eq!(q.hearing_status(), HearingStatusFilter::Scheduled);
        assert_eq!(q.after(), None);
    }
}

#[test]
fn boundaries_require_bounded_positive_whole_second_utc_intervals() {
    let from = datetime!(2026-01-01 00:00 UTC);
    let until = from + Duration::days(1);
    for (start, end) in [
        (from, from),
        (until, from),
        (from, from + Duration::days(366) + Duration::seconds(1)),
        (from + Duration::nanoseconds(1), until),
        (from, until + Duration::nanoseconds(1)),
        (
            from.to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap()),
            until,
        ),
        (from, until.to_offset(UtcOffset::from_hms(1, 0, 0).unwrap())),
        (datetime!(0000-12-31 00:00 UTC), from),
    ] {
        assert!(query(start, end).is_err(), "accepted {start}..{end}");
    }
}

#[test]
fn output_limit_and_deadline_only_filter_are_explicit() {
    let from = datetime!(2026-01-01 00:00 UTC);
    let until = from + Duration::days(1);
    for limit in [0, 101, u32::MAX] {
        assert!(AgendaQuery::new(
            limit,
            from,
            until,
            AgendaKind::All,
            HearingStatusFilter::Scheduled,
            None
        )
        .is_err());
    }
    for limit in [1, 20, 100] {
        assert_eq!(
            AgendaQuery::new(
                limit,
                from,
                until,
                AgendaKind::Deadline,
                HearingStatusFilter::Scheduled,
                None
            )
            .unwrap()
            .limit(),
            limit
        );
    }
    for status in [HearingStatusFilter::Cancelled, HearingStatusFilter::All] {
        assert!(AgendaQuery::new(20, from, until, AgendaKind::Deadline, status, None).is_err());
        assert!(AgendaQuery::new(20, from, until, AgendaKind::Hearing, status, None).is_ok());
    }
    assert_eq!(MAX_AGENDA_CANDIDATES, 100);
}

#[test]
fn cursor_keeps_nanoseconds_kind_and_zero_uuid_without_rounding() {
    let at = datetime!(2026-01-01 12:00:00.123456789 UTC);
    let cursor = AgendaCursor::new(at, AgendaItemKind::Deadline, Uuid::nil()).unwrap();
    assert_eq!(cursor.at(), at);
    assert_eq!(cursor.at().nanosecond(), 123_456_789);
    assert_eq!(cursor.kind(), AgendaItemKind::Deadline);
    assert_eq!(cursor.id(), Uuid::nil());
    let same_hearing = AgendaCursor::new(at, AgendaItemKind::Hearing, Uuid::nil()).unwrap();
    assert!(same_hearing < cursor);
    let later = AgendaCursor::new(
        at + Duration::nanoseconds(1),
        AgendaItemKind::Hearing,
        Uuid::nil(),
    )
    .unwrap();
    assert!(cursor < later);
    let next_id = AgendaCursor::new(at, AgendaItemKind::Deadline, Uuid::from_u128(1)).unwrap();
    assert!(cursor < next_id);
}

#[test]
fn cursor_requires_utc_year_range_and_the_selected_interval_and_family() {
    let from = datetime!(2026-01-01 00:00 UTC);
    let until = from + Duration::days(1);
    for at in [
        datetime!(0000-12-31 00:00 UTC),
        from.to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
    ] {
        assert!(AgendaCursor::new(at, AgendaItemKind::Hearing, Uuid::nil()).is_err());
    }
    for at in [from - Duration::nanoseconds(1), until] {
        let cursor = AgendaCursor::new(at, AgendaItemKind::Deadline, Uuid::nil()).unwrap();
        assert!(AgendaQuery::new(
            20,
            from,
            until,
            AgendaKind::All,
            HearingStatusFilter::Scheduled,
            Some(cursor)
        )
        .is_err());
    }
    let cursor = AgendaCursor::new(from, AgendaItemKind::Deadline, Uuid::nil()).unwrap();
    assert!(AgendaQuery::new(
        20,
        from,
        until,
        AgendaKind::Hearing,
        HearingStatusFilter::Scheduled,
        Some(cursor)
    )
    .is_err());
    let q = AgendaQuery::new(
        20,
        from,
        until,
        AgendaKind::All,
        HearingStatusFilter::Scheduled,
        Some(cursor),
    )
    .unwrap();
    assert_eq!(q.after(), Some(cursor));
}
