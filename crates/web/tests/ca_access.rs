mod ca_access_support;
#[allow(dead_code)]
#[path = "../../application/tests/owner_certificate_login_cases/mod.rs"]
mod identity_fixture;

use axum::{body::Body, http::Request};
use domain::identity::Role;
use serde_json::json;

use axum::http::StatusCode;
use ca_access_support::*;

#[tokio::test]
async fn password_only_returns_a_challenge_and_never_authorizes_protected_routes() {
    let harness = Harness::new();
    for (email, password) in [(EMAIL, "wrong password"), ("owner@example.test", PASSWORD)] {
        let denied = post(
            &harness.router,
            "/api/v1/auth/login",
            json!({"email": email, "password": password}),
        )
        .await;
        assert_eq!(denied.status, StatusCode::UNAUTHORIZED);
        assert_eq!(denied.json()["error"]["code"], "invalid_credentials");
        public(&denied);
    }
    let challenge = harness.challenge().await;
    for (method, path) in PROTECTED {
        let denied = bearer(&harness.router, method, path, &challenge).await;
        assert_eq!(denied.status, StatusCode::UNAUTHORIZED);
        assert_eq!(denied.json()["error"]["code"], "invalid_session");
        public(&denied);
    }
    assert_eq!(harness.env.read(|state| state.sessions_created), 0);
}

#[tokio::test]
async fn both_factors_return_operator_session_contract_and_a_role_free_identity() {
    for factor in ["totp", "recovery"] {
        let harness = Harness::new();
        let response = harness.session(factor).await;
        assert_eq!(response.status, StatusCode::OK);
        public(&response);
        let value = response.json();
        assert_eq!(value["token_type"], "Bearer");
        assert_eq!(value["expires_in_seconds"], 60);
        operator_status(&value, &harness.operator, 1_000_000, 1_060_000);
        let token = value["access_token"].as_str().unwrap();
        let me = bearer(&harness.router, "GET", "/api/v1/auth/me", token).await;
        assert_eq!(me.status, StatusCode::OK);
        public(&me);
        assert_eq!(me.json(), serde_json::to_value(&harness.operator).unwrap());
        let status = bearer(&harness.router, "GET", "/api/v1/auth/session", token).await;
        assert_eq!(status.status, StatusCode::OK);
        public(&status);
        operator_status(&status.json(), &harness.operator, 1_000_000, 1_060_000);
        assert!(status.json().get("access_token").is_none());
        assert!(status.json().get("token_type").is_none());
    }
}

#[tokio::test]
async fn invalid_second_factors_consume_the_challenge_without_issuing_a_session() {
    for factor in ["totp", "recovery"] {
        let harness = Harness::new();
        let challenge = harness.challenge().await;
        for code in ["invalid-factor", factor_code(factor)] {
            let response = factor_request(&harness.router, factor, &challenge, code).await;
            assert_eq!(response.status, StatusCode::UNAUTHORIZED);
            assert_eq!(response.json()["error"]["code"], "mfa_rejected");
            public(&response);
        }
        assert_eq!(harness.env.read(|state| state.sessions_created), 0);
    }
}

#[tokio::test]
async fn owner_role_and_unconfigured_uuid_are_rejected_after_either_factor() {
    for (role, configured_id) in [(Role::Owner, id(1)), (Role::Client, id(2))] {
        for factor in ["totp", "recovery"] {
            let harness = Harness::configured(role, configured_id);
            let response = harness.session(factor).await;
            assert_eq!(response.status, StatusCode::FORBIDDEN);
            assert_eq!(response.json()["error"]["code"], "permission_denied");
            assert!(response.json().get("access_token").is_none());
            public(&response);
            assert!(harness.env.read(|state| state.sessions.is_empty()));
            assert_eq!(harness.env.read(|state| state.revocations), 1);
        }
    }
}

#[tokio::test]
async fn foreign_bearers_never_read_touch_or_revoke_a_ca_session() {
    for (role, configured_id) in [(Role::Owner, id(1)), (Role::Client, id(2))] {
        let harness = Harness::configured(role, configured_id);
        let token = harness.raw_session();
        for (method, path) in PROTECTED {
            let denied = bearer(&harness.router, method, path, &token).await;
            assert_eq!(denied.status, StatusCode::FORBIDDEN);
            assert_eq!(denied.json()["error"]["code"], "permission_denied");
            public(&denied);
        }
        assert_eq!(harness.env.read(|state| state.touches), 0);
        assert_eq!(harness.env.read(|state| state.revocations), 0);
    }
    let harness = Harness::new();
    for (method, path) in PROTECTED {
        let denied = bearer(&harness.router, method, path, "unrelated-office-token").await;
        assert_eq!(denied.status, StatusCode::UNAUTHORIZED);
        assert_eq!(denied.json()["error"]["code"], "invalid_session");
    }
}

#[tokio::test]
async fn logout_revokes_the_token_for_every_protected_route() {
    let harness = Harness::new();
    let token = harness.session("totp").await.json()["access_token"]
        .as_str()
        .unwrap()
        .to_owned();
    let logout = bearer(&harness.router, "POST", "/api/v1/auth/logout", &token).await;
    assert_eq!(logout.status, StatusCode::NO_CONTENT);
    assert!(logout.body.is_empty());
    public(&logout);
    for (method, path) in PROTECTED {
        let denied = bearer(&harness.router, method, path, &token).await;
        assert_eq!(denied.status, StatusCode::UNAUTHORIZED);
        assert_eq!(denied.json()["error"]["code"], "invalid_session");
    }
}

#[tokio::test]
async fn identity_and_session_reads_do_not_extend_idle_expiration() {
    let harness = Harness::new();
    let token = harness.raw_session();
    harness.env.edit(|state| state.now = 1030);
    let me = bearer(&harness.router, "GET", "/api/v1/auth/me", &token).await;
    assert_eq!(me.status, StatusCode::OK);
    let status = bearer(&harness.router, "GET", "/api/v1/auth/session", &token).await;
    assert_eq!(status.status, StatusCode::OK);
    operator_status(&status.json(), &harness.operator, 1_030_000, 1_060_000);
    assert_eq!(harness.env.read(|state| state.touches), 0);
    harness.env.edit(|state| state.now = 1060);
    let expired = bearer(&harness.router, "POST", "/api/v1/auth/activity", &token).await;
    assert_eq!(expired.status, StatusCode::UNAUTHORIZED);
    assert_eq!(expired.json()["error"]["code"], "invalid_session");
    assert_eq!(harness.env.read(|state| state.touches), 0);
}

#[tokio::test]
async fn explicit_activity_extends_idle_without_extending_absolute_expiration() {
    let harness = Harness::new();
    let token = harness.raw_session();
    for now in [1030, 1080, 1130, 1180, 1230, 1280] {
        harness.env.edit(|state| state.now = now);
        let status = bearer(&harness.router, "POST", "/api/v1/auth/activity", &token).await;
        assert_eq!(status.status, StatusCode::OK);
        public(&status);
        operator_status(
            &status.json(),
            &harness.operator,
            now * 1000,
            ((now + 60) * 1000).min(1_300_000),
        );
        assert!(status.json().get("access_token").is_none());
    }
    assert_eq!(harness.env.read(|state| state.touches), 6);
    harness.env.edit(|state| state.now = 1300);
    let expired = bearer(&harness.router, "POST", "/api/v1/auth/activity", &token).await;
    assert_eq!(expired.status, StatusCode::UNAUTHORIZED);
    assert_eq!(expired.json()["error"]["code"], "invalid_session");
}

#[tokio::test]
async fn session_routes_reject_caller_supplied_deadlines_and_ambiguous_bearers() {
    let harness = Harness::new();
    let token = harness.raw_session();
    for (method, path) in [
        ("GET", "/api/v1/auth/session"),
        ("POST", "/api/v1/auth/activity"),
    ] {
        for (query, body) in [("?idle_ttl_seconds=86400", ""), ("", "{}"), ("", " ")] {
            let request = Request::builder()
                .method(method)
                .uri(format!("{path}{query}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body))
                .unwrap();
            let denied = send(&harness.router, request).await;
            assert_eq!(denied.status, StatusCode::BAD_REQUEST);
            assert_eq!(denied.json()["error"]["code"], "invalid_session_request");
            public(&denied);
        }
        for headers in [
            vec![],
            vec!["Basic unrelated"],
            vec!["Bearer a", "Bearer b"],
        ] {
            let mut request = Request::builder().method(method).uri(path);
            for value in headers {
                request = request.header("authorization", value);
            }
            let denied = send(&harness.router, request.body(Body::empty()).unwrap()).await;
            assert_eq!(denied.status, StatusCode::UNAUTHORIZED);
            assert_eq!(denied.json()["error"]["code"], "invalid_session");
        }
    }
    assert_eq!(harness.env.read(|state| state.touches), 0);
}

#[tokio::test]
async fn ca_router_exposes_no_bootstrap_user_creation_or_case_routes() {
    let harness = Harness::new();
    let token = harness.raw_session();
    for (method, path) in [
        ("POST", "/api/v1/auth/bootstrap"),
        ("POST", "/api/v1/users"),
        ("GET", "/api/v1/cases"),
        ("POST", "/api/v1/cases"),
        (
            "GET",
            "/api/v1/cases/00000000-0000-0000-0000-000000000001/documents",
        ),
    ] {
        let response = bearer(&harness.router, method, path, &token).await;
        assert_eq!(response.status, StatusCode::NOT_FOUND, "{method} {path}");
    }
}
