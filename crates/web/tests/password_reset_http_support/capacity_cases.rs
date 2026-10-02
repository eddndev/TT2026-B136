use crate::password_reset_http_support::*;
use axum::{
    body::{Body, Bytes},
    http::StatusCode,
};
use std::{future::Future, task::Poll};

#[tokio::test]
async fn request_admission_replies_while_the_only_completion_worker_is_retained() {
    let fixture = Harness::ready();
    let mut gate = fixture.ports.block_inspect();
    let router = fixture.router.clone();
    let pending =
        tokio::spawn(async move { post(&router, COMPLETE, completion(&token(), PASSWORD)).await });
    gate.entered().await;
    let reply = post(&fixture.router, REQUEST, email("private@example.test")).await;
    reply.public();
    assert_eq!(reply.status, StatusCode::ACCEPTED);
    assert_eq!(fixture.requests.retained.lock().unwrap().len(), 1);
    assert_eq!(fixture.ports.count("complete"), 0);
    drop(gate);
    assert_eq!(pending.await.unwrap().status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn shared_http_admission_and_cancelled_completion_keep_their_original_worker_bounds() {
    let fixture = Harness::new(RequestAdmission::Accepted, true, 1, 1);
    let mut gate = fixture.ports.block_inspect();
    let router = fixture.router.clone();
    let first =
        tokio::spawn(async move { post(&router, COMPLETE, completion(&token(), PASSWORD)).await });
    gate.entered().await;
    let unread =
        Body::from_stream(futures_util::stream::pending::<Result<Bytes, std::io::Error>>());
    raw(&fixture.router, REQUEST, unread, &["application/json"])
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "server_busy");
    assert_eq!(*fixture.requests.calls.lock().unwrap(), 0);
    first.abort();
    assert!(matches!(first.await, Err(error) if error.is_cancelled()));
    let mut second = Box::pin(post(
        &fixture.router,
        COMPLETE,
        completion(&token(), PASSWORD),
    ));
    let waiting =
        std::future::poll_fn(|cx| Poll::Ready(second.as_mut().poll(cx).is_pending())).await;
    assert!(
        waiting,
        "cancelled HTTP released a still-running completion worker"
    );
    assert_eq!(fixture.ports.count("inspect"), 1);
    drop(gate);
    assert_eq!(second.await.status, StatusCode::NO_CONTENT);
    assert_eq!(fixture.ports.count("inspect"), 2);
    assert_eq!(fixture.ports.count("complete"), 2);
}
