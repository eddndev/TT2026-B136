// Each workflow fixture retains its independently scoped dependency modules.
#[allow(clippy::duplicate_mod)]
mod password_reset_composition_support;
#[allow(dead_code, unused_imports)]
mod password_reset_http_support;

use axum::{
    body::{Body, Bytes},
    http::StatusCode,
};
use password_reset_composition_support::{get, Harness};
use password_reset_http_support::{
    completion, email, post, raw, token, COMPLETE, PASSWORD, REQUEST,
};
use std::{num::NonZeroUsize, time::Duration};
use web::HttpWorkBudget;

const DASHBOARD: &str = "/api/v1/dashboard";
const NO_PROGRESS: Duration = Duration::from_millis(50);

#[tokio::test]
async fn cancelled_existing_route_retains_the_worker_used_by_reset_completion() {
    let fixture = Harness::new(2, false);
    let mut prior_gate = fixture.dashboard.block();
    let router = fixture.router.clone();
    let prior = tokio::spawn(async move { get(&router, DASHBOARD).await });
    prior_gate.entered().await;
    assert_eq!(fixture.dashboard.calls(), 1);

    let admitted = post(&fixture.router, REQUEST, email("invalid address")).await;
    admitted.public();
    assert_eq!(admitted.status, StatusCode::ACCEPTED);
    assert!(fixture.ports.calls().is_empty());
    prior.abort();
    assert!(matches!(prior.await, Err(error) if error.is_cancelled()));

    let mut reset_gate = fixture.ports.block_inspect();
    let router = fixture.router.clone();
    let completion =
        tokio::spawn(async move { post(&router, COMPLETE, completion(&token(), PASSWORD)).await });
    assert!(
        tokio::time::timeout(NO_PROGRESS, reset_gate.entered())
            .await
            .is_err(),
        "reset completion escaped the existing route's retained worker permit"
    );
    assert!(fixture.ports.calls().is_empty());
    let admitted = post(&fixture.router, REQUEST, email("private@example.test")).await;
    admitted.public();
    assert_eq!(admitted.status, StatusCode::ACCEPTED);
    assert_eq!(*fixture.requests.calls.lock().unwrap(), 2);
    assert_eq!(fixture.requests.retained.lock().unwrap().len(), 2);

    drop(prior_gate);
    reset_gate.entered().await;
    drop(reset_gate);
    let reply = completion.await.unwrap();
    reply.public();
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(fixture.ports.count("inspect"), 1);
    assert_eq!(fixture.ports.count("complete"), 1);
}

#[tokio::test]
async fn cancelled_reset_retains_the_existing_worker_and_all_routes_share_http_admission() {
    let fixture = Harness::new(1, false);
    let mut reset_gate = fixture.ports.block_inspect();
    let router = fixture.router.clone();
    let reset =
        tokio::spawn(async move { post(&router, COMPLETE, completion(&token(), PASSWORD)).await });
    reset_gate.entered().await;
    get(&fixture.router, DASHBOARD)
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    let unread =
        Body::from_stream(futures_util::stream::pending::<Result<Bytes, std::io::Error>>());
    raw(&fixture.router, REQUEST, unread, &["application/json"])
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    assert_eq!(fixture.dashboard.calls(), 0);
    assert_eq!(*fixture.requests.calls.lock().unwrap(), 0);
    reset.abort();
    assert!(matches!(reset.await, Err(error) if error.is_cancelled()));

    let mut prior_gate = fixture.dashboard.block();
    let router = fixture.router.clone();
    let prior = tokio::spawn(async move { get(&router, DASHBOARD).await });
    assert!(
        tokio::time::timeout(NO_PROGRESS, prior_gate.entered())
            .await
            .is_err(),
        "an existing route escaped the cancelled completion's retained worker permit"
    );
    assert_eq!(fixture.dashboard.calls(), 0);
    post(&fixture.router, REQUEST, email("private@example.test"))
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    assert_eq!(*fixture.requests.calls.lock().unwrap(), 0);
    assert_eq!(fixture.ports.count("complete"), 0);

    drop(reset_gate);
    prior_gate.entered().await;
    assert_eq!(fixture.ports.count("complete"), 1);
    drop(prior_gate);
    let reply = prior.await.unwrap();
    reply.public();
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let admitted = post(&fixture.router, REQUEST, email("private@example.test")).await;
    admitted.public();
    assert_eq!(admitted.status, StatusCode::ACCEPTED);
    assert_eq!(*fixture.requests.calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn existing_api_constructor_disables_reset_without_changing_existing_routes() {
    let fixture = Harness::new(1, true);
    for (path, body) in [
        (REQUEST, email("private@example.test")),
        (COMPLETE, completion(&token(), PASSWORD)),
    ] {
        post(&fixture.router, path, body).await.error(
            StatusCode::SERVICE_UNAVAILABLE,
            "password_reset_unavailable",
        );
    }
    assert!(fixture.ports.calls().is_empty());
    assert_eq!(*fixture.requests.calls.lock().unwrap(), 0);
    let prior = get(&fixture.router, DASHBOARD).await;
    prior.public();
    assert_eq!(prior.status, StatusCode::FORBIDDEN);
    assert_eq!(fixture.dashboard.calls(), 1);
    let health = get(&fixture.router, "/healthz").await;
    assert_eq!(health.status, StatusCode::OK);
    assert_eq!(health.body, b"ok");
}

#[tokio::test]
async fn external_budget_and_both_http_workflows_hold_the_same_permit_after_cancellation() {
    let budget = HttpWorkBudget::new(NonZeroUsize::new(1).unwrap());
    assert_eq!(budget.capacity(), 1);
    let held = budget.acquire_owned().await.unwrap();
    let fixture = Harness::with_budget(2, 1, budget.clone()).unwrap();
    let mut reset_gate = fixture.ports.block_inspect();
    let router = fixture.router.clone();
    let reset =
        tokio::spawn(async move { post(&router, COMPLETE, completion(&token(), PASSWORD)).await });
    assert!(
        tokio::time::timeout(NO_PROGRESS, reset_gate.entered())
            .await
            .is_err(),
        "reset completion did not use the injected budget"
    );
    assert!(fixture.ports.calls().is_empty());
    let admitted = post(&fixture.router, REQUEST, email("private@example.test")).await;
    admitted.public();
    assert_eq!(admitted.status, StatusCode::ACCEPTED);
    drop(held);
    reset_gate.entered().await;
    reset.abort();
    assert!(matches!(reset.await, Err(error) if error.is_cancelled()));
    assert!(
        tokio::time::timeout(NO_PROGRESS, budget.acquire_owned())
            .await
            .is_err(),
        "external work escaped the cancelled reset worker's permit"
    );
    drop(reset_gate);
    let held = tokio::time::timeout(Duration::from_secs(2), budget.acquire_owned())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fixture.ports.count("complete"), 1);

    let mut prior_gate = fixture.dashboard.block();
    let router = fixture.router.clone();
    let prior = tokio::spawn(async move { get(&router, DASHBOARD).await });
    assert!(
        tokio::time::timeout(NO_PROGRESS, prior_gate.entered())
            .await
            .is_err(),
        "existing HTTP did not use the injected budget"
    );
    assert_eq!(fixture.dashboard.calls(), 0);
    let admitted = post(&fixture.router, REQUEST, email("invalid address")).await;
    admitted.public();
    assert_eq!(admitted.status, StatusCode::ACCEPTED);
    assert_eq!(*fixture.requests.calls.lock().unwrap(), 2);
    drop(held);
    prior_gate.entered().await;
    prior.abort();
    assert!(matches!(prior.await, Err(error) if error.is_cancelled()));
    assert!(
        tokio::time::timeout(NO_PROGRESS, budget.acquire_owned())
            .await
            .is_err(),
        "external work escaped the cancelled existing worker's permit"
    );
    drop(prior_gate);
    let released = tokio::time::timeout(Duration::from_secs(2), budget.acquire_owned())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(budget.capacity(), 1);
    drop(released);
}

#[test]
fn external_budget_capacity_must_match_the_declared_blocking_limit() {
    for (declared, actual) in [(1, 2), (2, 1)] {
        let budget = HttpWorkBudget::new(NonZeroUsize::new(actual).unwrap());
        assert!(
            matches!(
                Harness::with_budget(2, declared, budget),
                Err(application::ApplicationError::InvalidInput(_))
            ),
            "router accepted a budget with a different blocking capacity"
        );
    }
}

#[tokio::test]
async fn resource_hearing_routes_are_composed_with_shared_request_admission() {
    let fixture = Harness::new(1, false);
    let base = "/api/v1/cases/00000000-0000-4000-8000-000000000001/procedural-resources/00000000-0000-4000-8000-000000000002/activities/resource-hearings";
    for suffix in [
        "",
        "/00000000-0000-4000-8000-000000000003",
        "/00000000-0000-4000-8000-000000000003/revisions/1",
    ] {
        get(&fixture.router, &format!("{base}{suffix}"))
            .await
            .error(StatusCode::FORBIDDEN, "permission_denied");
    }
    let mut gate = fixture.dashboard.block();
    let router = fixture.router.clone();
    let blocked = tokio::spawn(async move { get(&router, DASHBOARD).await });
    gate.entered().await;
    get(&fixture.router, base)
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    drop(gate);
    blocked.await.unwrap();
}
