use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::post,
};
use std::sync::mpsc;
use tower::ServiceExt;

fn runtime() -> HttpRuntime {
    HttpRuntime::new(HttpLimits {
        max_requests: NonZeroUsize::new(1).unwrap(),
        max_blocking_operations: NonZeroUsize::new(1).unwrap(),
    })
}

#[tokio::test]
async fn excess_blocking_work_is_rejected_without_starting_it() {
    let runtime = runtime();
    let worker = runtime.clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = mpsc::channel();
    let first = tokio::spawn(async move {
        worker
            .run(move || {
                started.send(()).unwrap();
                wait.recv().unwrap();
                Ok(7)
            })
            .await
    });
    ready.await.unwrap();
    let rejected = runtime
        .run(|| -> Result<(), ApplicationError> { panic!("excess work started") })
        .await;
    release.send(()).unwrap();
    assert_eq!(
        rejected.err().unwrap().into_response().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(first.await.unwrap().unwrap(), 7);
    assert_eq!(runtime.run(|| Ok(9)).await.unwrap(), 9);
}

#[tokio::test]
async fn cancelling_http_future_does_not_release_running_blocking_work() {
    let runtime = runtime();
    let worker = runtime.clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = mpsc::channel();
    let first = tokio::spawn(async move {
        worker
            .run(move || {
                started.send(()).unwrap();
                wait.recv().unwrap();
                Ok(())
            })
            .await
    });
    ready.await.unwrap();
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    let rejected = runtime.run(|| Ok(())).await;
    release.send(()).unwrap();
    assert_eq!(
        rejected.err().unwrap().into_response().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let permit = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        runtime.blocking_slots.acquire(),
    )
    .await
    .unwrap()
    .unwrap();
    drop(permit);
    runtime.run(|| Ok(())).await.unwrap();
}

#[tokio::test]
async fn panic_and_port_failure_release_the_slot_without_exposing_details() {
    let runtime = runtime();
    let error = runtime
        .run(|| -> Result<(), ApplicationError> { panic!("sensitive worker details") })
        .await
        .err()
        .unwrap();
    assert_eq!(
        error.into_response().status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(runtime
        .run(|| -> Result<(), ApplicationError> { Err(ApplicationError::Port("secret".into())) })
        .await
        .is_err());
    runtime.run(|| Ok(())).await.unwrap();
}

#[tokio::test]
async fn admission_rejects_before_reading_an_extra_request_body_and_recovers() {
    let runtime = runtime();
    let admission = runtime.request_slots.clone().acquire_owned().await.unwrap();
    let router = protect(
        axum::Router::new().route("/", post(|_: axum::body::Bytes| async { "ok" })),
        runtime,
    );
    let body = Body::from_stream(futures_util::stream::pending::<
        Result<axum::body::Bytes, std::io::Error>,
    >());
    let rejected = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        router
            .clone()
            .oneshot(Request::post("/").body(body).unwrap()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(rejected.status(), StatusCode::SERVICE_UNAVAILABLE);
    drop(admission);
    let accepted = router
        .oneshot(Request::post("/").body(Body::from("ok")).unwrap())
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);
    assert_eq!(accepted.headers()["cache-control"], "no-store");
}

#[tokio::test]
async fn middleware_holds_admission_until_the_request_finishes_or_is_cancelled() {
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let entered = started.clone();
    let finish = release.clone();
    let slow = move || {
        let entered = entered.clone();
        let finish = finish.clone();
        async move {
            entered.notify_one();
            finish.notified().await;
            "ok"
        }
    };
    let router = protect(
        axum::Router::new()
            .route("/slow", post(slow))
            .route("/", post(|| async { "ok" })),
        runtime(),
    );
    let first_router = router.clone();
    let first = tokio::spawn(async move {
        first_router
            .oneshot(Request::post("/slow").body(Body::empty()).unwrap())
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    let rejected = router
        .clone()
        .oneshot(Request::post("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::SERVICE_UNAVAILABLE);
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    let accepted = router
        .oneshot(Request::post("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);
}
