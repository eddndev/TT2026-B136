use crate::member_invitations_support::*;
use axum::{
    body::{Body, Bytes},
    http::StatusCode,
};
use std::{future::Future, task::Poll};

#[tokio::test]
async fn public_and_owner_routes_share_admission_and_cancelled_work_retains_its_budget() {
    let fixture = Harness::new(true, 1);
    let mut gate = fixture.workflow.block();
    let router = fixture.router.clone();
    let first = tokio::spawn(async move {
        call(
            &router,
            "POST",
            &format!("{PUBLIC}/prepare"),
            None,
            Some(recipient()),
        )
        .await
    });
    gate.entered().await;
    let unread =
        Body::from_stream(futures_util::stream::pending::<Result<Bytes, std::io::Error>>());
    raw(
        &fixture.router,
        "POST",
        OWNER,
        unread,
        &[
            ("content-type", "application/json"),
            ("authorization", "Bearer owner"),
        ],
    )
    .await
    .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    first.abort();
    assert!(matches!(first.await, Err(error) if error.is_cancelled()));
    let mut second = Box::pin(call(&fixture.router, "GET", OWNER, Some("owner"), None));
    assert!(std::future::poll_fn(|cx| Poll::Ready(second.as_mut().poll(cx).is_pending())).await);
    assert_eq!(fixture.workflow.calls(), vec![Call::Recipient("prepare")]);
    drop(gate);
    assert_eq!(second.await.status, StatusCode::OK);
    assert_eq!(fixture.workflow.calls().len(), 2);
}

#[tokio::test]
async fn supplied_budget_is_used_by_recipient_and_owner_operations() {
    let fixture = Harness::ready();
    let held = fixture.budget.acquire_owned().await.unwrap();
    let path = format!("{PUBLIC}/prepare");
    let mut public = Box::pin(call(
        &fixture.router,
        "POST",
        &path,
        None,
        Some(recipient()),
    ));
    let mut owner = Box::pin(call(&fixture.router, "GET", OWNER, Some("owner"), None));
    assert!(std::future::poll_fn(|cx| Poll::Ready(public.as_mut().poll(cx).is_pending())).await);
    assert!(std::future::poll_fn(|cx| Poll::Ready(owner.as_mut().poll(cx).is_pending())).await);
    assert!(fixture.workflow.calls().is_empty());
    drop(held);
    assert_eq!(public.await.status, StatusCode::OK);
    assert_eq!(owner.await.status, StatusCode::OK);
    assert_eq!(fixture.workflow.calls().len(), 2);
}
