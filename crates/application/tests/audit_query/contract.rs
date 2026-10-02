use application::{audit_query::*, ApplicationError};
use domain::{audit::AuditEvent, clock::OffsetDateTime};
use time::{Duration, UtcOffset};

fn start() -> OffsetDateTime {
    super::case_support::instant()
}
fn query(
    actor: Option<&str>,
    action: Option<&str>,
    resource: Option<&str>,
    cursor: Option<&str>,
) -> Result<AuditEventQuery, ApplicationError> {
    AuditEventQuery::new(
        start(),
        start() + Duration::days(1),
        actor,
        action,
        resource,
        20,
        cursor,
    )
}
fn event() -> AuditEvent {
    AuditEvent::new(
        0,
        start().replace_nanosecond(123_456_789).unwrap(),
        "system",
        "created",
        "resource",
    )
}
fn invalid<T: std::fmt::Debug>(result: Result<T, ApplicationError>) {
    assert!(
        matches!(result, Err(ApplicationError::InvalidInput(_))),
        "{result:?}"
    );
}
#[test]
fn accepts_exact_utc_instants_and_preserves_all_nanoseconds() {
    let from = start().replace_nanosecond(123_456_789).unwrap();
    let query = AuditEventQuery::new(
        from.to_offset(UtcOffset::from_hms(5, 30, 0).unwrap()),
        from + Duration::nanoseconds(1),
        None,
        None,
        None,
        100,
        None,
    )
    .unwrap();
    assert_eq!(query.from(), from);
    assert_eq!(query.from().offset(), UtcOffset::UTC);
    assert_eq!(query.until() - query.from(), Duration::nanoseconds(1));
    assert_eq!(query.limit(), 100);
    assert_eq!(query.cursor(), None);
}
#[test]
fn rejects_empty_reversed_overlong_and_unformattable_ranges() {
    for (from, until) in [
        (start(), start()),
        (start(), start() - Duration::nanoseconds(1)),
        (
            start(),
            start() + Duration::days(366) + Duration::nanoseconds(1),
        ),
        (
            OffsetDateTime::from_unix_timestamp(-62_167_219_200).unwrap(),
            OffsetDateTime::from_unix_timestamp(-62_167_132_800).unwrap(),
        ),
    ] {
        invalid(AuditEventQuery::new(
            from, until, None, None, None, 20, None,
        ));
    }
    assert!(AuditEventQuery::new(
        start(),
        start() + Duration::days(366),
        None,
        None,
        None,
        20,
        None
    )
    .is_ok());
}
#[test]
fn rejects_zero_and_overlarge_limits() {
    for limit in [0, 101, u32::MAX] {
        invalid(AuditEventQuery::new(
            start(),
            start() + Duration::days(1),
            None,
            None,
            None,
            limit,
            None,
        ));
    }
}
#[test]
fn exact_filters_preserve_spaces_case_and_utf8_without_controls() {
    let q = query(
        Some(" Owner\u{e9} "),
        Some("Created"),
        Some(" a:b/- "),
        None,
    )
    .unwrap();
    assert_eq!(q.actor(), Some(" Owner\u{e9} "));
    assert_eq!(q.action(), Some("Created"));
    assert_eq!(q.resource(), Some(" a:b/- "));
    for value in ["", "new\nline", "tab\tvalue", "bad\u{7f}", "bad\u{85}"] {
        invalid(query(Some(value), None, None, None));
        invalid(query(None, Some(value), None, None));
        invalid(query(None, None, Some(value), None));
    }
}
#[test]
fn filter_bounds_count_utf8_bytes() {
    for (slot, max) in [(0, 254), (1, 128), (2, 1024)] {
        let valid = "a".repeat(max);
        let long = format!("{valid}b");
        let mut filters = [None; 3];
        filters[slot] = Some(valid.as_str());
        assert!(query(filters[0], filters[1], filters[2], None).is_ok());
        filters[slot] = Some(long.as_str());
        invalid(query(filters[0], filters[1], filters[2], None));
    }
    invalid(query(Some(&"\u{e9}".repeat(128)), None, None, None));
}
#[test]
fn cursor_roundtrip_keeps_snapshot_nanoseconds_and_zero_sequence() {
    let q = query(Some("system"), Some("created"), Some("resource"), None).unwrap();
    let token = q.cursor_after(i64::MAX as u64, &event()).unwrap();
    let restored = query(
        Some("system"),
        Some("created"),
        Some("resource"),
        Some(&token),
    )
    .unwrap();
    assert_eq!(
        restored.cursor(),
        Some(AuditEventCursor {
            snapshot_max_sequence: i64::MAX as u64,
            after: AuditEventPosition::of(&event()),
        })
    );
    let changed_limit = AuditEventQuery::new(
        q.from(),
        q.until(),
        q.actor(),
        q.action(),
        q.resource(),
        1,
        Some(&token),
    )
    .unwrap();
    assert_eq!(changed_limit.cursor(), restored.cursor());
}
#[test]
fn cursor_is_bound_to_every_exact_filter_and_time_boundary() {
    let q = query(Some("system"), Some("created"), Some("resource"), None).unwrap();
    let token = q.cursor_after(10, &event()).unwrap();
    for filters in [
        [Some("System"), q.action(), q.resource()],
        [q.actor(), None, q.resource()],
        [q.actor(), q.action(), Some("other")],
    ] {
        invalid(query(filters[0], filters[1], filters[2], Some(&token)));
    }
    for (from, until) in [
        (q.from() - Duration::nanoseconds(1), q.until()),
        (q.from(), q.until() + Duration::nanoseconds(1)),
    ] {
        invalid(AuditEventQuery::new(
            from,
            until,
            q.actor(),
            q.action(),
            q.resource(),
            20,
            Some(&token),
        ));
    }
}
#[test]
fn rejects_noncanonical_malformed_or_out_of_range_cursors() {
    let q = query(None, None, None, None).unwrap();
    let token = q.cursor_after(10, &event()).unwrap();
    for bad in [
        String::new(),
        "x".repeat(4097),
        "\u{e9}".into(),
        format!("{token}:extra"),
        token.replacen("aq1:", "aq2:", 1),
        token.replacen("aq1:10:", "aq1:010:", 1),
    ] {
        invalid(query(None, None, None, Some(&bad)));
    }
    for (index, replacement) in [
        (1, "9223372036854775808"),
        (2, "-9223372036854775808"),
        (3, "1000000000"),
        (4, "11"),
        (4, "-1"),
    ] {
        let mut parts: Vec<_> = token.split(':').collect();
        parts[index] = replacement;
        invalid(query(None, None, None, Some(&parts.join(":"))));
    }
}
#[test]
fn cursor_emission_rejects_an_inapplicable_last_event_or_snapshot() {
    let q = query(Some("system"), None, None, None).unwrap();
    let mut last = event();
    invalid(q.cursor_after(i64::MAX as u64 + 1, &last));
    last.sequence = 11;
    invalid(q.cursor_after(10, &last));
    last.sequence = 0;
    last.timestamp = q.until();
    invalid(q.cursor_after(10, &last));
    last.timestamp = event().timestamp;
    last.actor = "other".into();
    invalid(q.cursor_after(10, &last));
}
