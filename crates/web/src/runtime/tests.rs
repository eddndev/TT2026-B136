use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::post,
};
use std::{future::Future, sync::mpsc, task::Poll};
use tower::ServiceExt;

fn runtime() -> HttpRuntime {
    HttpRuntime::new(HttpLimits {
        max_requests: NonZeroUsize::new(1).unwrap(),
        max_blocking_operations: NonZeroUsize::new(1).unwrap(),
    })
}

#[tokio::test]
async fn admitted_blocking_work_waits_without_starting_an_extra_worker() {
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
    let mut second = Box::pin(runtime.run(|| Ok(9)));
    let queued =
        std::future::poll_fn(|cx| Poll::Ready(second.as_mut().poll(cx).is_pending())).await;
    release.send(()).unwrap();
    assert_eq!(first.await.unwrap().unwrap(), 7);
    assert!(
        queued,
        "admitted work must wait for the occupied worker slot"
    );
    assert_eq!(second.await.unwrap(), 9);
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
    let mut second = Box::pin(runtime.run(|| Ok(())));
    let queued =
        std::future::poll_fn(|cx| Poll::Ready(second.as_mut().poll(cx).is_pending())).await;
    release.send(()).unwrap();
    assert!(
        queued,
        "cancelling HTTP must not free the running worker slot"
    );
    second.await.unwrap();
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

#[tokio::test]
async fn admitted_http_requests_wait_for_workers_and_ninth_is_rejected() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };
    let runtime = HttpRuntime::new(HttpLimits::default());
    let entered = Arc::new(AtomicUsize::new(0));
    let (started, mut ready) = tokio::sync::mpsc::unbounded_channel();
    let (release, wait) = mpsc::channel();
    let wait = Arc::new(Mutex::new(wait));
    let worker = runtime.clone();
    let count = entered.clone();
    let handler = move || {
        let worker = worker.clone();
        let count = count.clone();
        let started = started.clone();
        let wait = wait.clone();
        async move {
            worker
                .run(move || {
                    let index = count.fetch_add(1, Ordering::SeqCst);
                    if index < 2 {
                        started.send(()).unwrap();
                        wait.lock().unwrap().recv().unwrap();
                    }
                    Ok("ready")
                })
                .await
        }
    };
    let router = protect(
        axum::Router::new().route("/", post(handler)),
        runtime.clone(),
    );
    let request = || Request::post("/").body(Body::empty()).unwrap();
    let first = tokio::spawn(router.clone().oneshot(request()));
    let second = tokio::spawn(router.clone().oneshot(request()));
    ready.recv().await.unwrap();
    ready.recv().await.unwrap();
    let mut waiting = Vec::new();
    let mut all_queued = true;
    for _ in 0..6 {
        let mut future = Box::pin(router.clone().oneshot(request()));
        let queued =
            std::future::poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx).is_pending())).await;
        all_queued &= queued;
        waiting.push(future);
    }
    let available = runtime.request_slots.available_permits();
    let running = entered.load(Ordering::SeqCst);
    let ninth = router.oneshot(request()).await.unwrap();
    release.send(()).unwrap();
    release.send(()).unwrap();
    assert_eq!(first.await.unwrap().unwrap().status(), StatusCode::OK);
    assert_eq!(second.await.unwrap().unwrap().status(), StatusCode::OK);
    assert!(
        all_queued,
        "six admitted requests must wait behind the two active workers"
    );
    assert_eq!(
        available, 0,
        "queued requests retain the original eight-slot request budget"
    );
    assert_eq!(running, 2, "waiting requests must not launch extra workers");
    assert_eq!(ninth.status(), StatusCode::SERVICE_UNAVAILABLE);
    for future in waiting {
        assert_eq!(future.await.unwrap().status(), StatusCode::OK);
    }
    assert_eq!(entered.load(Ordering::SeqCst), 8);
    assert_eq!(runtime.request_slots.available_permits(), 8);
    assert_eq!(runtime.blocking_slots.available_permits(), 2);
}

#[tokio::test]
async fn cancelling_queued_work_does_not_execute_it_or_consume_a_worker() {
    let runtime = runtime();
    let held = runtime
        .blocking_slots
        .clone()
        .acquire_owned()
        .await
        .unwrap();
    let mut queued = Box::pin(runtime.run(|| -> Result<(), ApplicationError> {
        panic!("cancelled queued work must not execute")
    }));
    let waiting =
        std::future::poll_fn(|cx| Poll::Ready(queued.as_mut().poll(cx).is_pending())).await;
    drop(queued);
    drop(held);
    assert!(
        waiting,
        "worker saturation must retain admitted work in the bounded queue"
    );
    assert_eq!(runtime.run(|| Ok(3)).await.unwrap(), 3);
}
