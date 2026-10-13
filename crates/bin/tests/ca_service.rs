//! Native CA operator acceptance against explicitly supplied disposable backends.
#![cfg(target_os = "linux")]

#[allow(dead_code)]
#[path = "../../infrastructure/tests/ca_identity_cases/support.rs"]
mod ca_identity_support;
mod ca_service_support;

use application::identity::{SessionPolicy, SessionStore};
use domain::{
    audit::{verify_chain, AuditLog, ChainVerification},
    crypto::{RecoveryCodeSet, RECOVERY_CODE_COUNT},
};
use infrastructure::{RedisSessionStore, RingSha256Hasher};
use reqwest::{Method, StatusCode};
use serde_json::{json, Value};

use ca_identity_support::Fixture;
use ca_service_support::{denied, redis_url, response_json, take_secret, Credentials, Service};

#[test]
fn native_ca_operator_login_mfa_session_and_recovery_stay_outside_the_office() {
    let redis = redis_url();
    let mut fixture = Fixture::new();
    let credentials = Credentials::enroll(&mut fixture);
    let mut service = Service::start(&fixture, &credentials, &redis);
    let email = fixture.operator.email.clone();

    denied(
        service.post(
            "/api/v1/auth/login",
            json!({
                "email": "owner@example.test", "password": &*credentials.password,
            }),
        ),
        "invalid_credentials",
    );
    denied(
        service.post(
            "/api/v1/auth/login",
            json!({
                "email": email, "password": "incorrect-password",
            }),
        ),
        "invalid_credentials",
    );

    let rejected = service.login(&email, &credentials.password);
    denied(
        service.bearer(Method::GET, "/api/v1/auth/me", &rejected),
        "invalid_session",
    );
    denied(
        service.post(
            "/api/v1/auth/mfa/totp",
            json!({
                "challenge_token": &*rejected, "code": "invalid",
            }),
        ),
        "mfa_rejected",
    );
    denied(
        service.post(
            "/api/v1/auth/mfa/totp",
            json!({
                "challenge_token": &*rejected, "code": &*credentials.totp(),
            }),
        ),
        "mfa_rejected",
    );

    let challenge = service.login(&email, &credentials.password);
    let mut session = response_json(
        service.post(
            "/api/v1/auth/mfa/totp",
            json!({
                "challenge_token": &*challenge, "code": &*credentials.totp(),
            }),
        ),
        StatusCode::OK,
    );
    assert_eq!(session["token_type"], "Bearer");
    assert_eq!(session["expires_in_seconds"], 120);
    let token = take_secret(&mut session, "access_token");
    assert_operator(&session["operator"], &fixture);
    assert_ca_session_namespace(&redis, &token, &fixture);
    let me = response_json(
        service.bearer(Method::GET, "/api/v1/auth/me", &token),
        StatusCode::OK,
    );
    assert_operator(&me, &fixture);
    assert_session_activity(&service, &token, &session, &fixture);
    assert_office_routes_absent(&service, &token);
    logout(&service, &token);

    let challenge = service.login(&email, &credentials.password);
    let mut recovered = response_json(
        service.post(
            "/api/v1/auth/mfa/recovery",
            json!({
                "challenge_token": &*challenge, "code": &*credentials.recovery[0],
            }),
        ),
        StatusCode::OK,
    );
    let token = take_secret(&mut recovered, "access_token");
    assert_operator(&recovered["operator"], &fixture);
    assert_operator(
        &response_json(
            service.bearer(Method::GET, "/api/v1/auth/me", &token),
            StatusCode::OK,
        ),
        &fixture,
    );
    denied(
        service.post(
            "/api/v1/auth/mfa/recovery",
            json!({
                "challenge_token": &*challenge, "code": &*credentials.recovery[0],
            }),
        ),
        "mfa_rejected",
    );
    logout(&service, &token);
    service.stop();

    assert_persistence(&mut fixture);
}

fn assert_operator(value: &Value, fixture: &Fixture) {
    assert_eq!(value["id"], fixture.operator.id.to_string());
    assert_eq!(value["email"], fixture.operator.email);
    assert!(value.get("role").is_none());
    assert_eq!(value.as_object().unwrap().len(), 2);
}

fn assert_ca_session_namespace(redis: &str, token: &str, fixture: &Fixture) {
    let policy = SessionPolicy::new(300, Some(120)).unwrap();
    let ca = RedisSessionStore::connect_ca(redis).unwrap();
    let office = RedisSessionStore::connect(redis).unwrap();
    let session = ca
        .find_session(token, policy)
        .unwrap()
        .expect("CA bearer was not persisted in its own Redis namespace");
    assert_eq!(session.identity.principal.id, fixture.operator.id);
    assert!(
        office.find_session(token, policy).unwrap().is_none(),
        "CA bearer leaked into the office Redis namespace"
    );
}

fn assert_session_activity(service: &Service, token: &str, session: &Value, fixture: &Fixture) {
    let status = response_json(
        service.bearer(Method::GET, "/api/v1/auth/session", token),
        StatusCode::OK,
    );
    assert_operator(&status["operator"], fixture);
    assert_eq!(
        status["policy"],
        json!({"absolute_ttl_seconds": 300, "idle_ttl_seconds": 120})
    );
    assert_eq!(
        status["absolute_expires_at_unix_ms"],
        session["absolute_expires_at_unix_ms"]
    );
    assert_eq!(
        status["idle_expires_at_unix_ms"],
        session["idle_expires_at_unix_ms"]
    );
    let active = response_json(
        service.bearer(Method::POST, "/api/v1/auth/activity", token),
        StatusCode::OK,
    );
    assert_operator(&active["operator"], fixture);
    assert_eq!(
        active["absolute_expires_at_unix_ms"],
        status["absolute_expires_at_unix_ms"]
    );
    let now = active["server_now_unix_ms"].as_i64().unwrap();
    let idle = active["idle_expires_at_unix_ms"].as_i64().unwrap();
    let absolute = active["absolute_expires_at_unix_ms"].as_i64().unwrap();
    assert!(idle >= status["idle_expires_at_unix_ms"].as_i64().unwrap());
    assert_eq!(idle, (now + 120_000).min(absolute));
    assert!(active.get("access_token").is_none());
}

fn assert_office_routes_absent(service: &Service, token: &str) {
    for (method, path) in [
        (Method::POST, "/api/v1/auth/bootstrap"),
        (Method::POST, "/api/v1/users"),
        (Method::GET, "/api/v1/cases"),
        (Method::POST, "/api/v1/cases"),
        (
            Method::GET,
            "/api/v1/cases/00000000-0000-0000-0000-000000000001/documents",
        ),
    ] {
        assert_eq!(
            service.bearer(method, path, token).status(),
            StatusCode::NOT_FOUND,
            "office route was composed into the CA service: {path}"
        );
    }
}

fn logout(service: &Service, token: &str) {
    assert_eq!(
        service
            .bearer(Method::POST, "/api/v1/auth/logout", token)
            .status(),
        StatusCode::NO_CONTENT
    );
    denied(
        service.bearer(Method::GET, "/api/v1/auth/me", token),
        "invalid_session",
    );
    denied(
        service.bearer(Method::POST, "/api/v1/auth/activity", token),
        "invalid_session",
    );
}

fn assert_persistence(fixture: &mut Fixture) {
    let id = fixture.operator.id.as_uuid();
    let row = fixture
        .admin()
        .query_one(
            "SELECT recovery_codes,revision,role FROM qadra_ca.users WHERE id=$1",
            &[&id],
        )
        .unwrap();
    let codes: Value = row.get(0);
    assert!(codes["slots"][0].is_null());
    let codes: RecoveryCodeSet = serde_json::from_value(codes).unwrap();
    assert_eq!(codes.remaining(), RECOVERY_CODE_COUNT - 1);
    assert_eq!(row.get::<_, i64>(1), 1);
    assert_eq!(row.get::<_, String>(2), "client");
    assert_eq!(
        fixture
            .admin()
            .query_one("SELECT count(*) FROM qadra_ca.users", &[])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    fixture.assert_office_unchanged();
    assert_eq!(
        fixture
            .admin()
            .query_one("SELECT count(*) FROM public.office_sentinel", &[])
            .unwrap()
            .get::<_, i64>(0),
        1
    );

    let (_users, audit) = fixture.open().unwrap();
    let events = audit.load_all().unwrap();
    assert_eq!(
        verify_chain(&RingSha256Hasher, &events).unwrap(),
        ChainVerification::Valid {
            entries: events.len()
        }
    );
    for (action, count) in [
        ("identity.password_rejected", 1),
        ("identity.password_accepted", 3),
        ("identity.totp_accepted", 1),
        ("identity.recovery_consumed", 1),
        ("identity.recovery_accepted", 1),
        ("identity.session_revoked", 2),
    ] {
        assert_eq!(
            events
                .iter()
                .filter(|entry| entry.event.action == action)
                .count(),
            count,
            "unexpected number of CA audit events for {action}"
        );
    }
    assert!(events
        .iter()
        .all(|entry| entry.event.actor == fixture.operator.email));
    assert!(!events.iter().any(|entry| matches!(
        entry.event.action.as_str(),
        "identity.owner_bootstrapped" | "identity.user_created"
    )));
}
