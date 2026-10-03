use super::{composition::composed, support::*};
use crate::password_reset_composition_support::Harness as Legacy;
use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode},
    Router,
};
use serde_json::json;
use std::{num::NonZeroUsize, sync::Arc};
use web::HttpWorkBudget;

const AVAILABILITY: &str = "/api/v1/auth/certificate-login/availability";

fn budget() -> HttpWorkBudget {
    HttpWorkBudget::new(NonZeroUsize::new(1).unwrap())
}

async fn available(router: &Router, enabled: bool) {
    for authorization in [None, Some("Bearer unrelated-expired-session")] {
        let mut request = Request::get(AVAILABILITY);
        if let Some(value) = authorization {
            request = request.header("authorization", value);
        }
        let reply = send(router, request.body(Body::empty()).unwrap()).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.json(), json!({"enabled": enabled}));
        public(&reply);
    }
}

#[tokio::test]
async fn availability_reflects_only_injected_workflow_without_admitting_an_identity() {
    let (standalone, workflow) = standalone();
    available(&standalone, true).await;
    assert!(workflow.calls.lock().unwrap().is_empty());
    let enabled = composed(2, 1, budget(), Some(workflow.clone())).unwrap();
    available(&enabled.router, true).await;
    assert!(workflow.calls.lock().unwrap().is_empty());
    assert_eq!(enabled.dashboard.calls(), 0);
    assert_eq!(*enabled.requests.calls.lock().unwrap(), 0);
    assert_eq!(enabled.ports.count("complete"), 0);

    for router in [
        Legacy::new(2, true).router,
        Legacy::new(2, false).router,
        Legacy::with_budget(2, 1, budget()).unwrap().router,
        composed(2, 1, budget(), None).unwrap().router,
    ] {
        available(&router, false).await;
        for (path, body) in [(START, start_body()), (PROOF, proof_body())] {
            let reply = post(&router, path, body).await;
            assert_eq!(reply.status, StatusCode::NOT_FOUND);
            public(&reply);
        }
    }
}

#[tokio::test]
async fn availability_rejects_queries_and_actual_body_bytes_without_workflow_calls() {
    let (enabled, workflow) = standalone();
    let disabled = composed(2, 1, budget(), None).unwrap().router;
    for router in [enabled, disabled] {
        for suffix in ["?", "?owner_id=private-owner", "?enabled=true"] {
            let reply = send(
                &router,
                Request::get(format!("{AVAILABILITY}{suffix}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(reply.status, StatusCode::BAD_REQUEST);
            public(&reply);
        }
        for bytes in [b" ".as_slice(), b"{}".as_slice(), &[b'x'; 2049]] {
            let chunks: Vec<Result<Bytes, std::io::Error>> = bytes
                .chunks(127)
                .map(|chunk| Ok(Bytes::copy_from_slice(chunk)))
                .collect();
            let reply = send(
                &router,
                Request::get(AVAILABILITY)
                    .header("content-length", "0")
                    .body(Body::from_stream(futures_util::stream::iter(chunks)))
                    .unwrap(),
            )
            .await;
            assert_eq!(reply.status, StatusCode::BAD_REQUEST);
            public(&reply);
        }
        let failed = futures_util::stream::iter([Err::<Bytes, _>(std::io::Error::other(
            "private-availability-read-error",
        ))]);
        let reply = send(
            &router,
            Request::get(AVAILABILITY)
                .body(Body::from_stream(failed))
                .unwrap(),
        )
        .await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST);
        assert!(!String::from_utf8(reply.body.clone())
            .unwrap()
            .contains("private-availability-read-error"));
        public(&reply);
        for method in ["POST", "PUT", "PATCH", "DELETE"] {
            let reply = send(
                &router,
                Request::builder()
                    .method(method)
                    .uri(AVAILABILITY)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED);
            public(&reply);
        }
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn availability_uses_shared_request_admission_but_no_blocking_work_permit() {
    let budget = budget();
    let held = budget.acquire_owned().await.unwrap();
    let workflow = Arc::new(Workflow::default());
    let fixture = composed(1, 1, budget.clone(), Some(workflow.clone())).unwrap();
    available(&fixture.router, true).await;
    assert!(workflow.calls.lock().unwrap().is_empty());
    drop(held);
    let mut gate = workflow.block_proof();
    let router = fixture.router.clone();
    let proof = tokio::spawn(async move { post(&router, PROOF, proof_body()).await });
    gate.entered().await;
    let reply = send(
        &fixture.router,
        Request::get(AVAILABILITY).body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply.json()["error"]["code"], "server_busy");
    public(&reply);
    proof.abort();
    assert!(matches!(proof.await, Err(error) if error.is_cancelled()));
    available(&fixture.router, true).await;
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::Proof(token(), signature())]
    );
    drop(gate);
}
