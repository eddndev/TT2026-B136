mod workflow;
pub use workflow::{Call, Mode, Workflow};

use std::{num::NonZeroUsize, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Request, StatusCode},
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;
use web::{member_invitation_router, HttpLimits, HttpWorkBudget, InvitationHttp};

pub const OWNER: &str = "/api/v1/member-invitations";
pub const PUBLIC: &str = "/api/v1/auth/invitations";
pub const TOKEN: [u8; 32] = [0x5a; 32];
pub const PASSWORD: &str = "synthetic recipient password";
pub const CODE: &str = "123456";
pub const REVISION: u64 = 9_007_199_254_740_993;
pub const SECRET: &str = "PRIVATE-INVITATION-SECRET";
pub const PRIVATE_ERROR: &str = "private invitation provider detail";

pub fn operation() -> Uuid {
    Uuid::from_u128(41)
}
pub fn invitation() -> Uuid {
    Uuid::from_u128(31)
}
pub fn token() -> String {
    URL_SAFE_NO_PAD.encode(TOKEN)
}
pub fn recipient() -> Value {
    json!({"token":token(),"operation_id":operation(),"password":PASSWORD})
}
pub fn issue() -> Value {
    json!({"operation_id":operation(),"email":"staff@example.test","role":"litigator"})
}
pub fn resend() -> Value {
    json!({"operation_id":operation(),"expected_revision":REVISION.to_string()})
}
pub fn summary() -> Value {
    json!({"id":invitation(),"revision":REVISION.to_string(),"operation_id":operation(),
        "email":"staff@example.test","role":"litigator","status":"pending",
        "delivery":"pending","issued_at":"1970-01-01T00:00:00Z",
        "expires_at":"1970-01-02T00:00:00Z","accepted_at":null})
}
pub fn receipt() -> Value {
    json!({"id":invitation(),"revision":REVISION.to_string(),"operation_id":operation(),
        "user_id":Uuid::from_u128(51),"email":"staff@example.test","role":"litigator",
        "accepted_at":"1970-01-01T00:01:00Z"})
}

pub struct Harness {
    pub workflow: Arc<Workflow>,
    pub router: Router,
    pub budget: HttpWorkBudget,
}
impl Harness {
    pub fn new(enabled: bool, requests: usize) -> Self {
        let workflow = Arc::new(Workflow::default());
        let limits = HttpLimits {
            max_requests: NonZeroUsize::new(requests).unwrap(),
            max_blocking_operations: NonZeroUsize::new(1).unwrap(),
        };
        let budget = HttpWorkBudget::new(limits.max_blocking_operations);
        let components = enabled.then(|| InvitationHttp {
            workflow: workflow.clone(),
        });
        let router = member_invitation_router(components, limits, budget.clone()).unwrap();
        Self {
            workflow,
            router,
            budget,
        }
    }
    pub fn ready() -> Self {
        Self::new(true, 4)
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
    pub fn private(&self) {
        assert_eq!(self.headers["cache-control"], "no-store");
        for name in [
            "set-cookie",
            "location",
            "authorization",
            "www-authenticate",
        ] {
            assert!(!self.headers.contains_key(name));
        }
        let text = String::from_utf8_lossy(&self.body);
        for secret in [PASSWORD, CODE, PRIVATE_ERROR, &token()] {
            assert!(
                !text.contains(secret),
                "response disclosed request credentials"
            );
        }
        assert!(!text.contains("access_token"));
        assert!(!text.contains("challenge_token"));
    }
    pub fn metadata(&self) {
        self.private();
        let text = String::from_utf8_lossy(&self.body);
        for secret in [
            SECRET,
            "totp_secret",
            "otpauth_uri",
            "recovery_codes",
            "token_digest",
        ] {
            assert!(
                !text.contains(secret),
                "metadata disclosed enrollment material"
            );
        }
    }
    pub fn error(&self, status: StatusCode, code: &str) {
        self.metadata();
        assert_eq!(self.status, status);
        assert_eq!(self.json()["error"]["code"], code);
    }
}

pub async fn raw(
    router: &Router,
    method: &str,
    path: &str,
    body: Body,
    headers: &[(&str, &str)],
) -> Reply {
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = router
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 32 * 1024)
        .await
        .unwrap()
        .to_vec();
    Reply {
        status,
        headers,
        body,
    }
}
pub async fn call(
    router: &Router,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    body: Option<Value>,
) -> Reply {
    let authorization = bearer.map(|token| format!("Bearer {token}"));
    let mut headers = Vec::new();
    if let Some(value) = authorization.as_deref() {
        headers.push(("authorization", value));
    }
    if body.is_some() {
        headers.push(("content-type", "application/json"));
    }
    raw(
        router,
        method,
        path,
        body.map_or_else(Body::empty, |v| Body::from(v.to_string())),
        &headers,
    )
    .await
}
