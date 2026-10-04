use super::{
    composition::{composed, get},
    support::*,
};
use crate::{
    password_reset_composition_support::Harness as Legacy, password_reset_http_support as reset,
};
use application::ApplicationError;
use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode},
};
use std::{num::NonZeroUsize, sync::Arc, time::Duration};
use web::HttpWorkBudget;

const DASHBOARD: &str = "/api/v1/dashboard";
const NO_PROGRESS: Duration = Duration::from_millis(50);
fn budget() -> HttpWorkBudget {
    HttpWorkBudget::new(NonZeroUsize::new(1).unwrap())
}

#[tokio::test]
async fn default_and_legacy_builders_leave_certificate_login_unavailable() {
    let routers = [
        Legacy::new(2, true).router,
        Legacy::new(2, false).router,
        Legacy::with_budget(2, 1, budget()).unwrap().router,
        composed(2, 1, budget(), None).unwrap().router,
    ];
    for router in routers {
        for (path, body) in [(START, start_body()), (PROOF, proof_body())] {
            let reply = post(&router, path, body).await;
            assert_eq!(reply.status, StatusCode::NOT_FOUND);
            public(&reply);
        }
    }
}

#[tokio::test]
async fn enabled_certificate_routes_keep_password_mfa_and_reset_contracts_available() {
    let workflow = Arc::new(Workflow::default());
    let fixture = composed(2, 1, budget(), Some(workflow.clone())).unwrap();
    let password = post(
        &fixture.router,
        "/api/v1/auth/login",
        serde_json::json!({"email":"owner@example.test","password":"synthetic password"})
            .to_string(),
    )
    .await;
    assert_eq!(password.status, StatusCode::UNAUTHORIZED);
    assert_eq!(password.json()["error"]["code"], "invalid_credentials");
    public(&password);
    for path in ["/api/v1/auth/mfa/totp", "/api/v1/auth/mfa/recovery"] {
        let reply = post(
            &fixture.router,
            path,
            serde_json::json!({"challenge_token":mfa_token(),"code":"123456"}).to_string(),
        )
        .await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert_eq!(reply.json()["error"]["code"], "mfa_rejected");
        public(&reply);
    }
    let request = reset::post(
        &fixture.router,
        reset::REQUEST,
        reset::email("private@example.test"),
    )
    .await;
    assert_eq!(request.status, StatusCode::ACCEPTED);
    request.public();
    assert_eq!(*fixture.requests.calls.lock().unwrap(), 1);
    let complete = reset::post(
        &fixture.router,
        reset::COMPLETE,
        reset::completion(&reset::token(), reset::PASSWORD),
    )
    .await;
    assert_eq!(complete.status, StatusCode::NO_CONTENT);
    assert_eq!(fixture.ports.count("complete"), 1);
    complete.public();
    assert!(workflow.calls.lock().unwrap().is_empty());
    let reply = post(&fixture.router, START, start_body()).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(workflow.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn new_builder_rejects_external_work_budget_capacity_mismatch() {
    assert!(matches!(
        composed(2, 2, budget(), Some(Arc::new(Workflow::default()))),
        Err(ApplicationError::InvalidInput(_))
    ));
}

#[tokio::test]
async fn cancelled_proof_keeps_external_shared_worker_until_blocking_work_returns() {
    let budget = budget();
    let held = budget.acquire_owned().await.unwrap();
    let workflow = Arc::new(Workflow::default());
    let mut proof_gate = workflow.block_proof();
    let fixture = composed(2, 1, budget.clone(), Some(workflow.clone())).unwrap();
    let router = fixture.router.clone();
    let proof = tokio::spawn(async move { post(&router, PROOF, proof_body()).await });
    assert!(tokio::time::timeout(NO_PROGRESS, proof_gate.entered())
        .await
        .is_err());
    assert!(workflow.calls.lock().unwrap().is_empty());
    drop(held);
    proof_gate.entered().await;
    proof.abort();
    assert!(matches!(proof.await, Err(error) if error.is_cancelled()));
    let mut business_gate = fixture.dashboard.block();
    let router = fixture.router.clone();
    let business = tokio::spawn(async move { get(&router, DASHBOARD).await });
    assert!(
        tokio::time::timeout(NO_PROGRESS, business_gate.entered())
            .await
            .is_err(),
        "business route escaped the cancelled proof's active worker"
    );
    assert_eq!(fixture.dashboard.calls(), 0);
    assert!(
        tokio::time::timeout(NO_PROGRESS, budget.acquire_owned())
            .await
            .is_err(),
        "external work escaped the cancelled proof's active worker"
    );
    drop(proof_gate);
    business_gate.entered().await;
    drop(business_gate);
    business
        .await
        .unwrap()
        .error(StatusCode::FORBIDDEN, "permission_denied");
    let released = tokio::time::timeout(Duration::from_secs(2), budget.acquire_owned())
        .await
        .unwrap()
        .unwrap();
    drop(released);
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::Proof(token(), signature())]
    );
}

#[tokio::test]
async fn certificate_proof_shares_request_admission_before_reading_body_and_existing_workers() {
    let workflow = Arc::new(Workflow::default());
    let mut proof_gate = workflow.block_proof();
    let fixture = composed(1, 1, budget(), Some(workflow.clone())).unwrap();
    let mut business_gate = fixture.dashboard.block();
    let router = fixture.router.clone();
    let business = tokio::spawn(async move { get(&router, DASHBOARD).await });
    business_gate.entered().await;
    let unread =
        Body::from_stream(futures_util::stream::pending::<Result<Bytes, std::io::Error>>());
    let reply = send(
        &fixture.router,
        Request::post(PROOF)
            .header("content-type", "application/json")
            .body(unread)
            .unwrap(),
    )
    .await;
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply.json()["error"]["code"], "server_busy");
    public(&reply);
    assert!(workflow.calls.lock().unwrap().is_empty());
    business.abort();
    assert!(matches!(business.await, Err(error) if error.is_cancelled()));
    let (body_read, admitted) = tokio::sync::oneshot::channel();
    let body = Body::from_stream(futures_util::stream::once(async move {
        let _ = body_read.send(());
        Ok::<Bytes, std::io::Error>(Bytes::from(proof_body()))
    }));
    let router = fixture.router.clone();
    let proof = tokio::spawn(async move {
        send(
            &router,
            Request::post(PROOF)
                .header("content-type", "application/json")
                .body(body)
                .unwrap(),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(2), admitted)
        .await
        .unwrap()
        .unwrap();
    get(&fixture.router, DASHBOARD)
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    assert!(
        tokio::time::timeout(NO_PROGRESS, proof_gate.entered())
            .await
            .is_err(),
        "proof escaped an existing cancelled business worker"
    );
    assert!(workflow.calls.lock().unwrap().is_empty());
    drop(business_gate);
    proof_gate.entered().await;
    drop(proof_gate);
    let reply = proof.await.unwrap();
    assert_eq!(reply.status, StatusCode::OK);
    public(&reply);
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::Proof(token(), signature())]
    );
    assert_eq!(fixture.dashboard.calls(), 1);
}
