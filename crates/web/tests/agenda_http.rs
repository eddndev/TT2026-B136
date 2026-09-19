use application::{agenda::*, cases::CaseAdministrativeStatus, hearings::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use domain::{cases::CaseId, clock::OffsetDateTime};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

const RANGE: &str = "from=2026-01-01T00:00:00Z&until=2026-01-02T00:00:00Z";
fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}
fn hearing() -> AgendaItem {
    AgendaItem::Hearing(HearingOverview {
        case_id: CaseId::from_uuid(Uuid::from_u128(1)),
        case_title: "Authorized case".into(),
        case_reference: "REF-1".into(),
        case_status: CaseAdministrativeStatus::Active,
        id: HearingId::from_uuid(Uuid::from_u128(2)),
        revision: HearingRevision::initial(),
        kind: HearingKind::Initial,
        scheduled_at: HearingTime::new(at(1767225601)).unwrap(),
        modality: HearingModality::InPerson,
        status: HearingStatus::Scheduled,
        participant_count: 1,
    })
}
struct Workflow {
    page: AgendaPage,
    calls: Mutex<Vec<(String, AgendaQuery)>>,
    denied: bool,
}
impl AgendaWorkflow for Workflow {
    fn list(&self, token: &str, query: AgendaQuery) -> Result<AgendaPage, ApplicationError> {
        self.calls.lock().unwrap().push((token.into(), query));
        if self.denied {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(self.page.clone())
    }
}
fn setup(page: AgendaPage) -> (Arc<Workflow>, Router) {
    let workflow = Arc::new(Workflow {
        page,
        calls: Mutex::new(Vec::new()),
        denied: false,
    });
    let router = web::agenda_router(workflow.clone());
    (workflow, router)
}
fn page() -> AgendaPage {
    AgendaPage {
        checked_at: at(1767225602),
        items: vec![hearing()],
        complete: true,
        next_after: None,
    }
}
async fn request(router: Router, query: &str, auth: bool) -> (StatusCode, Value) {
    let mut request = Request::builder().uri(format!("/api/v1/agenda?{query}"));
    if auth {
        request = request.header("Authorization", "Bearer agenda-session");
    }
    let response = router
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status();
    let body = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}
#[tokio::test]
async fn authorized_page_keeps_original_hearing_and_normalized_ordering_instant() {
    let (workflow, router) = setup(page());
    let (status, body) = request(router, RANGE, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["kind"], "all");
    assert_eq!(body["hearing_status"], "scheduled");
    assert_eq!(body["items"][0]["kind"], "hearing");
    assert_eq!(
        body["items"][0]["at"],
        serde_json::json!({"unix_seconds":1767225601i64,"nanosecond":0,"offset_seconds":0})
    );
    assert_eq!(body["items"][0]["hearing"]["case_title"], "Authorized case");
    assert_eq!(body["complete"], true);
    assert_eq!(body["next_cursor"], Value::Null);
    let calls = workflow.calls.lock().unwrap();
    assert_eq!(calls[0].0, "agenda-session");
    assert_eq!(calls[0].1.limit(), 20);
}
#[tokio::test]
async fn missing_authentication_never_calls_the_workflow() {
    let (workflow, router) = setup(page());
    assert_eq!(
        request(router, RANGE, false).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert!(workflow.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn malformed_and_ambiguous_queries_are_rejected_before_storage() {
    for suffix in [
        "&limit=0",
        "&limit=101",
        "&limit=1&limit=2",
        "&kind=unknown",
        "&kind=all&kind=deadline",
        "&hearing_status=unknown",
        "&kind=deadline&hearing_status=cancelled",
        "&after_id=123",
        "&cursor=broken",
        "&cursor=",
        "&from=2026-01-01T00:00:00Z",
    ] {
        let (workflow, router) = setup(page());
        assert_eq!(
            request(router, &format!("{RANGE}{suffix}"), true).await.0,
            StatusCode::BAD_REQUEST,
            "{suffix}"
        );
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
    for query in [
        "",
        "from=2026-01-01T00:00:00Z",
        "from=2026-01-01T00:00:00Z&until=2028-01-01T00:00:00Z",
        "from=2026-01-01T00:00:00.1Z&until=2026-01-02T00:00:00Z",
    ] {
        let (workflow, router) = setup(page());
        assert_eq!(
            request(router, query, true).await.0,
            StatusCode::BAD_REQUEST
        );
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn empty_partial_page_preserves_nanos_and_query_bound_continuation() {
    let mut value = page();
    value.items.clear();
    value.complete = false;
    value.next_after = Some(
        AgendaCursor::new(
            at(1767225601).replace_nanosecond(999999999).unwrap(),
            AgendaItemKind::Deadline,
            Uuid::from_u128(2),
        )
        .unwrap(),
    );
    let (_, router) = setup(value.clone());
    let (status, body) = request(router, RANGE, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"], serde_json::json!([]));
    assert_eq!(body["complete"], false);
    let cursor = body["next_cursor"].as_str().unwrap();
    assert!(cursor.is_ascii());
    assert!(cursor.len() <= 512);
    let mut exhausted = page();
    exhausted.items.clear();
    let (workflow, router) = setup(exhausted);
    assert_eq!(
        request(router, &format!("{RANGE}&cursor={cursor}"), true)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        workflow.calls.lock().unwrap()[0].1.after(),
        value.next_after
    );
    let (workflow, router) = setup(page());
    assert_eq!(
        request(
            router,
            &format!("{RANGE}&kind=hearing&cursor={cursor}"),
            true
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert!(workflow.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn inconsistent_page_does_not_disclose_resource_fields() {
    let mut outside = page();
    if let AgendaItem::Hearing(ref mut hearing) = outside.items[0] {
        hearing.scheduled_at = HearingTime::new(at(1767139200)).unwrap();
    }
    let mut unordered = page();
    unordered.items.push(hearing());
    let mut no_cursor = page();
    no_cursor.complete = false;
    for value in [outside, unordered, no_cursor] {
        let (_, router) = setup(value);
        let (status, body) = request(router, RANGE, true).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!body.to_string().contains("Authorized case"));
    }
}
#[tokio::test]
async fn workflow_access_denial_is_not_reported_as_an_empty_calendar() {
    let workflow = Arc::new(Workflow {
        page: page(),
        calls: Mutex::new(Vec::new()),
        denied: true,
    });
    let (status, body) = request(web::agenda_router(workflow), RANGE, true).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.get("items").is_none());
}
