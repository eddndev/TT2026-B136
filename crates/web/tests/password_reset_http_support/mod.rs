mod ports;
pub use ports::{Mode, Ports, TOKEN};

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use application::identity::password_reset::{
    PasswordResetPorts, PasswordResetService, ResetPolicy,
};
use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Request, StatusCode},
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::Value;
use tower::ServiceExt;
pub use web::password_reset::RequestAdmission;
use web::password_reset::{PasswordResetHttp, PasswordResetRequests};
use web::{password_reset_router, HttpLimits};
use zeroize::Zeroizing;

pub const REQUEST: &str = "/api/v1/auth/password-reset/request";
pub const COMPLETE: &str = "/api/v1/auth/password-reset/complete";
pub const PASSWORD: &str = "synthetic replacement password";

pub struct Requests {
    pub admission: RequestAdmission,
    pub calls: Mutex<usize>,
    pub retained: Mutex<Vec<Zeroizing<String>>>,
}

impl PasswordResetRequests for Requests {
    fn try_submit(&self, email: Zeroizing<String>) -> RequestAdmission {
        *self.calls.lock().unwrap() += 1;
        if matches!(self.admission, RequestAdmission::Accepted) {
            self.retained.lock().unwrap().push(email);
        }
        self.admission
    }
}

pub struct Harness {
    pub router: Router,
    pub requests: Arc<Requests>,
    pub ports: Arc<Ports>,
}

impl Harness {
    pub fn new(
        admission: RequestAdmission,
        enabled: bool,
        requests: usize,
        blocking: usize,
    ) -> Self {
        let requests_port = Arc::new(Requests {
            admission,
            calls: Mutex::new(0),
            retained: Mutex::new(Vec::new()),
        });
        let ports = Arc::new(Ports::default());
        let completion = Arc::new(PasswordResetService::new(
            PasswordResetPorts {
                repository: ports.clone(),
                delivery: ports.clone(),
                limiter: ports.clone(),
                tokens: ports.clone(),
                digests: ports.clone(),
                passwords: ports.clone(),
            },
            ResetPolicy::new(60, 2).unwrap(),
        ));
        let components = enabled.then(|| PasswordResetHttp {
            requests: requests_port.clone(),
            completion,
        });
        let router = password_reset_router(
            components,
            HttpLimits {
                max_requests: NonZeroUsize::new(requests).unwrap(),
                max_blocking_operations: NonZeroUsize::new(blocking).unwrap(),
            },
        );
        Self {
            router,
            requests: requests_port,
            ports,
        }
    }

    pub fn ready() -> Self {
        Self::new(RequestAdmission::Accepted, true, 2, 1)
    }

    pub fn untouched(&self) {
        assert_eq!(*self.requests.calls.lock().unwrap(), 0);
        assert!(
            self.ports.calls().is_empty(),
            "malformed input reached reset ports"
        );
    }
}

#[derive(PartialEq, Eq)]
pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }

    pub fn public(&self) {
        assert_eq!(self.headers["cache-control"], "no-store");
        for name in [
            "set-cookie",
            "location",
            "authorization",
            "www-authenticate",
            "retry-after",
            "x-ratelimit-limit",
            "x-ratelimit-remaining",
            "x-ratelimit-reset",
        ] {
            assert!(
                !self.headers.contains_key(name),
                "reset disclosed credential or quota headers"
            );
        }
        let text = String::from_utf8_lossy(&self.body);
        for secret in [PASSWORD, "private-reset-detail", "private@example.test"] {
            assert!(
                !text.contains(secret),
                "reset response disclosed private input"
            );
        }
        assert!(!text.contains(&token()), "reset response disclosed token");
    }

    pub fn error(&self, status: StatusCode, code: &str) {
        self.public();
        assert_eq!(self.status, status);
        assert_eq!(self.json()["error"]["code"], code);
    }
}

pub fn token() -> String {
    URL_SAFE_NO_PAD.encode(TOKEN)
}
pub fn completion(token: &str, password: &str) -> String {
    serde_json::json!({"token":token,"new_password":password}).to_string()
}
pub fn email(value: &str) -> String {
    serde_json::json!({"email":value}).to_string()
}

pub async fn raw(router: &Router, path: &str, body: Body, types: &[&str]) -> Reply {
    let mut request = Request::post(path);
    for value in types {
        request = request.header("content-type", *value);
    }
    let response = tokio::time::timeout(
        Duration::from_secs(2),
        router.clone().oneshot(request.body(body).unwrap()),
    )
    .await
    .expect("reset HTTP did not finish within the test bound")
    .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap()
        .to_vec();
    Reply {
        status,
        headers,
        body,
    }
}

pub async fn post(router: &Router, path: &str, body: String) -> Reply {
    raw(router, path, Body::from(body), &["application/json"]).await
}
