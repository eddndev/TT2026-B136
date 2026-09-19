mod alert_support;
use alert_support::*;
use application::{alerts::*, ApplicationError};
use domain::identity::UserId;
use time::{macros::datetime, Duration};

#[test]
fn limits_and_cursors_bind_filters_preserving_nanosecond_order() {
    for limit in [0, 101, u32::MAX] {
        assert!(matches!(
            AlertQuery::new(limit, AlertReadFilter::All, AlertStateFilter::Active, None),
            Err(ApplicationError::Alert(AlertError::Invalid(_)))
        ));
    }
    for limit in [1, 100] {
        assert_eq!(
            AlertQuery::new(limit, AlertReadFilter::All, AlertStateFilter::Active, None)
                .unwrap()
                .limit(),
            limit
        );
    }
    let cursor = AlertCursor::new(
        at(),
        id(1),
        AlertReadFilter::Unread,
        AlertStateFilter::Active,
    )
    .unwrap();
    let encoded = cursor.encode();
    assert_eq!(AlertCursor::parse(&encoded).unwrap(), cursor);
    assert_eq!(cursor.created_at().nanosecond(), 123_456_789);
    assert!(encoded.is_ascii() && encoded.len() <= 128);
    for (read, state) in [
        (AlertReadFilter::All, AlertStateFilter::Active),
        (AlertReadFilter::Unread, AlertStateFilter::All),
    ] {
        assert!(AlertQuery::new(20, read, state, Some(cursor)).is_err());
    }
    for malformed in [
        format!(" {encoded}"),
        format!("{encoded}:extra"),
        "x".repeat(129),
    ] {
        assert!(AlertCursor::parse(&malformed).is_err());
    }
}

#[test]
fn descending_pages_reject_duplicate_ids_wrong_filters_and_future_records() {
    let actor = UserId::new();
    let newer = record(actor, 2);
    let mut older = record(actor, 1);
    older.created_at -= Duration::nanoseconds(1);
    older.trigger_at = older.created_at;
    older.kind = AlertKind::OverdueUnattended {
        due_at: older.created_at,
    };
    let valid = AlertPage {
        checked_at: at(),
        alerts: vec![newer.clone(), older],
        has_more: false,
        next_cursor: None,
    };
    assert!(valid.validate(actor, &query()).is_ok());
    for invalid in 0..5 {
        let mut page = valid.clone();
        match invalid {
            0 => page.alerts.reverse(),
            1 => page.alerts[1].id = page.alerts[0].id,
            2 => page.alerts[0].recipient_id = UserId::new(),
            3 => page.checked_at -= Duration::nanoseconds(1),
            _ => {
                page.alerts[0].state = AlertState::Resolved {
                    at: at(),
                    reason: AlertResolutionReason::Superseded,
                }
            }
        }
        assert!(page.validate(actor, &query()).is_err(), "case {invalid}");
    }
}

#[test]
fn empty_bounded_pages_continue_without_claiming_that_the_inbox_is_exhausted() {
    let actor = UserId::new();
    let cursor =
        AlertCursor::new(at(), id(2), AlertReadFilter::All, AlertStateFilter::Active).unwrap();
    let mut page = AlertPage {
        checked_at: at(),
        alerts: vec![],
        has_more: true,
        next_cursor: Some(cursor),
    };
    assert!(page.validate(actor, &query()).is_ok());
    page.has_more = false;
    assert!(page.validate(actor, &query()).is_err());
    page.has_more = true;
    page.next_cursor = None;
    assert!(page.validate(actor, &query()).is_err());
    page.next_cursor = Some(cursor);
    let repeated = AlertQuery::new(
        20,
        AlertReadFilter::All,
        AlertStateFilter::Active,
        Some(cursor),
    )
    .unwrap();
    assert!(page.validate(actor, &repeated).is_err());
}

#[test]
fn alert_evidence_rejects_wrong_family_and_impossible_lifecycle_timestamps() {
    let actor = UserId::new();
    let value = record(actor, 1);
    assert!(value.validate(actor, at()).is_ok());
    for invalid in 0..5 {
        let mut alert = value.clone();
        match invalid {
            0 => alert.origin.revision = 0,
            1 => alert.read_at = Some(at() - Duration::seconds(1)),
            2 => alert.trigger_at = at() + Duration::nanoseconds(1),
            3 => {
                alert.email = AlertEmailStatus::Accepted {
                    accepted_at: at() + Duration::seconds(1),
                }
            }
            _ => {
                alert.subject = AlertSubject::Hearing {
                    case_id: alert.subject.case_id(),
                    id: application::hearings::HearingId::from_uuid(uuid::Uuid::nil()),
                }
            }
        }
        assert!(alert.validate(actor, at()).is_err(), "case {invalid}");
    }
    assert!(AlertCursor::new(
        datetime!(0000-01-01 00:00 UTC),
        id(1),
        AlertReadFilter::All,
        AlertStateFilter::Active
    )
    .is_err());
}

#[test]
fn captured_context_requires_exact_normalized_bounded_single_line_text() {
    let actor = UserId::new();
    let value = record(actor, 1);
    assert!(value.validate(actor, at()).is_ok());
    for field in 0..3 {
        let limit = if field == 2 { 100 } else { 200 };
        let mut boundary = value.clone();
        let text = match field {
            0 => &mut boundary.subject_title,
            1 => &mut boundary.case_title,
            _ => &mut boundary.case_reference,
        };
        *text = "x".repeat(limit);
        assert!(boundary.validate(actor, at()).is_ok());
        for invalid in [
            String::new(),
            "   ".into(),
            " title ".into(),
            "bad\ntext".into(),
            "x".repeat(limit + 1),
        ] {
            let mut alert = value.clone();
            match field {
                0 => alert.subject_title = invalid,
                1 => alert.case_title = invalid,
                _ => alert.case_reference = invalid,
            }
            assert!(alert.validate(actor, at()).is_err(), "field {field}");
        }
    }
}
