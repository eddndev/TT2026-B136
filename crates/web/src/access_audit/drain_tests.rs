use super::{with_access_audit_drain, AccessAuditDrain, AuditProxyPolicy};
use application::{
    access_audit::{AccessAuditRecord, AccessAuditWriter},
    ApplicationError,
};
use axum::{body::Body, http::Request, routing::get, Router};
use std::{
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};
use tokio::sync::Notify;
use tower::ServiceExt;

struct GatedSink {
    entered: Notify,
    release: Mutex<mpsc::Receiver<()>>,
    records: Mutex<Vec<AccessAuditRecord>>,
}

impl AccessAuditWriter for GatedSink {
    fn record(&self, record: AccessAuditRecord) -> Result<(), ApplicationError> {
        self.entered.notify_one();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| ApplicationError::Port("test audit gate timed out".into()))?;
        self.records.lock().unwrap().push(record);
        Ok(())
    }
}

#[tokio::test]
async fn drain_waits_for_cancelled_clients_handler_and_durable_append() {
    let drain = AccessAuditDrain::default();
    let (finish_append, release_append) = mpsc::channel();
    let sink = Arc::new(GatedSink {
        entered: Notify::new(),
        release: Mutex::new(release_append),
        records: Mutex::new(Vec::new()),
    });
    let handler_entered = Arc::new(Notify::new());
    let finish_handler = Arc::new(Notify::new());
    let runtime = crate::runtime::HttpRuntime::new(crate::HttpLimits::default());
    let (entered, released) = (handler_entered.clone(), finish_handler.clone());
    let router = Router::new().route(
        "/api/v1/cases",
        get(move || {
            let (entered, released) = (entered.clone(), released.clone());
            async move {
                entered.notify_one();
                released.notified().await;
                "completed"
            }
        }),
    );
    let router = with_access_audit_drain(
        crate::runtime::protect(router, runtime),
        sink.clone(),
        AuditProxyPolicy::Direct,
        drain.clone(),
    );
    let request = Request::builder()
        .uri("/api/v1/cases")
        .body(Body::empty())
        .unwrap();
    let caller = tokio::spawn(router.oneshot(request));
    tokio::time::timeout(Duration::from_secs(2), handler_entered.notified())
        .await
        .unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());

    let mut draining = tokio::spawn(async move { drain.drain().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut draining)
            .await
            .is_err()
    );
    assert!(sink.records.lock().unwrap().is_empty());

    finish_handler.notify_one();
    tokio::time::timeout(Duration::from_secs(2), sink.entered.notified())
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut draining)
            .await
            .is_err()
    );
    assert!(sink.records.lock().unwrap().is_empty());

    finish_append.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), &mut draining)
        .await
        .unwrap()
        .unwrap();
    let records = sink.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action, "access.granted");
}
