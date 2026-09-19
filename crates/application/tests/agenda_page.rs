mod agenda_support;

use agenda_support::*;
use application::{agenda::*, hearings::*};
use time::{Duration, UtcOffset};

#[test]
fn complete_pages_accept_empty_or_bounded_ordered_hearings() {
    assert!(page(vec![]).validate(&query(1)).is_ok());
    let mut first = hearing(from(), 1);
    first.case_status = domain::case_administration::CaseAdministrativeStatus::Closed;
    let result = page(vec![AgendaItem::Hearing(first), item(2)]);
    assert!(result.validate(&query(2)).is_ok());
    assert_eq!(result.items[0].key().unwrap(), cursor(1));
}

#[test]
fn an_empty_incomplete_page_advances_over_examined_candidates() {
    let first = AgendaPage {
        checked_at: checked_at(),
        items: vec![],
        complete: false,
        next_after: Some(cursor(10)),
    };
    assert!(first.validate(&query(1)).is_ok());
    let next = AgendaQuery::new(
        1,
        from(),
        until(),
        AgendaKind::All,
        HearingStatusFilter::Scheduled,
        first.next_after,
    )
    .unwrap();
    assert!(page(vec![item(11)]).validate(&next).is_ok());
    let last = page(vec![]);
    assert!(last.validate(&next).is_ok());
}

#[test]
fn continuation_can_follow_the_last_returned_row_but_cannot_precede_it() {
    let q = AgendaQuery::new(
        2,
        from(),
        until(),
        AgendaKind::All,
        HearingStatusFilter::Scheduled,
        Some(cursor(1)),
    )
    .unwrap();
    for after in [cursor(3), cursor(5)] {
        let result = AgendaPage {
            checked_at: checked_at(),
            items: vec![item(2), item(3)],
            complete: false,
            next_after: Some(after),
        };
        assert!(result.validate(&q).is_ok());
    }
    let result = AgendaPage {
        checked_at: checked_at(),
        items: vec![item(3)],
        complete: false,
        next_after: Some(cursor(2)),
    };
    assert!(result.validate(&q).is_err());
}

#[test]
fn size_completion_and_progress_must_agree() {
    let q = AgendaQuery::new(
        1,
        from(),
        until(),
        AgendaKind::All,
        HearingStatusFilter::Scheduled,
        Some(cursor(1)),
    )
    .unwrap();
    for result in [
        page(vec![item(2), item(3)]),
        AgendaPage {
            checked_at: checked_at(),
            items: vec![],
            complete: true,
            next_after: Some(cursor(2)),
        },
        AgendaPage {
            checked_at: checked_at(),
            items: vec![],
            complete: false,
            next_after: None,
        },
        AgendaPage {
            checked_at: checked_at(),
            items: vec![],
            complete: false,
            next_after: Some(cursor(1)),
        },
        AgendaPage {
            checked_at: checked_at(),
            items: vec![],
            complete: false,
            next_after: Some(cursor(0)),
        },
    ] {
        assert!(result.validate(&q).is_err());
    }
    let beyond = AgendaCursor::new(until(), AgendaItemKind::Hearing, uuid::Uuid::nil()).unwrap();
    assert!(AgendaPage {
        checked_at: checked_at(),
        items: vec![],
        complete: false,
        next_after: Some(beyond)
    }
    .validate(&q)
    .is_err());
}

#[test]
fn page_rejects_order_duplicates_exclusive_cursor_and_time_boundaries() {
    let q = AgendaQuery::new(
        3,
        from(),
        until(),
        AgendaKind::All,
        HearingStatusFilter::Scheduled,
        Some(cursor(1)),
    )
    .unwrap();
    for items in [
        vec![item(3), item(2)],
        vec![item(2), item(2)],
        vec![item(1)],
        vec![AgendaItem::Hearing(hearing(
            from() - Duration::seconds(1),
            2,
        ))],
        vec![AgendaItem::Hearing(hearing(until(), 2))],
        vec![
            item(2),
            AgendaItem::Hearing(hearing(from() + Duration::seconds(1), 2)),
        ],
    ] {
        assert!(page(items).validate(&q).is_err());
    }
}

#[test]
fn hearing_filters_and_canonical_case_metadata_are_enforced() {
    let mut cancelled = hearing(from(), 1);
    cancelled.status = HearingStatus::Cancelled;
    assert!(page(vec![AgendaItem::Hearing(cancelled.clone())])
        .validate(&query(1))
        .is_err());
    let cancelled_query = AgendaQuery::new(
        1,
        from(),
        until(),
        AgendaKind::Hearing,
        HearingStatusFilter::Cancelled,
        None,
    )
    .unwrap();
    assert!(page(vec![AgendaItem::Hearing(cancelled)])
        .validate(&cancelled_query)
        .is_ok());
    let deadlines = AgendaQuery::new(
        1,
        from(),
        until(),
        AgendaKind::Deadline,
        HearingStatusFilter::Scheduled,
        None,
    )
    .unwrap();
    assert!(page(vec![item(1)]).validate(&deadlines).is_err());
    for change in 0..6 {
        let mut row = hearing(from(), 1);
        match change {
            0 => row.case_title.clear(),
            1 => row.case_reference = " ".into(),
            2 => row.case_title = "x".repeat(201),
            3 => row.case_reference = "x".repeat(101),
            4 => row.case_title = " padded".into(),
            _ => row.case_reference = "bad\nreference".into(),
        }
        assert!(page(vec![AgendaItem::Hearing(row)])
            .validate(&query(1))
            .is_err());
    }
    let mut maximum = hearing(from(), 1);
    maximum.case_title = "x".repeat(200);
    maximum.case_reference = "r".repeat(100);
    assert!(page(vec![AgendaItem::Hearing(maximum)])
        .validate(&query(1))
        .is_ok());
}

#[test]
fn page_timestamp_is_a_canonical_utc_observation_not_the_event_date() {
    assert!(page(vec![item(1)]).validate(&query(1)).is_ok());
    let mut value = page(vec![item(1)]);
    value.checked_at = checked_at().to_offset(UtcOffset::from_hms(1, 0, 0).unwrap());
    assert!(value.validate(&query(1)).is_err());
}
