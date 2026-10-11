use crate::{
    case_administration_support::Fixture as Database,
    password_reset_composition_support::router_with_identity_invitations,
    password_reset_runtime_support::Fixture as RedisKeys,
    serve_identity_composition,
    serve_invitation_composition::compose,
    serve_invitation_runtime::InvitationConsumer,
    serve_password_reset_http_acceptance_support::{login, mfa, track_token},
};
use application::{
    identity::{
        invitations::*, password_reset::ResetDeliveryOutcome, IdentityWorkflow, SessionPolicy,
    },
    ApplicationError,
};
use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Request, StatusCode},
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use domain::{
    clock::Clock,
    crypto::{DocumentHasher, TotpProvider},
    identity::UserId,
};
use infrastructure::{decode_base32_secret, RingSha256Hasher, SystemClock, TotpRsProvider};
use redis::Commands;
use serde_json::{json, Value};
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::runtime::Runtime;
use tower::ServiceExt;
use uuid::Uuid;
use web::{HttpLimits, HttpWorkBudget};
use zeroize::Zeroizing;

pub const OWNER_PATH: &str = "/api/v1/member-invitations";
pub const PUBLIC_PATH: &str = "/api/v1/auth/invitations";
pub const PASSWORD: &str = "native invitation acceptance password";
const GLOBAL: &str = "identity:invitation:v1:global";

pub struct Delivered {
    pub id: InvitationId,
    pub revision: u64,
    pub email: String,
    pub token: Zeroizing<Vec<u8>>,
}

#[derive(Default)]
pub struct Capture(Mutex<Vec<Delivered>>);
impl InvitationDelivery for Capture {
    fn deliver(
        &self,
        lease: &InvitationDeliveryLease,
        token: Zeroizing<Vec<u8>>,
    ) -> Result<ResetDeliveryOutcome, ApplicationError> {
        self.0.lock().unwrap().push(Delivered {
            id: lease.id,
            revision: lease.revision,
            email: lease.email.clone(),
            token,
        });
        Ok(ResetDeliveryOutcome::Accepted)
    }
}

pub struct Acceptance {
    pub runtime: Runtime,
    pub router: Router,
    pub identity: Arc<dyn IdentityWorkflow>,
    pub worker: InvitationConsumer,
    pub capture: Arc<Capture>,
    pub keys: RedisKeys,
    pub db: Database,
}
impl Acceptance {
    pub fn new() -> Self {
        let mut keys = RedisKeys::new();
        assert!(
            !keys.connection.exists::<_, bool>(GLOBAL).unwrap(),
            "invitation global already exists; fixture will not claim it"
        );
        keys.track(GLOBAL);
        let mut db =
            Database::old().expect("invitation acceptance requires CASE_TEST_DATABASE_URL");
        assert_eq!(
            db.admin
                .execute("DELETE FROM cases WHERE id=$1", &[&db.case.as_uuid()])
                .unwrap(),
            1
        );
        assert_eq!(
            db.admin
                .execute("DELETE FROM users WHERE id=$1", &[&db.owner.as_uuid()])
                .unwrap(),
            1
        );
        db.migrate();
        let identity = serve_identity_composition::open(
            &db.runtime_url,
            &keys.url,
            Zeroizing::new(vec![42; 32]),
            SessionPolicy::new(300, None).unwrap(),
            None,
        )
        .unwrap_or_else(|_| panic!("open native invitation identity"))
        .identity;
        let capture = Arc::new(Capture::default());
        let (http, worker) = compose(
            &db.runtime_url,
            &keys.url,
            identity.clone(),
            Zeroizing::new(vec![42; 32]),
            capture.clone(),
        )
        .unwrap_or_else(|_| panic!("compose native invitations"));
        let router = router(identity.clone(), http.unwrap());
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        Self {
            runtime,
            router,
            identity,
            worker: worker.unwrap(),
            capture,
            keys,
            db,
        }
    }

    pub fn owner(&mut self) -> Zeroizing<String> {
        let email = format!("invitation-owner-{}@example.test", Uuid::new_v4().simple());
        let enrollment = self
            .identity
            .bootstrap_owner(&email, PASSWORD)
            .unwrap_or_else(|_| panic!("bootstrap native invitation owner"));
        self.authenticate(
            &email,
            enrollment.principal.id,
            &enrollment.totp_secret_base32,
        )
    }

    pub fn authenticate(&mut self, email: &str, id: UserId, secret: &str) -> Zeroizing<String> {
        track_token(&mut self.keys, "password-failures", email);
        let challenge = login(&self.runtime, &self.router, &mut self.keys, email, PASSWORD);
        let code = code(secret);
        let mut claim = Zeroizing::new(id.as_uuid().as_bytes().to_vec());
        claim.extend_from_slice(code.as_bytes());
        self.keys.track(format!(
            "identity:totp-used:{}",
            RingSha256Hasher.hash_bytes(&claim).to_hex()
        ));
        mfa(
            &self.runtime,
            &self.router,
            &mut self.keys,
            "totp",
            &challenge,
            &code,
        )
    }

    pub fn deliver(&mut self) -> Delivered {
        assert_eq!(
            (self.worker)().unwrap(),
            InvitationDeliveryRun::Recorded(InvitationDeliveryState::Sent)
        );
        let mut deliveries = self.capture.0.lock().unwrap();
        assert_eq!(deliveries.len(), 1);
        let delivered = deliveries.pop().unwrap();
        self.keys.track(format!(
            "identity:invitation:v1:token:{}",
            digest(&delivered.token).to_hex()
        ));
        delivered
    }

    pub fn post(&self, path: &str, bearer: Option<&str>, body: Value) -> Reply {
        self.runtime
            .block_on(post(&self.router, path, bearer, body))
    }

    pub fn reopen(&mut self) {
        let (http, worker) = compose(
            &self.db.runtime_url,
            &self.keys.url,
            self.identity.clone(),
            Zeroizing::new(vec![42; 32]),
            self.capture.clone(),
        )
        .unwrap_or_else(|_| panic!("reopen native invitations"));
        self.router = router(self.identity.clone(), http.unwrap());
        self.worker = worker.unwrap();
    }

    pub fn user_count(&mut self, email: &str) -> i64 {
        self.db
            .admin
            .query_one("SELECT count(*) FROM users WHERE email=$1", &[&email])
            .unwrap()
            .get(0)
    }

    pub fn accepted_count(&mut self) -> i64 {
        self.db
            .admin
            .query_one("SELECT count(*) FROM member_invitation_completions", &[])
            .unwrap()
            .get(0)
    }
}

fn router(identity: Arc<dyn IdentityWorkflow>, http: web::InvitationHttp) -> Router {
    let limits = HttpLimits {
        max_requests: NonZeroUsize::new(4).unwrap(),
        max_blocking_operations: NonZeroUsize::new(1).unwrap(),
    };
    router_with_identity_invitations(
        identity,
        http,
        limits,
        HttpWorkBudget::new(limits.max_blocking_operations),
    )
}

pub fn digest(token: &[u8]) -> domain::crypto::Sha256Digest {
    let mut bytes = Zeroizing::new(b"qadra:invitation:v1\0".to_vec());
    bytes.extend_from_slice(token);
    RingSha256Hasher.hash_bytes(&bytes)
}

pub fn code(secret: &str) -> Zeroizing<String> {
    let secret = decode_base32_secret(secret).unwrap_or_else(|_| panic!("decode enrollment TOTP"));
    Zeroizing::new(
        TotpRsProvider::new()
            .current_code(
                &secret,
                SystemClock::new()
                    .now()
                    .unix_timestamp()
                    .try_into()
                    .unwrap(),
            )
            .unwrap(),
    )
}

pub fn recipient(token: &[u8], operation: Uuid) -> Value {
    json!({"token":URL_SAFE_NO_PAD.encode(token),"operation_id":operation,"password":PASSWORD})
}

pub struct Reply {
    pub status: StatusCode,
    headers: HeaderMap,
    body: Zeroizing<Vec<u8>>,
}
impl Reply {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
    pub fn no_session(&self) {
        assert_eq!(self.headers["cache-control"], "no-store");
        for header in ["set-cookie", "location", "authorization"] {
            assert!(!self.headers.contains_key(header));
        }
        let text = String::from_utf8_lossy(&self.body);
        for secret in [PASSWORD, "access_token", "challenge_token"] {
            assert!(
                !text.contains(secret),
                "invitation response disclosed login credentials"
            );
        }
    }
    pub fn metadata(&self) {
        self.no_session();
        let text = String::from_utf8_lossy(&self.body);
        for name in [
            "totp_secret",
            "otpauth_uri",
            "recovery_codes",
            "token_digest",
            "protected_token",
        ] {
            assert!(
                !text.contains(name),
                "invitation metadata disclosed protected material"
            );
        }
    }
    pub fn secret(&self, field: &str) -> Zeroizing<String> {
        let mut body = self.json();
        match body[field].take() {
            Value::String(value) => Zeroizing::new(value),
            _ => panic!("missing enrollment secret"),
        }
    }
}

async fn post(router: &Router, path: &str, bearer: Option<&str>, body: Value) -> Reply {
    let mut request = Request::post(path).header("content-type", "application/json");
    if let Some(token) = bearer {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = tokio::time::timeout(
        Duration::from_secs(90),
        router
            .clone()
            .oneshot(request.body(Body::from(body.to_string())).unwrap()),
    )
    .await
    .expect("native invitation HTTP watchdog")
    .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = Zeroizing::new(
        to_bytes(response.into_body(), 32 * 1024)
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
