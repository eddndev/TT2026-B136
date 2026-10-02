use super::*;
use crate::runtime::{protect, HttpLimits};
use application::{
    audit_query::{AuditEventPage, AuditEventQuery},
    ApplicationError,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::{
    future::Future,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    task::Poll,
};
use tower::ServiceExt;

struct Workflow(AtomicUsize);
impl AuditEventWorkflow for Workflow {
    fn read(
        &self,
        _token: &str,
        query: AuditEventQuery,
    ) -> Result<AuditEventPage, ApplicationError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(AuditEventPage {
            checked_at: query.until(),
            snapshot_max_sequence: None,
            events: vec![],
            has_more: false,
            next_cursor: None,
        })
    }
}
#[tokio::test]
async fn audit_uses_the_existing_shared_blocking_budget() {
    let runtime = HttpRuntime::new(HttpLimits {
        max_requests: NonZeroUsize::new(2).unwrap(),
        max_blocking_operations: NonZeroUsize::new(1).unwrap(),
    });
    let worker = runtime.clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = mpsc::channel();
    let occupied = tokio::spawn(async move {
        worker
            .run(move || {
                started.send(()).unwrap();
                wait.recv().unwrap();
                Ok(())
            })
            .await
    });
    ready.await.unwrap();
    let workflow = Arc::new(Workflow(AtomicUsize::new(0)));
    let router = protect(router(workflow.clone(), runtime.clone()), runtime);
    let request = Request::builder()
        .uri("/api/v1/audit/events?from=2026-10-01T00:00:00Z&until=2026-10-02T00:00:00Z")
        .header("Authorization", "Bearer owner-session")
        .body(Body::empty())
        .unwrap();
    let mut pending = Box::pin(router.oneshot(request));
    let queued =
        std::future::poll_fn(|cx| Poll::Ready(pending.as_mut().poll(cx).is_pending())).await;
    let calls_before = workflow.0.load(Ordering::SeqCst);
    release.send(()).unwrap();
    occupied.await.unwrap().unwrap();
    assert!(queued, "audit must wait for the shared worker permit");
    assert_eq!(calls_before, 0);
    let response = pending.await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(workflow.0.load(Ordering::SeqCst), 1);
}
