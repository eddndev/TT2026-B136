use super::support::{binding_route, public_json, Capture};
use crate::{
    password_reset_identity_support::{ok, Acceptance},
    serve_password_reset_http_acceptance_support::{me, post},
};
use application::{
    credential_trust::CredentialTrustSnapshot,
    identity::{certificate_login::SessionAuthentication, Principal, SessionPolicy, SessionStore},
};
use axum::{http::StatusCode, Router};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{
    audit::{AuditLog, ChainVerification},
    crypto::DocumentHasher,
};
use infrastructure::{PostgresAuditLog, RingSha256Hasher};
use serde_json::Value;
use tokio::runtime::Runtime;
use uuid::Uuid;

pub fn certificate_origin(
    flow: &Acceptance,
    policy: SessionPolicy,
    token: &str,
    owner: &Principal,
    trust: &CredentialTrustSnapshot,
    receipt: &Value,
    capture: &Capture,
) {
    let session = ok(
        flow.sessions.find_session(token, policy),
        "read real certificate session",
    )
    .expect("certificate session stored");
    let SessionAuthentication::Certificate(origin) = &session.authentication else {
        panic!("certificate proof was downgraded to password provenance");
    };
    let registration = &receipt["registration"];
    let certificate = &registration["certificate"];
    let summary = &certificate["summary"];
    let leaf = STANDARD
        .decode(certificate["der_base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(&session.identity.principal, owner);
    assert_eq!(&origin.principal, owner);
    assert_eq!(origin.auth_generation, session.identity.auth_generation);
    assert_eq!(origin.binding_id.to_string(), receipt["binding_id"]);
    assert_eq!(origin.deployment_id, trust.deployment_id);
    assert_eq!(origin.trust_revision, trust.revision.get());
    assert_eq!(origin.root_fingerprint, trust.inspection.root_fingerprint);
    assert_eq!(origin.leaf_fingerprint, RingSha256Hasher.hash_bytes(&leaf));
    assert_eq!(origin.crl_digest, trust.inspection.crl_digest);
    assert_eq!(
        origin.valid_from_unix_seconds,
        summary["not_before_unix"]
            .as_i64()
            .unwrap()
            .max(trust.inspection.valid_from)
    );
    assert_eq!(
        origin.valid_until_unix_seconds,
        summary["not_after_unix"]
            .as_i64()
            .unwrap()
            .min(trust.inspection.valid_until)
    );
    assert_eq!(&capture.statement[10..26], trust.deployment_id.as_bytes());
    assert_eq!(
        &capture.statement[26..58],
        trust.inspection.root_fingerprint.as_bytes()
    );
    assert_eq!(
        &capture.statement[58..62],
        &trust.revision.get().to_be_bytes()
    );
    assert_eq!(
        &capture.statement[78..86],
        &origin.auth_generation.to_be_bytes()
    );
    assert_eq!(
        &capture.statement[102..134],
        origin.leaf_fingerprint.as_bytes()
    );
    assert!(
        session.absolute_expires_at_unix_ms > capture.expires * 1000,
        "presentation deadline incorrectly capped the derived session"
    );
    assert!(session.absolute_expires_at_unix_ms <= origin.valid_until_unix_seconds * 1000);
    assert!(session.absolute_expires_at_unix_ms <= session.server_now_unix_ms + 3_600_000);
    assert!(session.idle_expires_at_unix_ms.is_none());
}

pub fn admitted(runtime: &Runtime, router: &Router, token: &str, binding: Uuid, receipt: &Value) {
    assert_eq!(runtime.block_on(me(router, token)).status, StatusCode::OK);
    for (method, path) in [
        ("GET", "/api/v1/auth/session"),
        ("POST", "/api/v1/auth/activity"),
    ] {
        public_json(runtime, router, method, path, token, None, StatusCode::OK);
    }
    assert_eq!(
        public_json(
            runtime,
            router,
            "GET",
            &binding_route(binding),
            token,
            None,
            StatusCode::OK
        ),
        *receipt
    );
}

pub fn denied(runtime: &Runtime, router: &Router, token: &str, binding: Uuid) {
    runtime
        .block_on(me(router, token))
        .error(StatusCode::UNAUTHORIZED, "invalid_session");
    for (method, path) in [
        ("GET", "/api/v1/auth/session".to_owned()),
        ("POST", "/api/v1/auth/activity".to_owned()),
        ("GET", binding_route(binding)),
    ] {
        let value = public_json(
            runtime,
            router,
            method,
            &path,
            token,
            None,
            StatusCode::UNAUTHORIZED,
        );
        assert!(
            value["error"]["code"] == "invalid_session",
            "certificate authority rejection"
        );
    }
}

pub fn rejected_mfa(runtime: &Runtime, router: &Router, challenge: &str, code: &str) {
    runtime
        .block_on(post(
            router,
            "/api/v1/auth/mfa/recovery",
            &[("challenge_token", challenge), ("code", code)],
        ))
        .error(StatusCode::UNAUTHORIZED, "mfa_rejected");
}

pub fn audit(flow: &mut Acceptance, binding: Uuid) {
    let reader = ok(
        PostgresAuditLog::open(&flow.db.runtime_url),
        "open native audit reader",
    );
    let events = ok(reader.load_all(), "read native audit chain");
    assert!(
        matches!(domain::audit::verify_chain(&RingSha256Hasher, &events),
        Ok(ChainVerification::Valid { entries }) if entries == events.len()),
        "native audit chain invalid"
    );
    for action in [
        "identity.owner_certificate_registered",
        "identity.owner_certificate_withdrawn",
    ] {
        assert_eq!(
            events
                .iter()
                .filter(|entry| entry.event.action == action
                    && entry
                        .event
                        .resource
                        .starts_with(&format!("owner-certificate:{binding}:")))
                .count(),
            1
        );
    }
    let row = flow
        .db
        .admin
        .query_one(
            "SELECT
        (SELECT count(*) FROM owner_certificate_registrations WHERE binding_id=$1),
        (SELECT count(*) FROM owner_certificate_withdrawals WHERE binding_id=$1)",
            &[&binding],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), 1);
    assert_eq!(row.get::<_, i64>(1), 1);
}
