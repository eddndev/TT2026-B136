use application::{audit_query::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use domain::{audit::AuditEvent, clock::OffsetDateTime};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
const QUERY: &str = "?from=2026-10-01T00%3A00%3A00Z&until=2026-10-02T00%3A00%3A00Z";
struct Workflow {
    calls: Mutex<Vec<(String, AuditEventQuery)>>,
    result: Mutex<Option<Result<AuditEventPage, ApplicationError>>>,
}
impl AuditEventWorkflow for Workflow {
    fn read(
        &self,
        token: &str,
        query: AuditEventQuery,
    ) -> Result<AuditEventPage, ApplicationError> {
        self.calls.lock().unwrap().push((token.into(), query));
        self.result.lock().unwrap().take().unwrap()
    }
}
fn instant(raw: &str) -> OffsetDateTime {
    OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339).unwrap()
}
fn page() -> AuditEventPage {
    AuditEventPage {
        checked_at: instant("2026-10-02T01:02:03.123456789Z"),
        snapshot_max_sequence: Some(i64::MAX as u64),
        events: vec![AuditEvent::new(
            i64::MAX as u64,
            instant("2026-10-01T01:02:03.123456789Z"),
            "owner@example.test",
            "document.sealed",
            "literal <resource>",
        )],
        has_more: false,
        next_cursor: None,
    }
}
fn setup(result: Result<AuditEventPage, ApplicationError>) -> (Arc<Workflow>, Router) {
    let workflow = Arc::new(Workflow {
        calls: Mutex::new(Vec::new()),
        result: Mutex::new(Some(result)),
    });
    (workflow.clone(), web::audit_events_router(workflow))
}
async fn request(router: Router, suffix: &str, token: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::builder().uri(format!("/api/v1/audit/events{suffix}"));
    if let Some(token) = token {
        request = request.header("Authorization", token);
    }
    let response = router
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
#[tokio::test]
async fn exact_public_fields_preserve_nanoseconds_and_signed_64_bit_sequences() {
    let (workflow, router) = setup(Ok(page()));
    let suffix = "?from=2026-10-01T00:00:00.123456789%2B01:00&until=2026-10-02T00:00:00.987654321Z";
    let (status, body) = request(router, suffix, Some("Bearer owner-session")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["snapshot_max_sequence"], "9223372036854775807");
    assert_eq!(body["events"][0]["sequence"], "9223372036854775807");
    assert_eq!(
        body["events"][0]["timestamp"],
        "2026-10-01T01:02:03.123456789Z"
    );
    assert_eq!(body["checked_at"], "2026-10-02T01:02:03.123456789Z");
    assert_eq!(body["events"][0]["resource"], "literal <resource>");
    assert_eq!(body.as_object().unwrap().len(), 5);
    assert_eq!(body["events"][0].as_object().unwrap().len(), 5);
    let calls = workflow.calls.lock().unwrap();
    assert_eq!(calls[0].0, "owner-session");
    assert_eq!(calls[0].1.limit(), 20);
    assert_eq!(calls[0].1.from(), instant("2026-09-30T23:00:00.123456789Z"));
    assert_eq!(
        calls[0].1.until(),
        instant("2026-10-02T00:00:00.987654321Z")
    );
}
#[tokio::test]
async fn utf8_exact_filters_and_canonical_cursor_reach_the_workflow() {
    let from = instant("2026-10-01T00:00:00Z");
    let until = instant("2026-10-02T00:00:00Z");
    let query = AuditEventQuery::new(
        from,
        until,
        Some(" Owner\u{e9} "),
        Some("read"),
        Some("a:b/x"),
        1,
        None,
    )
    .unwrap();
    let last = AuditEvent::new(0, from, " Owner\u{e9} ", "read", "a:b/x");
    let cursor = query.cursor_after(10, &last).unwrap();
    let (workflow, router) = setup(Ok(AuditEventPage {
        checked_at: until,
        snapshot_max_sequence: Some(10),
        events: vec![],
        has_more: false,
        next_cursor: None,
    }));
    let suffix = format!(
        "{QUERY}&actor=+Owner%C3%A9+&action=read&resource=a%3Ab%2Fx&limit=1&cursor={cursor}"
    );
    assert_eq!(
        request(router, &suffix, Some("Bearer owner-session"))
            .await
            .0,
        StatusCode::OK
    );
    let calls = workflow.calls.lock().unwrap();
    assert_eq!(calls[0].1.actor(), Some(" Owner\u{e9} "));
    assert_eq!(calls[0].1.resource(), Some("a:b/x"));
    assert_eq!(calls[0].1.cursor().unwrap().snapshot_max_sequence, 10);
}
#[tokio::test]
async fn malformed_unknown_duplicate_or_unbounded_queries_never_invoke_the_workflow() {
    let invalid = [
        String::new(),
        "?from=bad&until=bad".into(),
        format!("{QUERY}&unknown=x"),
        format!("{QUERY}&actor=one&actor=two"),
        format!("{QUERY}&%61ctor=one&actor=two"),
        format!("{QUERY}&actor=%ZZ"),
        format!("{QUERY}&actor=%FF"),
        format!("{QUERY}&actor=%"),
        format!("{QUERY}&actor="),
        format!("{QUERY}&actor=%0A"),
        format!("{QUERY}&limit=0"),
        format!("{QUERY}&limit=101"),
        format!("{QUERY}&limit=01"),
        format!("{QUERY}&limit=%2B1"),
        format!("{QUERY}&limit=1.0"),
        format!("{QUERY}&cursor=x"),
        format!("{QUERY}&actor={}", "x".repeat(16384)),
        format!("{QUERY}&"),
        "?from=2026-10-01T00:00:60Z&until=2026-10-02T00:00:00Z".into(),
        "?from=2026-02-30T00:00:00Z&until=2026-10-02T00:00:00Z".into(),
        "?from=2026-10-01T00:00:00.1234567899Z&until=2026-10-02T00:00:00Z".into(),
        "?from=2026-10-01T00:00:00Z&until=2026-10-02T00:00:00.1234567899Z".into(),
    ];
    for suffix in invalid {
        let (workflow, router) = setup(Ok(page()));
        assert_eq!(
            request(router, &suffix, Some("Bearer owner-session"))
                .await
                .0,
            StatusCode::BAD_REQUEST,
            "{suffix}"
        );
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn absent_or_malformed_session_does_not_consult_private_records() {
    for token in [None, Some("Basic credential"), Some("Bearer ")] {
        let (workflow, router) = setup(Ok(page()));
        assert_eq!(
            request(router, QUERY, token).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn permission_revocation_and_capacity_failures_expose_no_events() {
    for (error, status, code) in [
        (
            ApplicationError::PermissionDenied,
            StatusCode::FORBIDDEN,
            "permission_denied",
        ),
        (
            ApplicationError::InvalidSession,
            StatusCode::UNAUTHORIZED,
            "invalid_session",
        ),
        (
            ApplicationError::AuditQueryCapacityExceeded,
            StatusCode::PAYLOAD_TOO_LARGE,
            "audit_query_capacity_exceeded",
        ),
    ] {
        let (_, router) = setup(Err(error));
        let (actual, body) = request(router, QUERY, Some("Bearer owner-session")).await;
        assert_eq!(actual, status);
        assert_eq!(body["error"]["code"], code);
        assert!(body.get("events").is_none());
        assert!(body.get("snapshot_max_sequence").is_none());
    }
}
#[tokio::test]
async fn transport_defensively_rejects_unbounded_workflow_output() {
    for change in 0..4 {
        let mut value = page();
        match change {
            0 => value.events = vec![value.events[0].clone(); 21],
            1 => value.snapshot_max_sequence = Some(i64::MAX as u64 + 1),
            2 => value.next_cursor = Some("x".repeat(4097)),
            _ => value.events[0].resource = "x".repeat(MAX_AUDIT_PAGE_TEXT_BYTES + 1),
        }
        let (_, router) = setup(Ok(value));
        let (status, body) = request(router, QUERY, Some("Bearer owner-session")).await;
        assert_eq!(
            status,
            if change == 3 {
                StatusCode::PAYLOAD_TOO_LARGE
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        );
        assert!(body.get("events").is_none());
    }
}
