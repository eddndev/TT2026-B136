use application::{dashboard::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use domain::{clock::OffsetDateTime, identity::UserId};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

struct Workflow {
    calls: Mutex<Vec<String>>,
    denied: bool,
}
impl DashboardWorkflow for Workflow {
    fn read(&self, token: &str) -> Result<DashboardSnapshot, ApplicationError> {
        self.calls.lock().unwrap().push(token.into());
        if self.denied {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(DashboardSnapshot {
            checked_at: OffsetDateTime::from_unix_timestamp(1790467200).unwrap(),
            scope: DashboardScope::AssignedCases,
            active_cases: 3,
            pending_contracts: 2,
            deadlines_overdue: 1,
            deadlines_due_48h: 2,
            deadlines_due_7d: 5,
            deadlines_unresolved: 4,
            workload: vec![DashboardWorkload {
                user_id: UserId::from_uuid(uuid::Uuid::from_u128(7)),
                email: "lawyer@example.test".into(),
                active_cases: 3,
            }],
        })
    }
}
fn setup(denied: bool) -> (Arc<Workflow>, Router) {
    let workflow = Arc::new(Workflow {
        calls: Mutex::new(Vec::new()),
        denied,
    });
    (workflow.clone(), web::dashboard_router(workflow))
}
async fn request(router: Router, suffix: &str, authenticated: bool) -> (StatusCode, Value) {
    let mut request = Request::builder().uri(format!("/api/v1/dashboard{suffix}"));
    if authenticated {
        request = request.header("Authorization", "Bearer dashboard-session");
    }
    let response = router
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn dashboard_exposes_authorized_snapshot_and_temporal_scope_without_secrets() {
    let (workflow, router) = setup(false);
    let (status, body) = request(router, "", true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["checked_at"], "2026-09-27T00:00:00Z");
    assert_eq!(body["scope"], "assigned_cases");
    assert_eq!(body["active_cases"], 3);
    assert_eq!(body["pending_contracts"], 2);
    assert_eq!(body["deadlines_overdue"], 1);
    assert_eq!(body["deadlines_due_48h"], 2);
    assert_eq!(body["deadlines_due_7d"], 5);
    assert_eq!(body["deadlines_unresolved"], 4);
    assert_eq!(body["workload"][0]["active_cases"], 3);
    assert_eq!(
        body["workload"][0]["user_id"],
        "00000000-0000-0000-0000-000000000007"
    );
    assert_eq!(body["workload"][0]["email"], "lawyer@example.test");
    assert_eq!(body.as_object().unwrap().len(), 9);
    assert_eq!(*workflow.calls.lock().unwrap(), ["dashboard-session"]);
}
#[tokio::test]
async fn missing_session_and_unsupported_filters_do_not_read_aggregates() {
    for (suffix, auth, expected) in [
        ("", false, StatusCode::UNAUTHORIZED),
        ("?case_id=foreign", true, StatusCode::BAD_REQUEST),
        ("?scope=office", true, StatusCode::BAD_REQUEST),
    ] {
        let (workflow, router) = setup(false);
        assert_eq!(request(router, suffix, auth).await.0, expected);
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn permission_denial_has_no_counters_or_workload() {
    let (_, router) = setup(true);
    let (status, body) = request(router, "", true).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "permission_denied");
    assert!(body.get("active_cases").is_none());
    assert!(body.get("workload").is_none());
}
