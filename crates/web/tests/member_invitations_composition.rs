#[allow(dead_code, unused_imports)]
mod member_invitations_support;
#[allow(dead_code, clippy::duplicate_mod)]
mod password_reset_composition_support;
#[allow(dead_code, unused_imports)]
mod password_reset_http_support;

use axum::{
    body::{Body, Bytes},
    http::StatusCode,
};
use member_invitations_support::{call, raw, recipient, Call, Workflow, OWNER, PUBLIC};
use password_reset_composition_support::{get, with_invitations};
use std::{future::Future, num::NonZeroUsize, sync::Arc, task::Poll};
use web::HttpWorkBudget;

const DASHBOARD: &str = "/api/v1/dashboard";

#[tokio::test]
async fn integrated_invitation_routes_share_existing_request_admission_and_cancelled_work() {
    let workflow = Arc::new(Workflow::default());
    let budget = HttpWorkBudget::new(NonZeroUsize::new(1).unwrap());
    let (router, dashboard) = with_invitations(1, budget, workflow.clone()).unwrap();
    let mut gate = dashboard.block();
    let routes = router.clone();
    let prior = tokio::spawn(async move { get(&routes, DASHBOARD).await });
    gate.entered().await;
    let path = format!("{PUBLIC}/prepare");
    let unread =
        Body::from_stream(futures_util::stream::pending::<Result<Bytes, std::io::Error>>());
    raw(
        &router,
        "POST",
        &path,
        unread,
        &[("content-type", "application/json")],
    )
    .await
    .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    call(&router, "GET", OWNER, Some("owner"), None)
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    assert_eq!(
        call(&router, "GET", "/healthz", None, None).await.status,
        StatusCode::OK
    );
    assert!(workflow.calls().is_empty());
    prior.abort();
    assert!(matches!(prior.await, Err(error) if error.is_cancelled()));
    let mut invitation = Box::pin(call(&router, "POST", &path, None, Some(recipient())));
    assert!(
        std::future::poll_fn(|cx| Poll::Ready(invitation.as_mut().poll(cx).is_pending())).await
    );
    assert!(workflow.calls().is_empty());
    get(&router, DASHBOARD)
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    drop(gate);
    let reply = invitation.await;
    reply.private();
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(workflow.calls(), vec![Call::Recipient("prepare")]);
    assert_eq!(dashboard.calls(), 1);
}

#[tokio::test]
async fn cancelled_invitation_holds_the_existing_api_worker_without_double_admission() {
    let workflow = Arc::new(Workflow::default());
    let budget = HttpWorkBudget::new(NonZeroUsize::new(1).unwrap());
    let (router, dashboard) = with_invitations(1, budget, workflow.clone()).unwrap();
    let mut gate = workflow.block();
    let routes = router.clone();
    let invitation = tokio::spawn(async move {
        call(
            &routes,
            "POST",
            &format!("{PUBLIC}/prepare"),
            None,
            Some(recipient()),
        )
        .await
    });
    gate.entered().await;
    get(&router, DASHBOARD)
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    assert_eq!(dashboard.calls(), 0);
    invitation.abort();
    assert!(matches!(invitation.await, Err(error) if error.is_cancelled()));
    let mut prior = Box::pin(get(&router, DASHBOARD));
    assert!(std::future::poll_fn(|cx| Poll::Ready(prior.as_mut().poll(cx).is_pending())).await);
    assert_eq!(dashboard.calls(), 0);
    drop(gate);
    prior
        .await
        .error(StatusCode::FORBIDDEN, "permission_denied");
    assert_eq!(dashboard.calls(), 1);
    assert_eq!(workflow.calls(), vec![Call::Recipient("prepare")]);
}
