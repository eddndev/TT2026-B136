mod auth_support;
use auth_support::UnusedDocuments;

use std::sync::Arc;

use application::identity::{
    EnrollmentResult, IdentityWorkflow, LoginChallenge, Principal, SessionResult,
};
use application::ApplicationError;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use domain::identity::{Permission, Role, UserId};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;
use web::application_router;
use zeroize::Zeroizing;

struct StubIdentity;

impl StubIdentity {
    fn principal(role: Role) -> Principal {
        Principal {
            id: UserId::from_uuid(Uuid::from_u128(9)),
            email: "owner@example.com".to_string(),
            role,
        }
    }

    fn enrollment(role: Role) -> EnrollmentResult {
        EnrollmentResult {
            principal: Self::principal(role),
            totp_secret_base32: Zeroizing::new("BASE32SECRET".to_string()),
            otpauth_uri: Zeroizing::new("otpauth://totp/app:owner".to_string()),
            recovery_codes: vec![Zeroizing::new("RECOVERY-0".to_string())],
        }
    }
}

impl IdentityWorkflow for StubIdentity {
    fn bootstrap_owner(
        &self,
        email: &str,
        password: &str,
    ) -> Result<EnrollmentResult, ApplicationError> {
        assert_eq!(email, "owner@example.com");
        assert_eq!(password, "correct horse battery");
        Ok(Self::enrollment(Role::Owner))
    }

    fn create_user(
        &self,
        token: &str,
        _email: &str,
        _password: &str,
        role: Role,
    ) -> Result<EnrollmentResult, ApplicationError> {
        self.authorize(token, Permission::CreateUser)?;
        Ok(Self::enrollment(role))
    }

    fn start_login(&self, email: &str, password: &str) -> Result<LoginChallenge, ApplicationError> {
        if email != "owner@example.com" || password != "correct horse battery" {
            return Err(ApplicationError::InvalidCredentials);
        }
        Ok(LoginChallenge {
            challenge_token: "challenge-token".to_string(),
            expires_in_seconds: 300,
        })
    }

    fn complete_totp(
        &self,
        challenge_token: &str,
        code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        if challenge_token != "challenge-token" || code != "123456" {
            return Err(ApplicationError::MfaRejected);
        }
        Ok(SessionResult {
            access_token: "owner-token".to_string(),
            expires_in_seconds: 86_400,
            principal: Self::principal(Role::Owner),
        })
    }

    fn complete_recovery(
        &self,
        _challenge_token: &str,
        _code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        Err(ApplicationError::MfaRejected)
    }

    fn authenticate(&self, access_token: &str) -> Result<Principal, ApplicationError> {
        match access_token {
            "owner-token" => Ok(Self::principal(Role::Owner)),
            "client-token" => Ok(Self::principal(Role::Client)),
            _ => Err(ApplicationError::InvalidSession),
        }
    }

    fn authorize(
        &self,
        token: &str,
        permission: Permission,
    ) -> Result<Principal, ApplicationError> {
        let principal = self.authenticate(token)?;
        if principal.role.allows(permission) {
            Ok(principal)
        } else {
            Err(ApplicationError::PermissionDenied)
        }
    }

    fn logout(&self, access_token: &str) -> Result<(), ApplicationError> {
        self.authenticate(access_token).map(|_| ())
    }
}

fn router() -> axum::Router {
    application_router(Arc::new(UnusedDocuments), Arc::new(StubIdentity))
}

async fn json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn enrollment_login_totp_me_and_logout_have_stable_contracts() {
    let bootstrap = router()
        .oneshot(
            Request::post("/api/v1/auth/bootstrap")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"owner@example.com","password":"correct horse battery"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bootstrap.status(), StatusCode::CREATED);
    let enrollment = json(bootstrap).await;
    assert_eq!(enrollment["user"]["role"], "owner");
    assert_eq!(enrollment["totp_secret_base32"], "BASE32SECRET");
    assert_eq!(enrollment["recovery_codes"][0], "RECOVERY-0");

    let login = router()
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"owner@example.com","password":"correct horse battery"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(json(login).await["challenge_token"], "challenge-token");

    let totp = router()
        .oneshot(
            Request::post("/api/v1/auth/mfa/totp")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"challenge_token":"challenge-token","code":"123456"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let session = json(totp).await;
    assert_eq!(session["token_type"], "Bearer");
    assert_eq!(session["access_token"], "owner-token");

    let me = router()
        .oneshot(
            Request::get("/api/v1/auth/me")
                .header("authorization", "Bearer owner-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(json(me).await["email"], "owner@example.com");

    let logout = router()
        .oneshot(
            Request::post("/api/v1/auth/logout")
                .header("authorization", "Bearer owner-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn invalid_login_and_client_authorization_have_distinct_statuses() {
    let login = router()
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"nobody@example.com","password":"wrong"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json(login).await["error"]["code"], "invalid_credentials");

    let denied = router()
        .oneshot(
            Request::post("/api/v1/cases/00000000-0000-0000-0000-000000000001/documents")
                .header("authorization", "Bearer client-token")
                .header("x-document-name", "acta.txt")
                .body(Body::from("content"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(json(denied).await["error"]["code"], "permission_denied");
}

#[tokio::test]
async fn identity_json_has_a_small_body_limit_and_rejects_spoofed_fields() {
    for (body, expected) in [
        (serde_json::json!({"email":"owner@example.com","password":"x".repeat(17*1024)}).to_string(), StatusCode::PAYLOAD_TOO_LARGE),
        (serde_json::json!({"email":"owner@example.com","password":"correct horse battery","role":"owner"}).to_string(), StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let response=router().oneshot(Request::post("/api/v1/auth/login").header("content-type","application/json").body(Body::from(body)).unwrap()).await.unwrap();
        assert_eq!(response.status(),expected);
    }
}

#[tokio::test]
async fn bearer_headers_reject_ambiguity_and_accept_case_insensitive_scheme() {
    let response = router()
        .oneshot(
            Request::get("/api/v1/auth/me")
                .header("authorization", "Bearer owner-token")
                .header("authorization", "Bearer client-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = router()
        .oneshot(
            Request::get("/api/v1/auth/me")
                .header("authorization", "bearer owner-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
}
