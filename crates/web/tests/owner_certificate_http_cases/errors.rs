use application::{identity::owner_certificates::OwnerCertificateError as Error, ApplicationError};
use axum::http::StatusCode;
use domain::{crypto::CredentialFailure, identity::Role};
use serde_json::json;

use crate::{owner_certificate_http_support::*, submission_support, support::*};

#[tokio::test]
async fn all_routes_require_one_bearer_and_current_owner_before_accessing_evidence_ports() {
    for (method, operation, body) in [
        ("POST", "/prepare", Some(prepare_body())),
        ("POST", "/register", Some(registration_body())),
        ("GET", "", None),
        ("POST", "/withdraw", Some(json!({"expected_revision":1}))),
    ] {
        for headers in [
            vec![],
            vec!["Basic session"],
            vec!["Bearer "],
            vec!["Bearer session", "Bearer session"],
        ] {
            let harness = Harness::new();
            let observed = harness.state.clone();
            let app = router(harness);
            raw(
                &app,
                method,
                &path(operation),
                body.as_ref()
                    .map(|v| v.to_string().into_bytes())
                    .unwrap_or_default(),
                &["application/json"],
                &headers,
            )
            .await
            .error(StatusCode::UNAUTHORIZED, "invalid_session");
            assert!(observed.lock().unwrap().calls.is_empty());
        }
        for role in [
            None,
            Some(Role::Litigator),
            Some(Role::Paralegal),
            Some(Role::Client),
        ] {
            let harness = Harness::new();
            let observed = harness.state.clone();
            observed.lock().unwrap().principal =
                role.map(|role| application::identity::Principal {
                    role,
                    ..principal()
                });
            let response = request(&router(harness), method, operation, body.clone()).await;
            if role.is_none() {
                response.error(StatusCode::UNAUTHORIZED, "invalid_session");
            } else {
                response.error(StatusCode::FORBIDDEN, "permission_denied");
            }
            assert_eq!(observed.lock().unwrap().calls, vec!["authenticate"]);
        }
    }
}

#[tokio::test]
async fn receipt_absence_and_lost_current_authority_never_expose_historical_evidence() {
    let app = router(Harness::new());
    request(&app, "GET", "", None)
        .await
        .error(StatusCode::NOT_FOUND, "owner_certificate_not_found");
    let harness = Harness::new();
    let observed = harness.state.clone();
    observed.lock().unwrap().found = Some(retired());
    observed.lock().unwrap().expire_on_find = true;
    let response = request(&router(harness), "GET", "", None).await;
    response.error(StatusCode::UNAUTHORIZED, "invalid_session");
    assert!(!String::from_utf8_lossy(&response.bytes).contains("Synthetic Partner"));
    submission_support::no_crypto_or_commit(&observed);
}

#[tokio::test]
async fn application_failures_have_static_categories_and_no_private_diagnostics() {
    for (error, status, code) in [
        (
            Error::InvalidInput,
            StatusCode::BAD_REQUEST,
            "owner_certificate_invalid_input",
        ),
        (
            Error::NotFound,
            StatusCode::NOT_FOUND,
            "owner_certificate_not_found",
        ),
        (
            Error::AccountChanged,
            StatusCode::CONFLICT,
            "owner_certificate_conflict",
        ),
        (
            Error::TrustUnavailable,
            StatusCode::CONFLICT,
            "owner_certificate_conflict",
        ),
        (
            Error::TrustChanged,
            StatusCode::CONFLICT,
            "owner_certificate_conflict",
        ),
        (
            Error::BindingConflict,
            StatusCode::CONFLICT,
            "owner_certificate_conflict",
        ),
        (
            Error::FingerprintConflict,
            StatusCode::CONFLICT,
            "owner_certificate_conflict",
        ),
        (
            Error::ActiveBinding,
            StatusCode::CONFLICT,
            "owner_certificate_conflict",
        ),
        (
            Error::RevisionConflict,
            StatusCode::CONFLICT,
            "owner_certificate_conflict",
        ),
        (
            Error::MaterialMismatch,
            StatusCode::UNPROCESSABLE_ENTITY,
            "owner_certificate_credential_rejected",
        ),
        (
            Error::Credential(CredentialFailure::InvalidSignature),
            StatusCode::UNPROCESSABLE_ENTITY,
            "owner_certificate_credential_rejected",
        ),
        (
            Error::Inconsistent,
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
        ),
    ] {
        let mut harness = Harness::new();
        harness.store = MockStore::new();
        harness
            .store
            .expect_find()
            .times(1)
            .returning(move |actor, id| {
                assert_eq!((actor, id), (principal().id, binding()));
                Err(error.into())
            });
        request(&router(harness), "GET", "", None)
            .await
            .error(status, code);
    }
    let mut harness = Harness::new();
    harness.verifier = MockVerifier::new();
    harness
        .verifier
        .expect_inspect_certificate()
        .times(1)
        .returning(|_| Err(CredentialFailure::MalformedCertificate));
    request(&router(harness), "POST", "/prepare", Some(prepare_body()))
        .await
        .error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "owner_certificate_credential_rejected",
        );
}

#[tokio::test]
async fn uncertain_registration_commit_is_not_retried_and_storage_detail_stays_private() {
    let mut harness = submission_support::harness();
    let observed = harness.state.clone();
    harness.verify_with(|_, _| {});
    let calls = observed.clone();
    harness
        .store
        .expect_commit_registration()
        .times(1)
        .returning(move |_| {
            calls.lock().unwrap().calls.push("commit_registration");
            Err(ApplicationError::Port(
                "private-port-detail includes synthetic material".into(),
            ))
        });
    request(
        &router(harness),
        "POST",
        "/register",
        Some(registration_body()),
    )
    .await
    .error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error");
    assert_eq!(submission_support::count(&observed, "verify"), 1);
    assert_eq!(
        submission_support::count(&observed, "commit_registration"),
        1
    );
}
