use super::*;
use application::{
    access_audit::{AccessAuditRecord, AccessAuditWriter},
    ApplicationError,
};
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    routing::{get, post},
    Router,
};
use domain::identity::UserId;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower::ServiceExt;

#[derive(Default)]
struct Sink {
    records: Mutex<Vec<AccessAuditRecord>>,
    fail: bool,
    written: tokio::sync::Notify,
}
impl AccessAuditWriter for Sink {
    fn record(&self, record: AccessAuditRecord) -> Result<(), ApplicationError> {
        if self.fail {
            return Err(ApplicationError::Port("private failure".into()));
        }
        self.records.lock().unwrap().push(record);
        self.written.notify_one();
        Ok(())
    }
}

fn request(path: &str, peer: &str, forwarded: &str) -> Request<Body> {
    let mut request = Request::builder()
        .uri(path)
        .header("x-forwarded-for", forwarded)
        .header("authorization", "Bearer private-token")
        .body(Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    request
}

fn app(sink: Arc<Sink>, policy: AuditProxyPolicy, user: UserId, status: StatusCode) -> Router {
    let runtime = crate::runtime::HttpRuntime::new(crate::HttpLimits::default());
    let worker = runtime.clone();
    let router = Router::new()
        .route(
            "/api/v1/cases/:id",
            get(move || {
                let runtime = worker.clone();
                async move {
                    runtime
                        .run(move || application::audit_context::bind_actor(user))
                        .await
                        .unwrap();
                    status
                }
            }),
        )
        .route("/healthz", get(|| async { StatusCode::OK }));
    with_access_audit(crate::runtime::protect(router, runtime), sink, policy)
}

#[tokio::test]
async fn records_current_identity_and_direct_peer_without_request_secrets() {
    let sink = Arc::new(Sink::default());
    let user = UserId::new();
    let id = uuid::Uuid::new_v4();
    let response = app(sink.clone(), AuditProxyPolicy::Direct, user, StatusCode::OK)
        .oneshot(request(
            &format!("/api/v1/cases/{id}?token=private-query"),
            "192.0.2.1:12",
            "203.0.113.7",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let rows = sink.records.lock().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].context.user_id, Some(user));
    assert_eq!(rows[0].context.ip_address.unwrap().to_string(), "192.0.2.1");
    assert!(rows[0].context.request_id.is_some());
    assert_eq!(rows[0].action, "access.granted");
    assert!(rows[0].resource.contains(&id.to_string()));
    assert!(!rows[0].resource.contains("private"));
    assert_eq!(
        application::audit_context::snapshot().unwrap().user_id,
        None
    );
}

#[tokio::test]
async fn only_an_explicit_loopback_proxy_can_supply_one_forwarded_address() {
    let user = UserId::new();
    for (peer, header, expected) in [
        ("127.0.0.1:12", "2001:db8::4", "2001:db8::4"),
        ("192.0.2.3:12", "203.0.113.4", "192.0.2.3"),
        ("127.0.0.1:12", "203.0.113.4, 192.0.2.8", "127.0.0.1"),
        ("127.0.0.1:12", "not-an-ip", "127.0.0.1"),
    ] {
        let sink = Arc::new(Sink::default());
        app(
            sink.clone(),
            AuditProxyPolicy::LoopbackProxy,
            user,
            StatusCode::OK,
        )
        .oneshot(request("/api/v1/cases/unknown", peer, header))
        .await
        .unwrap();
        assert_eq!(
            sink.records.lock().unwrap()[0]
                .context
                .ip_address
                .unwrap()
                .to_string(),
            expected
        );
    }
}

#[tokio::test]
async fn duplicate_forwarded_headers_keep_the_observed_loopback_peer() {
    let sink = Arc::new(Sink::default());
    let mut incoming = request("/api/v1/cases/unknown", "127.0.0.1:12", "203.0.113.4");
    incoming
        .headers_mut()
        .append("x-forwarded-for", "192.0.2.8".parse().unwrap());
    let response = app(
        sink.clone(),
        AuditProxyPolicy::LoopbackProxy,
        UserId::new(),
        StatusCode::OK,
    )
    .oneshot(incoming)
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let records = sink.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].context.ip_address,
        Some("127.0.0.1".parse().unwrap())
    );
}

#[tokio::test]
async fn missing_forwarded_header_keeps_the_observed_loopback_peer() {
    let sink = Arc::new(Sink::default());
    let mut incoming = request("/api/v1/cases/unknown", "127.0.0.1:12", "203.0.113.4");
    incoming.headers_mut().remove("x-forwarded-for");
    let response = app(
        sink.clone(),
        AuditProxyPolicy::LoopbackProxy,
        UserId::new(),
        StatusCode::OK,
    )
    .oneshot(incoming)
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let records = sink.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].context.ip_address,
        Some("127.0.0.1".parse().unwrap())
    );
}

#[tokio::test]
async fn forwarded_address_without_socket_metadata_stays_unavailable() {
    for policy in [AuditProxyPolicy::Direct, AuditProxyPolicy::LoopbackProxy] {
        let sink = Arc::new(Sink::default());
        let mut incoming = request("/api/v1/cases/unknown", "127.0.0.1:12", "203.0.113.4");
        incoming
            .extensions_mut()
            .remove::<ConnectInfo<SocketAddr>>();
        let response = app(sink.clone(), policy, UserId::new(), StatusCode::OK)
            .oneshot(incoming)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let records = sink.records.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].context.ip_address, None);
        assert_eq!(records[0].context.source, AuditSource::Http);
    }
}

#[tokio::test]
async fn mfa_response_and_durable_event_share_the_server_assigned_request_identity() {
    for path in ["/api/v1/auth/mfa/totp", "/api/v1/auth/mfa/recovery"] {
        let sink = Arc::new(Sink::default());
        let runtime = crate::runtime::HttpRuntime::new(crate::HttpLimits::default());
        let router = Router::new().route(path, post(|| async { StatusCode::UNAUTHORIZED }));
        let router = with_access_audit(
            crate::runtime::protect(router, runtime),
            sink.clone(),
            AuditProxyPolicy::Direct,
        );
        let supplied = uuid::Uuid::new_v4();
        let incoming = Request::builder()
            .method("POST")
            .uri(path)
            .header("x-request-id", supplied.to_string())
            .body(Body::empty())
            .unwrap();
        let response = router.oneshot(incoming).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let returned = uuid::Uuid::parse_str(
            response
                .headers()
                .get("x-request-id")
                .unwrap()
                .to_str()
                .unwrap(),
        )
        .unwrap();
        assert_ne!(returned, supplied);
        let records = sink.records.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].context.request_id, Some(returned));
        assert_eq!(records[0].context.user_id, None);
        assert_eq!(records[0].action, "access.denied");
    }
}

#[tokio::test]
async fn rejection_preserves_ambiguous_not_found_and_operational_failure() {
    for (status, action) in [
        (403, "access.denied"),
        (404, "access.rejected"),
        (409, "access.rejected"),
        (500, "access.failed"),
    ] {
        let sink = Arc::new(Sink::default());
        app(
            sink.clone(),
            AuditProxyPolicy::Direct,
            UserId::new(),
            StatusCode::from_u16(status).unwrap(),
        )
        .oneshot(request("/api/v1/cases/not-a-real-id", "192.0.2.1:12", ""))
        .await
        .unwrap();
        assert_eq!(sink.records.lock().unwrap()[0].action, action);
        assert!(!sink.records.lock().unwrap()[0]
            .resource
            .contains("not-a-real-id"));
    }
}

#[tokio::test]
async fn audit_failure_does_not_release_a_success_response() {
    let sink = Arc::new(Sink {
        fail: true,
        ..Sink::default()
    });
    let response = app(
        sink,
        AuditProxyPolicy::Direct,
        UserId::new(),
        StatusCode::OK,
    )
    .oneshot(request("/api/v1/cases/unknown", "192.0.2.1:12", ""))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn health_and_unrecognized_paths_do_not_store_arbitrary_strings() {
    let sink = Arc::new(Sink::default());
    let router = app(
        sink.clone(),
        AuditProxyPolicy::Direct,
        UserId::new(),
        StatusCode::OK,
    );
    for path in ["/healthz", "/private-token"] {
        router
            .clone()
            .oneshot(request(path, "192.0.2.1:12", ""))
            .await
            .unwrap();
    }
    assert!(sink.records.lock().unwrap().is_empty());
}

#[tokio::test]
async fn cancelled_client_keeps_the_owned_operation_and_audit_alive() {
    let sink = Arc::new(Sink::default());
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let (ready, gate) = (entered.clone(), release.clone());
    let runtime = crate::runtime::HttpRuntime::new(crate::HttpLimits::default());
    let router = Router::new().route(
        "/api/v1/cases",
        get(move || {
            let (ready, gate) = (ready.clone(), gate.clone());
            async move {
                ready.notify_one();
                gate.notified().await;
                StatusCode::OK
            }
        }),
    );
    let router = with_access_audit(
        crate::runtime::protect(router, runtime),
        sink.clone(),
        AuditProxyPolicy::Direct,
    );
    let request = tokio::spawn(router.oneshot(request("/api/v1/cases", "192.0.2.1:12", "")));
    entered.notified().await;
    request.abort();
    release.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(2), sink.written.notified())
        .await
        .unwrap();
    assert_eq!(sink.records.lock().unwrap().len(), 1);
}
