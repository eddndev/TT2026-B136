use super::material::Material;
use crate::{
    password_reset_identity_support::ok,
    password_reset_runtime_support::Fixture,
    serve_args::OwnerLoginArgs,
    serve_owner_login_config::OwnerLoginSettings,
    serve_password_reset_http_acceptance_support::{post, track_token, WATCHDOG},
};
use application::identity::Principal;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use domain::{crypto::DocumentHasher, identity::UserId};
use infrastructure::RingSha256Hasher;
use redis::Commands;
use serde_json::{json, Value};
use tokio::runtime::Runtime;
use tower::ServiceExt;
use uuid::Uuid;
use zeroize::Zeroizing;

pub const START: &str = "/api/v1/auth/certificate-login/start";
pub const PROOF: &str = "/api/v1/auth/certificate-login/proof";
pub const START_GLOBAL: &str = "identity:certificate-login-rate:v1:start:global";
pub const PROOF_GLOBAL: &str = "identity:certificate-login-rate:v1:proof:global";

pub fn require_backends() {
    for name in ["CASE_TEST_DATABASE_URL", "IDENTITY_TEST_REDIS_URL"] {
        let value =
            std::env::var(name).expect("native Owner login requires both disposable backend URLs");
        let parsed = reqwest::Url::parse(&value).expect("invalid native backend URL");
        assert!(
            matches!(
                parsed.host_str(),
                Some("127.0.0.1" | "localhost" | "::1" | "[::1]")
            ),
            "native Owner login requires loopback disposable backends"
        );
    }
}

pub fn settings() -> OwnerLoginSettings {
    ok(
        OwnerLoginSettings::resolve(&OwnerLoginArgs {
            enabled: true,
            start_global_max: Some(64),
            start_global_window_seconds: Some(300),
            start_owner_binding_max: Some(64),
            start_owner_binding_window_seconds: Some(300),
            proof_global_max: Some(64),
            proof_global_window_seconds: Some(300),
            proof_token_max: Some(8),
            proof_token_window_seconds: Some(300),
            redis_connect_ms: Some(1000),
            redis_io_ms: Some(1000),
        }),
        "resolve enabled Owner login settings",
    )
    .expect("Owner login enabled")
}

pub fn own_login_keys(keys: &mut Fixture, owner: UserId, binding: Uuid) {
    let parsed = redis::parse_redis_url(&keys.url).expect("disposable Redis URL");
    assert!(
        matches!(
            parsed.host_str(),
            Some("127.0.0.1" | "localhost" | "::1" | "[::1]")
        ),
        "native login requires loopback Redis"
    );
    let count: u64 = redis::cmd("EXISTS")
        .arg(START_GLOBAL)
        .arg(PROOF_GLOBAL)
        .query(&mut keys.connection)
        .expect("check unowned login globals");
    assert_eq!(
        count, 0,
        "native acceptance will not claim existing login budgets"
    );
    keys.track(START_GLOBAL);
    keys.track(PROOF_GLOBAL);
    let mut bytes = b"qadra:owner-certificate-login-start-limit:v1\0".to_vec();
    bytes.extend_from_slice(owner.as_uuid().as_bytes());
    bytes.extend_from_slice(binding.as_bytes());
    keys.track(format!(
        "identity:certificate-login-rate:v1:start:owner-binding:{}",
        RingSha256Hasher.hash_bytes(&bytes).to_hex()
    ));
}

pub struct Capture {
    pub token: Zeroizing<String>,
    pub statement: Vec<u8>,
    pub signature: Vec<u8>,
    pub expires: i64,
}

pub fn start(
    runtime: &Runtime,
    router: &Router,
    keys: &mut Fixture,
    material: &Material,
    owner: &Principal,
    binding: Uuid,
) -> Capture {
    let reply = runtime.block_on(post(
        router,
        START,
        &[
            ("owner_id", &owner.id.to_string()),
            ("binding_id", &binding.to_string()),
        ],
    ));
    reply.reset_public(&[]);
    let token = reply.secret("challenge_token");
    track_token(keys, "certificate-login", &token);
    let mut proof_scope =
        Zeroizing::new(b"qadra:owner-certificate-login-proof-limit:v1\0".to_vec());
    proof_scope.extend_from_slice(token.as_bytes());
    keys.track(format!(
        "identity:certificate-login-rate:v1:proof:token:{}",
        RingSha256Hasher.hash_bytes(&proof_scope).to_hex()
    ));
    assert_eq!(token.len(), 43);
    let decoded = Zeroizing::new(
        URL_SAFE_NO_PAD
            .decode(token.as_bytes())
            .expect("canonical token"),
    );
    assert_eq!(decoded.len(), 32);
    assert!(
        URL_SAFE_NO_PAD.encode(decoded.as_slice()) == token.as_str(),
        "canonical capture token"
    );
    let mut value: Value = serde_json::from_slice(&reply.body).expect("start JSON");
    value["challenge_token"] = Value::Null;
    assert_eq!(value.as_object().unwrap().len(), 3);
    let bytes = STANDARD
        .decode(
            value["statement_base64"]
                .as_str()
                .expect("statement base64"),
        )
        .expect("decode public login statement");
    assert!(
        STANDARD.encode(&bytes) == value["statement_base64"],
        "canonical statement encoding"
    );
    assert_eq!(bytes.len(), 182);
    assert_eq!(&bytes[..10], b"OWNAUTH1\x01\x01");
    assert_eq!(&bytes[62..78], owner.id.as_uuid().as_bytes());
    assert_eq!(&bytes[86..102], binding.as_bytes());
    let issued = i64::from_be_bytes(bytes[166..174].try_into().unwrap());
    let expires = i64::from_be_bytes(bytes[174..182].try_into().unwrap());
    assert!(issued >= 0 && (1..=300).contains(&(expires - issued)));
    assert!((1..=300).contains(&value["expires_in_seconds"].as_u64().unwrap()));
    let key = format!(
        "identity:certificate-login:{}",
        RingSha256Hasher.hash_bytes(token.as_bytes()).to_hex()
    );
    assert_eq!(keys.deadline(&key), expires * 1000);
    let signature = material.sign(&bytes);
    Capture {
        token,
        statement: bytes,
        signature,
        expires,
    }
}

pub fn proof(
    runtime: &Runtime,
    router: &Router,
    keys: &mut Fixture,
    capture: &Capture,
) -> Zeroizing<String> {
    let reply = runtime.block_on(post(
        router,
        PROOF,
        &[
            ("challenge_token", &capture.token),
            ("signature_base64", &STANDARD.encode(&capture.signature)),
        ],
    ));
    reply.reset_public(&[]);
    let challenge = reply.secret("challenge_token");
    track_token(keys, "challenge", &challenge);
    let mut value: Value = serde_json::from_slice(&reply.body).expect("proof JSON");
    value["challenge_token"] = Value::Null;
    assert_eq!(value.as_object().unwrap().len(), 2);
    assert!(value.get("access_token").is_none());
    assert!((1..=300).contains(&value["expires_in_seconds"].as_u64().unwrap()));
    challenge
}

pub fn proof_rejected(runtime: &Runtime, router: &Router, capture: &Capture, signature: &[u8]) {
    runtime
        .block_on(post(
            router,
            PROOF,
            &[
                ("challenge_token", &capture.token),
                ("signature_base64", &STANDARD.encode(signature)),
            ],
        ))
        .error(StatusCode::UNAUTHORIZED, "invalid_credentials");
}

pub fn public_json(
    runtime: &Runtime,
    router: &Router,
    method: &str,
    path: &str,
    token: &str,
    body: Option<Value>,
    status: StatusCode,
) -> Value {
    let bearer = Zeroizing::new(format!("Bearer {token}"));
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", bearer.as_str());
    let body = if let Some(body) = body {
        request = request.header("content-type", "application/json");
        Body::from(serde_json::to_vec(&body).expect("public request JSON"))
    } else {
        Body::empty()
    };
    let response = runtime.block_on(async {
        tokio::time::timeout(
            WATCHDOG,
            router.clone().oneshot(request.body(body).unwrap()),
        )
        .await
        .expect("native Owner HTTP watchdog")
        .unwrap()
    });
    assert_eq!(
        response.status(),
        status,
        "native Owner public request status"
    );
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert!(!response.headers().contains_key("set-cookie"));
    let raw = Zeroizing::new(
        runtime
            .block_on(to_bytes(response.into_body(), 64 * 1024))
            .expect("bounded public response")
            .to_vec(),
    );
    serde_json::from_slice(&raw).expect("public response JSON")
}

pub fn binding_route(binding: Uuid) -> String {
    format!("/api/v1/auth/certificate-bindings/{binding}")
}

pub fn register(
    runtime: &Runtime,
    router: &Router,
    material: &Material,
    token: &str,
    binding: Uuid,
) -> Value {
    let path = binding_route(binding);
    let prepared = public_json(
        runtime,
        router,
        "POST",
        &format!("{path}/prepare"),
        token,
        Some(json!({"certificate_base64": STANDARD.encode(&material.leaf)})),
        StatusCode::OK,
    );
    let bytes = STANDARD
        .decode(prepared["statement_base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(bytes.len(), 150);
    let receipt = public_json(
        runtime,
        router,
        "POST",
        &format!("{path}/register"),
        token,
        Some(json!({"statement_base64": prepared["statement_base64"],
            "certificate_der_base64": prepared["certificate"]["der_base64"],
            "signature_base64": STANDARD.encode(material.sign(&bytes))})),
        StatusCode::OK,
    );
    assert_eq!(receipt["binding_id"], binding.to_string());
    assert_eq!(receipt["revision"], 1);
    assert!(receipt["withdrawal"].is_null());
    let current = public_json(
        runtime,
        router,
        "GET",
        "/api/v1/auth/certificate-bindings/current",
        token,
        None,
        StatusCode::OK,
    );
    assert_eq!(current, receipt);
    receipt
}

pub fn absent_capture(keys: &mut Fixture, capture: &Capture) {
    let key = format!(
        "identity:certificate-login:{}",
        RingSha256Hasher
            .hash_bytes(capture.token.as_bytes())
            .to_hex()
    );
    assert!(
        !keys.connection.exists::<_, bool>(key).unwrap(),
        "proof capture was not consumed"
    );
}
