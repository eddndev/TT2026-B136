#[allow(dead_code)]
#[path = "../../application/tests/case_document_support/mod.rs"]
mod case_document_support;
#[allow(dead_code)]
#[path = "../../application/tests/case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
#[path = "../../application/tests/document_signing_support/mod.rs"]
mod document_signing_support;
#[allow(dead_code)]
#[path = "../../application/tests/individual_document_support/mod.rs"]
mod individual_document_support;
mod individual_signing_http_support;

use base64::{engine::general_purpose::STANDARD, Engine};
use document_signing_support as bindings;
use individual_document_support as docs;
use individual_signing_http_support::*;
use serde_json::json;

#[tokio::test]
async fn binding_preparation_and_current_expose_only_normalized_public_context() {
    let app = router(bindings::Harness::new(), docs::Harness::new());
    let prepared = request(
        app.clone(),
        "POST",
        &binding_path("prepare"),
        json!({"certificate_base64":STANDARD.encode(bindings::certificate().der)}),
    )
    .await;
    assert_eq!(prepared.0, 200);
    assert_eq!(
        prepared.1["account_id"],
        bindings::principal().id.to_string()
    );
    assert_eq!(prepared.1["account_label"], bindings::principal().email);
    assert_eq!(
        prepared.1["statement_base64"],
        STANDARD.encode(bindings::statement().canonical_bytes())
    );
    assert_eq!(prepared.1["account_revision"], "9");
    assert_eq!(prepared.1["policy"], "internal_individual_signing_v1");
    let current = request(
        app,
        "GET",
        "/api/v1/identity/signing-certificates/current",
        json!(null),
    )
    .await;
    assert_eq!(current, (200, json!({"binding":null})));
}

#[tokio::test]
async fn exact_pdf_preparation_and_recovery_keep_canonical_operation_identity() {
    let mut document = docs::Harness::new();
    document.verify_with(|_, _| {});
    let app = router(bindings::Harness::new(), document);
    let prepared = request(app.clone(), "POST", &document_path("prepare"),
        json!({"operation_id":docs::operation(),"pdf_signature_base64":STANDARD.encode(docs::pdf_signature().as_bytes())})).await;
    assert_eq!(prepared.0, 200);
    assert_eq!(
        prepared.1["consent_base64"],
        STANDARD.encode(docs::consent().canonical_bytes())
    );
    assert_eq!(prepared.1["name"], "selected.pdf");
    assert_eq!(prepared.1["version"], 1);
    let accepted = request(app.clone(), "POST", &document_path("submit"), submission()).await;
    assert_eq!(accepted.0, 200);
    assert_eq!(accepted.1["status"], "accepted");
    assert_eq!(
        accepted.1["account_id"],
        bindings::principal().id.to_string()
    );
    let recovered = request(
        app,
        "GET",
        &document_path(&format!("receipts/{}", docs::operation())),
        json!(null),
    )
    .await;
    assert_eq!(accepted, recovered);
}

#[tokio::test]
async fn url_identity_is_authoritative_and_malformed_public_material_never_reaches_crypto() {
    let app = router(bindings::Harness::new(), docs::Harness::new());
    let wrong = document_path("submit").replace("/versions/1/", "/versions/2/");
    let result = request(app.clone(), "POST", &wrong, submission()).await;
    assert_eq!(result.0, 409);
    assert_eq!(result.1["error"]["code"], "signing_conflict");
    for body in [
        json!([]),
        json!({"certificate_base64":"?"}),
        json!({"certificate_base64":"AQ==","password":"forbidden"}),
        json!({"certificate_base64":"AQ==","private_key":"forbidden"}),
    ] {
        let result = request(app.clone(), "POST", &binding_path("prepare"), body).await;
        assert_eq!(result.0, 400);
    }
    for path in [
        document_path("prepare") + "?account=other",
        document_path("prepare").replace("/versions/1/", "/versions/0/"),
    ] {
        assert_eq!(request(app.clone(), "POST", &path, json!({})).await.0, 400);
    }
}

#[tokio::test]
async fn missing_binding_trust_and_proof_rejections_have_stable_codes() {
    let harness = bindings::Harness::new();
    harness.state.lock().unwrap().context.trust = None;
    let app = router(harness, docs::Harness::new());
    let result = request(
        app,
        "POST",
        &binding_path("prepare"),
        json!({"certificate_base64":STANDARD.encode(bindings::certificate().der)}),
    )
    .await;
    assert_eq!(result.0, 503);
    assert_eq!(result.1["error"]["code"], "signing_trust_unavailable");
    let mut document = docs::Harness::new();
    document
        .binding
        .verifier
        .expect_verify_consent()
        .returning(|_, _, _, _, _, _| {
            Err(
                application::document_signing::SigningWorkflowError::Credential(
                    domain::crypto::CredentialFailure::Revoked,
                ),
            )
        });
    let result = request(
        router(bindings::Harness::new(), document),
        "POST",
        &document_path("submit"),
        submission(),
    )
    .await;
    assert_eq!(result.0, 422);
    assert_eq!(result.1["error"]["code"], "signing_credential_rejected");
}

#[tokio::test]
async fn registration_recovery_and_withdrawal_use_self_bound_canonical_evidence() {
    let mut harness = bindings::Harness::new();
    harness.state.lock().unwrap().found = Some(bindings::receipt());
    harness
        .store
        .expect_withdraw()
        .returning(|principal, id, account, at| {
            assert_eq!(principal, &bindings::principal());
            assert_eq!(id, bindings::id());
            assert_eq!(account, bindings::account());
            assert_eq!(at, bindings::at(1000));
            let mut receipt = bindings::receipt();
            receipt.withdrawn_at = Some(at);
            Ok(receipt)
        });
    let app = router(harness, docs::Harness::new());
    let value = request(
        app.clone(),
        "POST",
        &binding_path("register"),
        json!({
        "statement_base64":STANDARD.encode(bindings::statement().canonical_bytes()),
        "certificate_der_base64":STANDARD.encode(bindings::certificate().der),
        "signature_base64":STANDARD.encode(bindings::receipt().check.signature.as_bytes())}),
    )
    .await;
    assert_eq!(value.0, 200);
    assert!(value.1["withdrawn_at"].is_null());
    let recovered = request(
        app.clone(),
        "GET",
        binding_path("").trim_end_matches('/'),
        json!(null),
    )
    .await;
    assert_eq!(value, recovered);
    let withdrawn = request(app, "POST", &binding_path("withdraw"), json!(null)).await;
    assert_eq!(withdrawn.0, 200);
    assert!(withdrawn.1["withdrawn_at"].is_string());
}

#[tokio::test]
async fn public_proof_requests_bound_bodies_and_reject_duplicate_headers_and_fields() {
    let app = router(bindings::Harness::new(), docs::Harness::new());
    for headers in [vec![], vec!["Bearer session", "Bearer session"]] {
        assert_eq!(
            raw(
                app.clone(),
                "GET",
                "/api/v1/identity/signing-certificates/current",
                vec![],
                &headers,
                &[]
            )
            .await
            .0,
            401
        );
    }
    assert_eq!(
        raw(
            app.clone(),
            "POST",
            &binding_path("prepare"),
            b"{}".to_vec(),
            &["Bearer session"],
            &["application/json", "application/json"]
        )
        .await
        .0,
        400
    );
    assert_eq!(
        raw(
            app.clone(),
            "POST",
            &binding_path("prepare"),
            br#"{"certificate_base64":"AQ==","certificate_base64":"AQ=="}"#.to_vec(),
            &["Bearer session"],
            &["application/json"]
        )
        .await
        .0,
        400
    );
    assert_eq!(
        raw(
            app,
            "POST",
            &binding_path("prepare"),
            vec![b' '; 32769],
            &["Bearer session"],
            &["application/json"]
        )
        .await
        .0,
        413
    );
}

#[tokio::test]
async fn verification_allows_owner_and_explicitly_identifies_legacy_server_evidence() {
    let mut document = docs::Harness::new();
    document
        .binding
        .state
        .lock()
        .unwrap()
        .principal
        .as_mut()
        .unwrap()
        .role = domain::identity::Role::Owner;
    let processor = crypto::processor();
    let record = processor
        .seal(&document.state.lock().unwrap().record)
        .unwrap();
    document.processor = std::sync::Arc::new(processor);
    document
        .store
        .expect_verification_context()
        .returning(move |principal, case, reference| {
            assert_eq!(principal.role, domain::identity::Role::Owner);
            assert_eq!((case, reference), (docs::case(), docs::reference()));
            Ok(application::document_signing::SigningVerificationContext {
                signing: None,
                record: record.clone(),
                trust: None,
                binding_withdrawn_at: None,
            })
        });
    let result = request(
        router(bindings::Harness::new(), document),
        "GET",
        &document_path("verification"),
        json!(null),
    )
    .await;
    assert_eq!(result.0, 200);
    assert_eq!(result.1["kind"], "legacy_server");
    assert_eq!(result.1["certificate_current"]["status"], "not_individual");
    assert!(result.1["signing"].is_null());
    assert_eq!(result.1["historical"]["verdict"], "valid");
}
