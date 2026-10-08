use crate::password_reset_composition_support::{get, Harness};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use std::{num::NonZeroUsize, time::Duration};
use tower::ServiceExt;
use web::HttpWorkBudget;

const CASE: &str = "00000000-0000-0000-0000-000000000001";
const HEARING: &str = "00000000-0000-0000-0000-000000000002";
const OPERATION: &str = "00000000-0000-0000-0000-000000000003";
const NO_PROGRESS: Duration = Duration::from_millis(50);

fn context() -> String {
    format!("/api/v1/cases/{CASE}/precautionary-context")
}

fn base() -> String {
    format!("/api/v1/cases/{CASE}/precautionary-hearings")
}

fn calls() -> Vec<(String, Option<Value>)> {
    let command = json!({
        "case_id":CASE,"operation_id":OPERATION,"hearing_id":HEARING,
        "change":{"action":"schedule",
            "context":{"administration_revision":1,"stage_revision":1,"context_digest":"00".repeat(32)},
            "values":{"purpose":"imposition","scheduled_at":"2027-01-01T10:00:00-06:00",
                "modality":"in_person","venue":"Court","note":null,"participants":[],
                "scheduling_basis":{"statement":"Declared appointment","locator":"Page 1",
                    "support":{"document_id":"00000000-0000-0000-0000-000000000004","version":1,"digest":"01".repeat(32)}},
                "review_targets":[]}}
    });
    let administrative_base = format!("/api/v1/cases/{CASE}/measure-administrative-operations");
    let administrative = json!({
        "case_id":CASE,"operation_id":OPERATION,
        "target":{"id":HEARING,"revision":1,"capture_digest":"04".repeat(32)},
        "context":{"administration_revision":1,"stage_revision":1,"context_digest":"00".repeat(32)},
        "reason":"Incorrectly entered record","action":{"kind":"entered_in_error"}
    });
    let records = format!("/api/v1/cases/{CASE}/measures");
    let mut routes = vec![
        (context(), None),
        (base(), None),
        (format!("{}/{HEARING}", base()), None),
        (format!("{}/{HEARING}/revisions/1", base()), None),
        (format!("{}/operations/{OPERATION}", base()), None),
        (format!("{}/prepare", base()), Some(command.clone())),
        (
            format!("{}/submit", base()),
            Some(json!({"command":command,
            "expected_submission_digest":"02".repeat(32),"expected_review_digest":"03".repeat(32)})),
        ),
        (administrative_base.clone(), None),
        (format!("{administrative_base}/{OPERATION}"), None),
        (
            format!("{administrative_base}/prepare"),
            Some(administrative.clone()),
        ),
        (
            format!("{administrative_base}/submit"),
            Some(json!({"command":administrative,
                "expected_submission_digest":"02".repeat(32),"expected_review_digest":"03".repeat(32)})),
        ),
        (records.clone(), None),
        (format!("{records}/{HEARING}"), None),
        (
            format!(
                "{records}/{HEARING}/revisions/1?capture_digest={}",
                "04".repeat(32)
            ),
            None,
        ),
    ];
    routes.extend(decision_calls());
    routes
}

fn decision_calls() -> Vec<(String, Option<Value>)> {
    let base = format!("/api/v1/cases/{CASE}/measure-decisions");
    let command = json!({
        "case_id":CASE,"operation_id":OPERATION,"decision_id":HEARING,
        "context":{"administration_revision":1,"stage_revision":1,"context_digest":"00".repeat(32)},
        "values":{"authority":"Declared court","declared_at":{"precision":"unknown","reason":"Time not stated"},
            "justification":"Decision retained without measure changes","locator":"Page 1",
            "support":{"document_id":"00000000-0000-0000-0000-000000000004","version":1,"digest":"01".repeat(32)}},
        "anchor":null,"outcome":{"kind":"no_measure_change","statement":"No changes ordered"}
    });
    vec![
        (base.clone(), None),
        (format!("{base}/{HEARING}"), None),
        (format!("{base}/operations/{OPERATION}"), None),
        (format!("{base}/prepare"), Some(command.clone())),
        (
            format!("{base}/submit"),
            Some(json!({"command":command,
                "expected_submission_digest":"02".repeat(32),"expected_review_digest":"03".repeat(32)})),
        ),
    ]
}

async fn send(router: Router, path: String, body: Option<Value>, authorized: bool) -> (u16, Value) {
    let mut request = Request::builder()
        .method(if body.is_some() { "POST" } else { "GET" })
        .uri(path)
        .header("content-type", "application/json");
    if authorized {
        request = request.header("authorization", "Bearer dashboard-session");
    }
    let response = router
        .oneshot(
            request
                .body(Body::from(
                    body.map(|value| value.to_string()).unwrap_or_default(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

fn error(result: (u16, Value), status: u16, code: &str) {
    assert_eq!(result.0, status, "{}", result.1);
    assert_eq!(result.1["error"]["code"], code);
}

#[tokio::test]
async fn precautionary_routes_are_composed_with_bearer_and_no_store_boundaries() {
    for legacy in [false, true] {
        let fixture = Harness::new(2, legacy);
        for (path, body) in calls() {
            error(
                send(fixture.router.clone(), path.clone(), body.clone(), false).await,
                401,
                "invalid_session",
            );
            error(
                send(fixture.router.clone(), path, body, true).await,
                403,
                "permission_denied",
            );
        }
        assert_eq!(fixture.dashboard.calls(), 0);
        assert!(fixture.ports.calls().is_empty());
    }
}

#[tokio::test]
async fn precautionary_reads_and_mutations_share_existing_request_admission() {
    let fixture = Harness::new(1, false);
    let mut gate = fixture.dashboard.block();
    let router = fixture.router.clone();
    let dashboard = tokio::spawn(async move { get(&router, "/api/v1/dashboard").await });
    gate.entered().await;
    for (path, body) in calls() {
        error(
            send(fixture.router.clone(), path, body, true).await,
            503,
            "server_busy",
        );
    }
    drop(gate);
    dashboard
        .await
        .unwrap()
        .error(StatusCode::FORBIDDEN, "permission_denied");
    error(
        send(fixture.router, context(), None, true).await,
        403,
        "permission_denied",
    );
}

#[tokio::test]
async fn precautionary_ports_wait_for_the_injected_existing_blocking_budget() {
    let budget = HttpWorkBudget::new(NonZeroUsize::new(1).unwrap());
    let fixture = Harness::with_budget(2, 1, budget.clone()).unwrap();
    for (path, body) in calls() {
        let held = budget.acquire_owned().await.unwrap();
        let mut pending = tokio::spawn(send(fixture.router.clone(), path, body, true));
        assert!(
            tokio::time::timeout(NO_PROGRESS, &mut pending)
                .await
                .is_err(),
            "precautionary workflow escaped the shared blocking budget"
        );
        drop(held);
        let result = tokio::time::timeout(Duration::from_secs(2), pending)
            .await
            .unwrap()
            .unwrap();
        error(result, 403, "permission_denied");
    }
    assert_eq!(fixture.dashboard.calls(), 0);
    assert!(fixture.ports.calls().is_empty());
}
