use std::sync::{Arc, Mutex};

use application::cases::{CaseRecord, CaseWorkflow};
use application::ApplicationError;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use domain::cases::CaseId;
use domain::identity::UserId;
use domain::DomainError;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const CASE: &str = "00000000-0000-0000-0000-000000000001";
const USER: &str = "00000000-0000-0000-0000-000000000002";

#[derive(Default)]
struct Workflow {
    calls: Mutex<Vec<String>>,
}

impl Workflow {
    fn record(&self, token: &str, call: String) -> Result<CaseRecord, ApplicationError> {
        self.calls.lock().unwrap().push(format!("{token} {call}"));
        match token {
            "expired" => return Err(ApplicationError::InvalidSession),
            "forbidden" => return Err(ApplicationError::PermissionDenied),
            "hidden" => return Err(ApplicationError::CaseNotFound),
            "missing-user" => return Err(ApplicationError::UserNotFound),
            "database-failed" => return Err(ApplicationError::Port("secret DSN".into())),
            _ => {}
        }
        Ok(CaseRecord {
            id: CaseId::from_uuid(Uuid::from_u128(1)),
            title: "Example case".into(),
            reference: "REF-123".into(),
            created_by: UserId::from_uuid(Uuid::from_u128(2)),
        })
    }
}

impl CaseWorkflow for Workflow {
    fn create(
        &self,
        token: &str,
        title: &str,
        reference: &str,
    ) -> Result<CaseRecord, ApplicationError> {
        if title.is_empty() {
            return Err(DomainError::InvalidCaseMetadata {
                field: "title",
                reason: "must not be empty",
            }
            .into());
        }
        self.record(token, format!("create {title} {reference}"))
    }

    fn list(
        &self,
        token: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<CaseRecord>, ApplicationError> {
        Ok(vec![self.record(token, format!("list {limit} {offset}"))?])
    }

    fn get(&self, token: &str, id: CaseId) -> Result<CaseRecord, ApplicationError> {
        self.record(token, format!("get {id}"))
    }

    fn assign(&self, token: &str, id: CaseId, user_id: UserId) -> Result<(), ApplicationError> {
        self.record(token, format!("assign {id} {user_id}"))?;
        Ok(())
    }

    fn remove(&self, token: &str, id: CaseId, user_id: UserId) -> Result<(), ApplicationError> {
        self.record(token, format!("remove {id} {user_id}"))?;
        Ok(())
    }
}

async fn request(
    workflow: &Arc<Workflow>,
    method: &str,
    uri: &str,
    authorization: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, String) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(authorization) = authorization {
        request = request.header("authorization", authorization);
    }
    let body = match body {
        Some(value) => {
            request = request.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    let response = web::case_router(workflow.clone())
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 32_768).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn cases_route_methods_pass_the_bearer_token_and_return_the_contract() {
    let workflow = Arc::new(Workflow::default());
    let (status, body) = request(
        &workflow,
        "POST",
        "/api/v1/cases",
        Some("Bearer owner-token"),
        Some(json!({"title":"Example case", "reference":"REF-123"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap(),
        json!({"id": CASE, "title": "Example case", "reference": "REF-123", "created_by": USER})
    );
    let path = format!("/api/v1/cases/{CASE}");
    assert_eq!(
        request(&workflow, "GET", &path, Some("Bearer owner-token"), None)
            .await
            .0,
        StatusCode::OK
    );
    let path = format!("{path}/members/{USER}");
    for method in ["PUT", "DELETE"] {
        let (status, body) =
            request(&workflow, method, &path, Some("Bearer owner-token"), None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(body.is_empty());
    }
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![
            "owner-token create Example case REF-123".to_string(),
            format!("owner-token get {CASE}"),
            format!("owner-token assign {CASE} {USER}"),
            format!("owner-token remove {CASE} {USER}"),
        ]
    );
}

#[tokio::test]
async fn listing_preserves_pagination_and_defaults() {
    let workflow = Arc::new(Workflow::default());
    for path in ["/api/v1/cases", "/api/v1/cases?limit=12&offset=24"] {
        let (status, body) = request(&workflow, "GET", path, Some("Bearer member"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(serde_json::from_str::<Value>(&body).unwrap().is_array());
    }
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec!["member list 50 0", "member list 12 24"]
    );
}

#[tokio::test]
async fn malformed_bearer_headers_never_reach_the_workflow() {
    let workflow = Arc::new(Workflow::default());
    for header in [
        None,
        Some("Basic owner"),
        Some("Bearer "),
        Some("Bearer    "),
    ] {
        let (status, body) = request(&workflow, "GET", "/api/v1/cases", header, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap()["error"]["code"],
            "invalid_session"
        );
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn malformed_identifiers_return_stable_errors_before_application_calls() {
    let workflow = Arc::new(Workflow::default());
    for (method, path, code) in [
        ("GET", "/api/v1/cases/not-a-uuid".into(), "invalid_case_id"),
        (
            "PUT",
            format!("/api/v1/cases/bad/members/{USER}"),
            "invalid_case_id",
        ),
        (
            "DELETE",
            format!("/api/v1/cases/{CASE}/members/bad"),
            "invalid_user_id",
        ),
    ] {
        let (status, body) = request(&workflow, method, &path, Some("Bearer member"), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap()["error"]["code"],
            code
        );
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn malformed_pagination_never_reaches_the_workflow() {
    let workflow = Arc::new(Workflow::default());
    for query in ["limit=bad", "offset=-1", "offset=4294967296"] {
        let (status, _) = request(
            &workflow,
            "GET",
            &format!("/api/v1/cases?{query}"),
            Some("Bearer member"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn create_rejects_spoofed_identity_fields_and_oversized_bodies() {
    let workflow = Arc::new(Workflow::default());
    for field in ["actor", "created_by", "role"] {
        let mut body = json!({"title":"Example case", "reference":"REF-123"});
        body[field] = json!("owner");
        assert_eq!(
            request(
                &workflow,
                "POST",
                "/api/v1/cases",
                Some("Bearer member"),
                Some(body)
            )
            .await
            .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(
        request(
            &workflow,
            "POST",
            "/api/v1/cases",
            Some("Bearer member"),
            Some(json!({"title":"x".repeat(20_000), "reference":"REF-123"}))
        )
        .await
        .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn application_failures_keep_their_status_without_leaking_internals() {
    let workflow = Arc::new(Workflow::default());
    for (token, status, code) in [
        ("expired", StatusCode::UNAUTHORIZED, "invalid_session"),
        ("forbidden", StatusCode::FORBIDDEN, "permission_denied"),
        ("hidden", StatusCode::NOT_FOUND, "case_not_found"),
        ("missing-user", StatusCode::NOT_FOUND, "user_not_found"),
        (
            "database-failed",
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
        ),
    ] {
        let (actual, body) = request(
            &workflow,
            "GET",
            &format!("/api/v1/cases/{CASE}"),
            Some(&format!("Bearer {token}")),
            None,
        )
        .await;
        assert_eq!(actual, status);
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap()["error"]["code"],
            code
        );
        assert!(!body.contains("secret DSN"));
    }
}

#[tokio::test]
async fn invalid_metadata_is_an_unprocessable_entity() {
    let workflow = Arc::new(Workflow::default());
    let (status, body) = request(
        &workflow,
        "POST",
        "/api/v1/cases",
        Some("Bearer member"),
        Some(json!({"title":"", "reference":"REF-123"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["error"]["code"],
        "invalid_case_metadata"
    );
}
