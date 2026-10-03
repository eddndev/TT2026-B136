// Keep imported workflow fixtures scoped for the existing binary acceptance too.
#[allow(dead_code)]
#[path = "../../application/tests/case_support/mod.rs"]
mod case_support;
mod owner_certificate_composition_support;
#[allow(dead_code)]
mod owner_certificate_http_support;
#[allow(dead_code, clippy::duplicate_mod)]
mod password_reset_composition_support;
#[allow(dead_code, unused_imports)]
mod password_reset_http_support;
#[allow(dead_code)]
#[path = "../../application/tests/owner_certificate_cases/submission_support.rs"]
mod submission_support;
#[allow(dead_code)]
#[path = "../../application/tests/owner_certificate_cases/support.rs"]
mod support;

use axum::{
    body::{Body, Bytes},
    http::StatusCode,
};
use domain::identity::Role;
use owner_certificate_composition_support::blocked;
use owner_certificate_http_support::{path, raw, request};
use password_reset_composition_support::{get, Harness};
use std::{num::NonZeroUsize, sync::Arc, time::Duration};
use web::HttpWorkBudget;

const DASHBOARD: &str = "/api/v1/dashboard";
const NO_PROGRESS: Duration = Duration::from_millis(50);

fn budget() -> HttpWorkBudget {
    HttpWorkBudget::new(NonZeroUsize::new(1).unwrap())
}

#[tokio::test]
async fn owner_work_uses_external_budget_and_keeps_it_for_existing_routes_after_http_cancel() {
    let budget = budget();
    let held = budget.acquire_owned().await.unwrap();
    let (service, observed, mut owner_gate) = blocked();
    let fixture = Harness::with_owner_certificate(2, budget.clone(), service).unwrap();
    let router = fixture.router.clone();
    let owner = tokio::spawn(async move { request(&router, "GET", "", None).await });
    assert!(
        tokio::time::timeout(NO_PROGRESS, owner_gate.entered())
            .await
            .is_err(),
        "Owner evidence work escaped the external shared permit"
    );
    assert!(observed.lock().unwrap().calls.is_empty());
    raw(&fixture.router, "GET", &path(""), Vec::new(), &[], &[])
        .await
        .error(StatusCode::UNAUTHORIZED, "invalid_session");
    drop(held);
    owner_gate.entered().await;
    owner.abort();
    assert!(matches!(owner.await, Err(error) if error.is_cancelled()));
    let mut prior_gate = fixture.dashboard.block();
    let router = fixture.router.clone();
    let prior = tokio::spawn(async move { get(&router, DASHBOARD).await });
    assert!(
        tokio::time::timeout(NO_PROGRESS, prior_gate.entered())
            .await
            .is_err(),
        "existing route escaped the cancelled Owner worker's permit"
    );
    assert_eq!(fixture.dashboard.calls(), 0);
    assert!(
        tokio::time::timeout(NO_PROGRESS, budget.acquire_owned())
            .await
            .is_err(),
        "external work escaped the cancelled Owner worker's permit"
    );
    drop(owner_gate);
    prior_gate.entered().await;
    drop(prior_gate);
    prior
        .await
        .unwrap()
        .error(StatusCode::FORBIDDEN, "permission_denied");
    let released = tokio::time::timeout(Duration::from_secs(2), budget.acquire_owned())
        .await
        .unwrap()
        .unwrap();
    drop(released);
    assert_eq!(submission_support::count(&observed, "find"), 1);
    assert_eq!(submission_support::count(&observed, "authenticate"), 2);
}

#[tokio::test]
async fn owner_routes_share_global_http_admission_and_existing_cancelled_worker_budget() {
    let budget = budget();
    let (service, observed, mut owner_gate) = blocked();
    let fixture = Harness::with_owner_certificate(1, budget, service).unwrap();
    let mut prior_gate = fixture.dashboard.block();
    let router = fixture.router.clone();
    let prior = tokio::spawn(async move { get(&router, DASHBOARD).await });
    prior_gate.entered().await;
    let unread =
        Body::from_stream(futures_util::stream::pending::<Result<Bytes, std::io::Error>>());
    password_reset_http_support::raw(
        &fixture.router,
        &path("/prepare"),
        unread,
        &["application/json"],
    )
    .await
    .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    assert!(observed.lock().unwrap().calls.is_empty());
    prior.abort();
    assert!(matches!(prior.await, Err(error) if error.is_cancelled()));
    let router = fixture.router.clone();
    let owner = tokio::spawn(async move { request(&router, "GET", "", None).await });
    assert!(
        tokio::time::timeout(NO_PROGRESS, owner_gate.entered())
            .await
            .is_err(),
        "Owner route escaped the existing cancelled worker's permit"
    );
    get(&fixture.router, DASHBOARD)
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    assert!(observed.lock().unwrap().calls.is_empty());
    drop(prior_gate);
    owner_gate.entered().await;
    drop(owner_gate);
    owner
        .await
        .unwrap()
        .error(StatusCode::NOT_FOUND, "owner_certificate_not_found");
    assert_eq!(fixture.dashboard.calls(), 1);
    assert_eq!(submission_support::count(&observed, "find"), 1);
}

#[tokio::test]
async fn current_owner_authority_is_checked_inside_the_shared_work_budget() {
    for role in [None, Some(Role::Paralegal)] {
        let budget = budget();
        let held = budget.acquire_owned().await.unwrap();
        let harness = support::Harness::new();
        let observed = harness.state.clone();
        observed.lock().unwrap().principal = role.map(|role| application::identity::Principal {
            role,
            ..support::principal()
        });
        let fixture =
            Harness::with_owner_certificate(1, budget.clone(), Arc::new(harness.service()))
                .unwrap();
        let router = fixture.router.clone();
        let mut owner = tokio::spawn(async move { request(&router, "GET", "", None).await });
        assert!(tokio::time::timeout(NO_PROGRESS, &mut owner).await.is_err());
        assert!(observed.lock().unwrap().calls.is_empty());
        drop(held);
        let response = owner.await.unwrap();
        if role.is_none() {
            response.error(StatusCode::UNAUTHORIZED, "invalid_session");
        } else {
            response.error(StatusCode::FORBIDDEN, "permission_denied");
        }
        assert_eq!(observed.lock().unwrap().calls, vec!["authenticate"]);
        let released = tokio::time::timeout(Duration::from_secs(2), budget.acquire_owned())
            .await
            .unwrap()
            .unwrap();
        drop(released);
    }
}
