use application::{
    document_content::{DocumentContent, DocumentContentWorkflow},
    ApplicationError,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    response::Response,
    Router,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentVersionRef, Sha256Digest},
};
use http_body_util::BodyExt;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tower::ServiceExt;
use uuid::Uuid;

const PAYLOAD: &[u8] = b"exact content retained until its final consumer releases it";

struct Workflow {
    calls: AtomicUsize,
}

impl DocumentContentWorkflow for Workflow {
    fn content_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<DocumentContent, ApplicationError> {
        assert_eq!(token, "content-session");
        self.calls.fetch_add(1, Ordering::SeqCst);
        if case_id.as_uuid() == Uuid::from_u128(9) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(DocumentContent {
            case_id,
            reference,
            file_name: "retained.bin".into(),
            digest: Sha256Digest::from_array([0xab; 32]),
            bytes: PAYLOAD.to_vec().into(),
        })
    }
}

fn setup() -> (Router, Arc<Workflow>, usize) {
    let workflow = Arc::new(Workflow {
        calls: AtomicUsize::new(0),
    });
    let router = web::document_content_router(workflow.clone());
    (
        router,
        workflow,
        web::HttpLimits::default().max_requests.get(),
    )
}

async fn request(router: &Router, case: u128) -> Response {
    let path = format!(
        "/api/v1/cases/{}/documents/{}/versions/1/content",
        Uuid::from_u128(case),
        Uuid::from_u128(2),
    );
    router
        .clone()
        .oneshot(
            Request::get(path)
                .header("authorization", "Bearer content-session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn reserve_all(router: &Router, capacity: usize) -> Vec<Response> {
    let mut retained = Vec::new();
    for _ in 0..capacity {
        let response = request(router, 1).await;
        assert_eq!(response.status(), StatusCode::OK);
        retained.push(response);
    }
    retained
}

async fn assert_busy_without_work(router: &Router, workflow: &Workflow) {
    let before = workflow.calls.load(Ordering::SeqCst);
    let response = request(router, 1).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(workflow.calls.load(Ordering::SeqCst), before);
    assert!(!response.headers().contains_key("content-disposition"));
    assert!(!response.headers().contains_key("x-document-digest"));
}

#[tokio::test]
async fn retained_content_bodies_bound_new_work_and_release_on_drop_or_consumption() {
    let (router, workflow, capacity) = setup();
    // Errors have no retained content, even when their JSON body remains alive.
    let denial = request(&router, 9).await;
    assert_eq!(denial.status(), StatusCode::FORBIDDEN);
    let mut retained = reserve_all(&router, capacity).await;
    assert_eq!(workflow.calls.load(Ordering::SeqCst), capacity + 1);
    assert_busy_without_work(&router, &workflow).await;

    drop(retained.pop().unwrap());
    let replacement = request(&router, 1).await;
    assert_eq!(replacement.status(), StatusCode::OK);
    retained.push(replacement);
    assert_busy_without_work(&router, &workflow).await;

    let response = retained.pop().unwrap();
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        PAYLOAD
    );
    let after_consumption = request(&router, 1).await;
    assert_eq!(after_consumption.status(), StatusCode::OK);
    retained.push(after_consumption);
    assert_busy_without_work(&router, &workflow).await;
    drop(denial);
    drop(retained);
}

#[tokio::test]
async fn transferred_content_chunks_keep_capacity_after_body_drop_until_last_clone_drops() {
    let (router, workflow, capacity) = setup();
    let mut retained = reserve_all(&router, capacity).await;
    let mut body = retained.pop().unwrap().into_body();
    let bytes = body.frame().await.unwrap().unwrap().into_data().unwrap();
    assert_eq!(bytes.as_ref(), PAYLOAD);
    assert!(body.frame().await.is_none());
    drop(body);
    assert_busy_without_work(&router, &workflow).await;

    let transport_copy = bytes.clone();
    drop(bytes);
    assert_busy_without_work(&router, &workflow).await;
    drop(transport_copy);
    let replacement = request(&router, 1).await;
    assert_eq!(replacement.status(), StatusCode::OK);
    assert_eq!(workflow.calls.load(Ordering::SeqCst), capacity + 1);
    retained.push(replacement);
    assert_busy_without_work(&router, &workflow).await;
}
