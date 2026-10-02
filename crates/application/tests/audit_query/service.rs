use application::{audit_query::*, identity::Principal, ApplicationError};
use domain::{
    audit::AuditEvent,
    clock::OffsetDateTime,
    identity::{Role, UserId},
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use time::Duration;

type Trace = Arc<Mutex<Vec<&'static str>>>;
type Capture = Arc<Mutex<Option<(Principal, AuditEventQuery, OffsetDateTime)>>>;
struct Store {
    trace: Trace,
    capture: Capture,
    result: Mutex<Option<Result<AuditEventBatch, ApplicationError>>>,
}
impl AuditEventStore for Store {
    fn read(
        &self,
        actor: &Principal,
        query: &AuditEventQuery,
        at: OffsetDateTime,
    ) -> Result<AuditEventBatch, ApplicationError> {
        self.trace.lock().unwrap().push("store");
        *self.capture.lock().unwrap() = Some((actor.clone(), query.clone(), at));
        self.result.lock().unwrap().take().unwrap()
    }
}
fn actor(role: Role) -> Principal {
    Principal {
        id: UserId::new(),
        email: "owner@example.test".into(),
        role,
    }
}
fn at() -> OffsetDateTime {
    super::case_support::instant()
}
fn query(limit: u32, cursor: Option<&str>) -> AuditEventQuery {
    AuditEventQuery::new(
        at(),
        at() + Duration::days(1),
        None,
        None,
        None,
        limit,
        cursor,
    )
    .unwrap()
}
fn event(sequence: u64, nanos: u32) -> AuditEvent {
    AuditEvent::new(
        sequence,
        at().replace_nanosecond(nanos).unwrap(),
        "system",
        "created",
        "record",
    )
}
fn batch(events: Vec<AuditEvent>, more: bool) -> AuditEventBatch {
    AuditEventBatch {
        snapshot_max_sequence: Some(20),
        events,
        has_more: more,
    }
}
fn workflow(
    replies: Vec<Result<Principal, ApplicationError>>,
    result: Result<AuditEventBatch, ApplicationError>,
) -> (AuditEventService, Trace, Capture) {
    let trace = Arc::new(Mutex::new(Vec::new()));
    let capture = Arc::new(Mutex::new(None));
    let auth_trace = trace.clone();
    let replies = Mutex::new(VecDeque::from(replies));
    let mut identity = super::case_support::MockIdentity::new();
    identity
        .expect_authenticate()
        .withf(|token| token == "session")
        .times(0..=2)
        .returning(move |_| {
            auth_trace.lock().unwrap().push("auth");
            replies.lock().unwrap().pop_front().unwrap()
        });
    let store = Arc::new(Store {
        trace: trace.clone(),
        capture: capture.clone(),
        result: Mutex::new(Some(result)),
    });
    (
        AuditEventService::new(
            store,
            Arc::new(identity),
            Arc::new(super::case_support::CountingClock::default()),
        ),
        trace,
        capture,
    )
}
#[test]
fn owner_receives_exact_records_and_a_filter_bound_next_cursor() {
    let owner = actor(Role::Owner);
    let events = vec![event(7, 123_456_788), event(0, 123_456_789)];
    let q = query(2, None);
    let (service, trace, capture) = workflow(
        vec![Ok(owner.clone()), Ok(owner.clone())],
        Ok(batch(events.clone(), true)),
    );
    let page = service.read("session", q.clone()).unwrap();
    assert_eq!(page.events, events);
    assert_eq!(page.checked_at, at());
    assert_eq!(page.snapshot_max_sequence, Some(20));
    assert!(page.has_more);
    assert_eq!(
        page.next_cursor,
        Some(q.cursor_after(20, &events[1]).unwrap())
    );
    assert_eq!(*trace.lock().unwrap(), ["auth", "store", "auth"]);
    assert_eq!(*capture.lock().unwrap(), Some((owner, q, at())));
}
#[test]
fn denied_roles_and_invalid_session_never_read_even_an_empty_log() {
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        let (service, trace, _) = workflow(vec![Ok(actor(role))], Ok(batch(vec![], false)));
        assert!(matches!(
            service.read("session", query(20, None)),
            Err(ApplicationError::PermissionDenied)
        ));
        assert_eq!(*trace.lock().unwrap(), ["auth"]);
    }
    let (service, trace, _) = workflow(
        vec![Err(ApplicationError::InvalidSession)],
        Ok(batch(vec![], false)),
    );
    assert!(matches!(
        service.read("session", query(20, None)),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(*trace.lock().unwrap(), ["auth"]);
}
#[test]
fn empty_log_and_empty_filtered_page_are_authorized_and_complete() {
    for maximum in [None, Some(0), Some(20)] {
        let owner = actor(Role::Owner);
        let value = AuditEventBatch {
            snapshot_max_sequence: maximum,
            events: vec![],
            has_more: false,
        };
        let (service, trace, _) = workflow(vec![Ok(owner.clone()), Ok(owner)], Ok(value));
        let page = service.read("session", query(20, None)).unwrap();
        assert_eq!(page.snapshot_max_sequence, maximum);
        assert!(page.events.is_empty());
        assert!(!page.has_more);
        assert_eq!(page.next_cursor, None);
        assert_eq!(*trace.lock().unwrap(), ["auth", "store", "auth"]);
    }
}
#[test]
fn revoked_or_changed_full_principal_prevents_disclosure() {
    let owner = actor(Role::Owner);
    for change in 0..4 {
        let mut changed = owner.clone();
        match change {
            0 => changed.id = UserId::new(),
            1 => changed.email = "other@example.test".into(),
            2 => changed.role = Role::Litigator,
            _ => (),
        }
        let reply = if change == 3 {
            Err(ApplicationError::InvalidSession)
        } else {
            Ok(changed)
        };
        let (service, trace, _) =
            workflow(vec![Ok(owner.clone()), reply], Ok(batch(vec![], false)));
        assert!(matches!(
            service.read("session", query(20, None)),
            Err(ApplicationError::InvalidSession)
        ));
        assert_eq!(*trace.lock().unwrap(), ["auth", "store", "auth"]);
    }
}
#[test]
fn store_failure_stays_distinct_and_does_not_disclose_or_reauthenticate() {
    let (service, trace, _) = workflow(
        vec![Ok(actor(Role::Owner))],
        Err(ApplicationError::Port("offline".into())),
    );
    assert!(
        matches!(service.read("session", query(20, None)), Err(ApplicationError::Port(message)) if message == "offline")
    );
    assert_eq!(*trace.lock().unwrap(), ["auth", "store"]);
}
#[test]
fn malformed_snapshot_page_length_time_order_or_identity_is_rejected() {
    for change in 0..12 {
        let mut value = batch(vec![event(0, 0), event(1, 1)], false);
        match change {
            0 => value.snapshot_max_sequence = None,
            1 => value.snapshot_max_sequence = Some(i64::MAX as u64 + 1),
            2 => value.events.push(event(2, 2)),
            3 => {
                value.has_more = true;
                value.events.pop();
            }
            4 => {
                value.has_more = true;
                value.events.clear();
            }
            5 => value.events[1].sequence = 21,
            6 => value.events[0].timestamp = at() - Duration::nanoseconds(1),
            7 => value.events[1].timestamp = at() + Duration::days(1),
            8 => value.events.swap(0, 1),
            9 => value.events[1] = value.events[0].clone(),
            10 => value.events[1].sequence = 0,
            _ => {
                value.events[0].sequence = 1;
                value.events[1].sequence = 0;
                value.events[1].timestamp = value.events[0].timestamp;
            }
        }
        let (service, _, _) = workflow(vec![Ok(actor(Role::Owner))], Ok(value));
        assert!(
            matches!(
                service.read("session", query(2, None)),
                Err(ApplicationError::Port(_))
            ),
            "change {change}"
        );
    }
}
#[test]
fn mismatched_exact_filters_from_storage_are_rejected() {
    let q = AuditEventQuery::new(
        at(),
        at() + Duration::days(1),
        Some("system"),
        Some("created"),
        Some("record"),
        20,
        None,
    )
    .unwrap();
    for change in 0..3 {
        let mut value = event(0, 0);
        match change {
            0 => value.actor = "SYSTEM".into(),
            1 => value.action = "created ".into(),
            _ => value.resource = "other".into(),
        }
        let (service, _, _) = workflow(vec![Ok(actor(Role::Owner))], Ok(batch(vec![value], false)));
        assert!(matches!(
            service.read("session", q.clone()),
            Err(ApplicationError::Port(_))
        ));
    }
}
#[test]
fn continuation_rejects_snapshot_changes_and_replayed_or_earlier_positions() {
    let token = query(2, None).cursor_after(20, &event(5, 5)).unwrap();
    for change in 0..4 {
        let mut value = batch(vec![event(6, 6)], false);
        match change {
            0 => value.snapshot_max_sequence = Some(21),
            1 => value.snapshot_max_sequence = None,
            2 => value.events[0] = event(5, 5),
            _ => value.events[0] = event(19, 4),
        }
        let (service, _, _) = workflow(vec![Ok(actor(Role::Owner))], Ok(value));
        assert!(matches!(
            service.read("session", query(2, Some(&token))),
            Err(ApplicationError::Port(_))
        ));
    }
}
#[test]
fn chronological_continuation_preserves_nanoseconds_and_sequence_tie_break() {
    let token = query(2, None)
        .cursor_after(20, &event(5, 123_456_788))
        .unwrap();
    let owner = actor(Role::Owner);
    let records = vec![event(6, 123_456_788), event(0, 123_456_789)];
    let (service, _, _) = workflow(
        vec![Ok(owner.clone()), Ok(owner)],
        Ok(batch(records.clone(), false)),
    );
    let page = service.read("session", query(2, Some(&token))).unwrap();
    assert_eq!(page.events, records);
    assert!(!page.has_more);
    assert_eq!(page.next_cursor, None);
}
#[test]
fn historical_strings_are_not_truncated_or_subject_to_filter_input_limits() {
    let owner = actor(Role::Owner);
    let mut record = event(0, 0);
    record.actor = "a".repeat(255);
    record.action = "b".repeat(129);
    record.resource = "\u{e9}\n<script>".repeat(1000);
    let (service, _, _) = workflow(
        vec![Ok(owner.clone()), Ok(owner)],
        Ok(batch(vec![record.clone()], false)),
    );
    assert_eq!(
        service.read("session", query(20, None)).unwrap().events,
        [record]
    );
}
#[test]
fn aggregate_text_capacity_is_checked_without_a_partial_or_truncated_page() {
    for extra in [0, 1] {
        let owner = actor(Role::Owner);
        let mut record = event(0, 0);
        record.resource = "r"
            .repeat(MAX_AUDIT_PAGE_TEXT_BYTES - record.actor.len() - record.action.len() + extra);
        let replies = if extra == 0 {
            vec![Ok(owner.clone()), Ok(owner)]
        } else {
            vec![Ok(owner)]
        };
        let (service, trace, _) = workflow(replies, Ok(batch(vec![record.clone()], false)));
        let result = service.read("session", query(20, None));
        if extra == 0 {
            assert_eq!(result.unwrap().events, [record]);
        } else {
            assert!(matches!(
                result,
                Err(ApplicationError::AuditQueryCapacityExceeded)
            ));
            assert_eq!(*trace.lock().unwrap(), ["auth", "store"]);
        }
    }
}
