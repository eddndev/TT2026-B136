use std::{sync::Arc, time::Duration};

use application::{
    ca_access::{CaOperatorAccess, OperatorIdentity},
    identity::{IdentityService, SessionPolicy},
};
use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Request, StatusCode},
    Router,
};
use domain::identity::{Role, UserId};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;
use web::HttpLimits;

use crate::identity_fixture::Env;

pub const EMAIL: &str = "ca@example.test";
pub const PASSWORD: &str = "separate ca password";
pub const PROTECTED: [(&str, &str); 4] = [
    ("GET", "/api/v1/auth/me"),
    ("GET", "/api/v1/auth/session"),
    ("POST", "/api/v1/auth/activity"),
    ("POST", "/api/v1/auth/logout"),
];

pub fn id(value: u128) -> UserId {
    UserId::from_uuid(Uuid::from_u128(value))
}

pub struct Harness {
    pub router: Router,
    pub env: Env,
    pub identity: Arc<IdentityService>,
    pub operator: OperatorIdentity,
}

impl Harness {
    pub fn new() -> Self {
        Self::configured(Role::Client, id(1))
    }

    pub fn configured(role: Role, configured_id: UserId) -> Self {
        let env = Env::new();
        env.edit(|state| {
            state.user.email = EMAIL.into();
            state.user.role = role;
            state.user.password_hash = format!("fake:{PASSWORD}");
        });
        let identity = Arc::new(env.service(false, SessionPolicy::new(300, Some(60)).unwrap()));
        let operator = OperatorIdentity::new(configured_id, EMAIL).unwrap();
        let access = Arc::new(CaOperatorAccess::new(identity.clone(), operator.clone()));
        Self {
            router: web::ca_access_router(access, HttpLimits::default()),
            env,
            identity,
            operator,
        }
    }

    pub async fn challenge(&self) -> String {
        let response = post(
            &self.router,
            "/api/v1/auth/login",
            json!({"email": EMAIL, "password": PASSWORD}),
        )
        .await;
        assert_eq!(response.status, StatusCode::OK);
        public(&response);
        let value = response.json();
        assert_eq!(value["expires_in_seconds"], 300);
        assert_eq!(value.as_object().unwrap().len(), 2);
        value["challenge_token"].as_str().unwrap().into()
    }

    pub async fn session(&self, factor: &str) -> Reply {
        let challenge = self.challenge().await;
        factor_request(&self.router, factor, &challenge, factor_code(factor)).await
    }

    pub fn raw_session(&self) -> String {
        let challenge = self.identity.start_login(EMAIL, PASSWORD).unwrap();
        self.identity
            .complete_totp(&challenge.challenge_token, "123456")
            .unwrap()
            .access_token
    }
}

pub fn factor_code(factor: &str) -> &'static str {
    match factor {
        "totp" => "123456",
        "recovery" => "RECOVERY-0",
        _ => panic!("unknown test factor"),
    }
}

pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}

pub async fn send(router: &Router, request: Request<Body>) -> Reply {
    let response = tokio::time::timeout(Duration::from_secs(3), router.clone().oneshot(request))
        .await
        .expect("CA HTTP request exceeded the test bound")
        .unwrap();
    Reply {
        status: response.status(),
        headers: response.headers().clone(),
        body: to_bytes(response.into_body(), 16 * 1024)
            .await
            .unwrap()
            .to_vec(),
    }
}

pub async fn post(router: &Router, path: &str, value: Value) -> Reply {
    send(
        router,
        Request::post(path)
            .header("content-type", "application/json")
            .body(Body::from(value.to_string()))
            .unwrap(),
    )
    .await
}

pub async fn factor_request(router: &Router, factor: &str, challenge: &str, code: &str) -> Reply {
    post(
        router,
        &format!("/api/v1/auth/mfa/{factor}"),
        json!({"challenge_token": challenge, "code": code}),
    )
    .await
}

pub async fn bearer(router: &Router, method: &str, path: &str, token: &str) -> Reply {
    send(
        router,
        Request::builder()
            .method(method)
            .uri(path)
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

pub fn public(response: &Reply) {
    assert_eq!(response.headers["cache-control"], "no-store");
    for name in ["set-cookie", "authorization", "location"] {
        assert!(!response.headers.contains_key(name));
    }
}

pub fn operator_status(value: &Value, operator: &OperatorIdentity, now: i64, idle: i64) {
    assert_eq!(value["operator"], serde_json::to_value(operator).unwrap());
    assert_eq!(value["policy"]["absolute_ttl_seconds"], 300);
    assert_eq!(value["policy"]["idle_ttl_seconds"], 60);
    assert_eq!(value["server_now_unix_ms"], now);
    assert_eq!(value["absolute_expires_at_unix_ms"], 1_300_000);
    assert_eq!(value["idle_expires_at_unix_ms"], idle);
    for name in [
        "user",
        "role",
        "principal",
        "auth_generation",
        "recovery_codes",
    ] {
        assert!(value.get(name).is_none(), "private field {name}");
    }
}
