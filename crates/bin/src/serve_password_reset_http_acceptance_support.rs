use crate::{
    password_reset_runtime_support::Fixture,
    serve_password_reset_config::{PasswordResetOptions, PasswordResetSettings},
};
use application::{
    identity::password_reset::{PasswordResetDelivery, ResetDeliveryOutcome, ResetEnvelope},
    ApplicationError,
};
use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Request, StatusCode},
    Router,
};
use domain::crypto::{DocumentHasher, Sha256Digest};
use infrastructure::RingSha256Hasher;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        mpsc, Mutex,
    },
    time::Duration,
};
use tokio::{runtime::Runtime, sync::Notify};
use tower::ServiceExt;
use zeroize::Zeroizing;

pub const WATCHDOG: Duration = Duration::from_secs(30);
pub const REQUEST: &str = "/api/v1/auth/password-reset/request";
pub const COMPLETE: &str = "/api/v1/auth/password-reset/complete";

pub struct Capture {
    envelope: Mutex<Option<ResetEnvelope>>,
    arrived: Notify,
    pub calls: AtomicUsize,
    release: Mutex<mpsc::Receiver<()>>,
}

impl Capture {
    pub fn new() -> (Self, mpsc::Sender<()>) {
        let (release, wait) = mpsc::channel();
        (
            Self {
                envelope: Mutex::new(None),
                arrived: Notify::new(),
                calls: AtomicUsize::new(0),
                release: Mutex::new(wait),
            },
            release,
        )
    }

    pub async fn take(&self) -> ResetEnvelope {
        tokio::time::timeout(WATCHDOG, self.arrived.notified())
            .await
            .expect("recovery delivery did not arrive");
        self.envelope
            .lock()
            .unwrap()
            .take()
            .expect("captured delivery")
    }
}

impl PasswordResetDelivery for Capture {
    fn deliver(&self, envelope: ResetEnvelope) -> Result<ResetDeliveryOutcome, ApplicationError> {
        assert_eq!(self.calls.fetch_add(1, SeqCst), 0, "duplicate delivery");
        *self.envelope.lock().unwrap() = Some(envelope);
        self.arrived.notify_one();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(WATCHDOG)
            .map_err(|_| ApplicationError::Port("local capture was not released".into()))?;
        Ok(ResetDeliveryOutcome::Accepted)
    }
}

pub fn settings() -> Option<PasswordResetSettings> {
    PasswordResetSettings::resolve(
        PasswordResetOptions {
            enabled: true,
            from_email: Some("recovery@example.test".into()),
            public_url: Some("https://qadra.example.test/".into()),
            ttl_seconds: Some(300),
            max_pending: Some(2),
            request_global_max: Some(8),
            request_global_window_seconds: Some(300),
            request_email_max: Some(2),
            request_email_window_seconds: Some(300),
            complete_global_max: Some(8),
            complete_global_window_seconds: Some(300),
            complete_token_max: Some(4),
            complete_token_window_seconds: Some(300),
            redis_connect_ms: Some(1000),
            redis_io_ms: Some(1000),
            email_connect_ms: Some(1000),
            email_total_ms: Some(2000),
        },
        Some(Zeroizing::new("local-capture-not-a-provider-key".into())),
    )
    .unwrap_or_else(|_| panic!("valid isolated recovery settings"))
}

pub struct Reply {
    pub status: StatusCode,
    headers: HeaderMap,
    pub body: Zeroizing<Vec<u8>>,
}

impl Reply {
    pub fn reset_public(&self, secrets: &[&str]) {
        assert_eq!(self.headers["cache-control"], "no-store");
        for header in ["set-cookie", "location", "authorization", "retry-after"] {
            assert!(!self.headers.contains_key(header), "private reset header");
        }
        let text = String::from_utf8_lossy(&self.body);
        for secret in secrets {
            assert!(!text.contains(secret), "reset response exposed input");
        }
    }

    pub fn error(&self, status: StatusCode, code: &str) {
        assert_eq!(self.status, status);
        let value: serde_json::Value = serde_json::from_slice(&self.body).unwrap();
        assert!(
            value["error"]["code"] == code,
            "unexpected public error code"
        );
    }

    pub fn secret(&self, field: &str) -> Zeroizing<String> {
        assert_eq!(self.status, StatusCode::OK);
        let mut value: serde_json::Value = serde_json::from_slice(&self.body).unwrap();
        match value[field].take() {
            serde_json::Value::String(secret) => Zeroizing::new(secret),
            _ => panic!("expected credential field"),
        }
    }
}

async fn send(router: &Router, request: Request<Body>) -> Reply {
    let response = tokio::time::timeout(WATCHDOG, router.clone().oneshot(request))
        .await
        .expect("HTTP acceptance watchdog")
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = Zeroizing::new(
        to_bytes(response.into_body(), 16 * 1024)
            .await
            .unwrap()
            .to_vec(),
    );
    Reply {
        status,
        headers,
        body,
    }
}

pub async fn post(router: &Router, path: &str, fields: &[(&str, &str)]) -> Reply {
    let borrowed: BTreeMap<_, _> = fields.iter().copied().collect();
    let body = Zeroizing::new(serde_json::to_vec(&borrowed).unwrap());
    send(
        router,
        Request::post(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_vec()))
            .unwrap(),
    )
    .await
}

pub async fn me(router: &Router, token: &str) -> Reply {
    let bearer = Zeroizing::new(format!("Bearer {token}"));
    send(
        router,
        Request::get("/api/v1/auth/me")
            .header("authorization", bearer.as_str())
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

pub fn track_token(keys: &mut Fixture, namespace: &str, token: &str) {
    keys.track(format!(
        "identity:{namespace}:{}",
        RingSha256Hasher.hash_bytes(token.as_bytes()).to_hex()
    ));
}

pub fn login(
    runtime: &Runtime,
    router: &Router,
    keys: &mut Fixture,
    email: &str,
    password: &str,
) -> Zeroizing<String> {
    let reply = runtime.block_on(post(
        router,
        "/api/v1/auth/login",
        &[("email", email), ("password", password)],
    ));
    let challenge = reply.secret("challenge_token");
    track_token(keys, "challenge", &challenge);
    assert!(
        !String::from_utf8_lossy(&reply.body).contains("access_token"),
        "password alone created a session"
    );
    challenge
}

pub fn mfa(
    runtime: &Runtime,
    router: &Router,
    keys: &mut Fixture,
    kind: &str,
    challenge: &str,
    code: &str,
) -> Zeroizing<String> {
    let reply = runtime.block_on(post(
        router,
        &format!("/api/v1/auth/mfa/{kind}"),
        &[("challenge_token", challenge), ("code", code)],
    ));
    let token = reply.secret("access_token");
    track_token(keys, "session", &token);
    token
}

pub fn reset_digest(token: &[u8]) -> Sha256Digest {
    let mut bytes = Zeroizing::new(b"qadra:password-reset:v1\0".to_vec());
    bytes.extend_from_slice(token);
    RingSha256Hasher.hash_bytes(&bytes)
}
