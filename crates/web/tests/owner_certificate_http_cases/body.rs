use axum::http::StatusCode;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;

use crate::{owner_certificate_http_support::*, support::*};

#[tokio::test]
async fn strict_json_rejects_untrusted_authority_duplicate_fields_and_trailing_input() {
    let harness = Harness::new();
    let observed = harness.state.clone();
    let app = router(harness);
    for (operation, body) in [
        ("/prepare", prepare_body()),
        ("/register", registration_body()),
        ("/withdraw", json!({"expected_revision":1})),
    ] {
        for key in [
            "owner_id",
            "trust",
            "verified",
            "checked_at",
            "success",
            "private_key",
        ] {
            let mut altered = body.clone();
            altered[key] = json!("private-port-detail");
            request(&app, "POST", operation, Some(altered))
                .await
                .error(StatusCode::BAD_REQUEST, "invalid_json");
        }
        let key = body.as_object().unwrap().keys().next().unwrap();
        let text = body.to_string();
        let duplicate = format!(
            "{{{}:{},{}",
            serde_json::to_string(key).unwrap(),
            body[key.as_str()],
            &text[1..]
        );
        let positional = match operation {
            "/prepare" => json!([body["certificate_base64"]]),
            "/register" => json!([
                body["statement_base64"],
                body["certificate_der_base64"],
                body["signature_base64"]
            ]),
            _ => json!([1]),
        };
        for text in [
            duplicate,
            format!("{text} {{}}"),
            positional.to_string(),
            "[]".into(),
            "null".into(),
        ] {
            raw(
                &app,
                "POST",
                &path(operation),
                text.into_bytes(),
                &["application/json"],
                &["Bearer session"],
            )
            .await
            .error(StatusCode::BAD_REQUEST, "invalid_json");
        }
    }
    assert!(observed.lock().unwrap().calls.is_empty());
}

#[tokio::test]
async fn queries_get_entities_invalid_ids_and_ambiguous_mime_are_rejected_before_ports() {
    let harness = Harness::new();
    let observed = harness.state.clone();
    let app = router(harness);
    for (method, operation, body) in [
        ("GET", "", None),
        ("POST", "/prepare", Some(prepare_body())),
        ("POST", "/register", Some(registration_body())),
        ("POST", "/withdraw", Some(json!({"expected_revision":1}))),
    ] {
        let bytes = body.map(|v| v.to_string().into_bytes()).unwrap_or_default();
        raw(
            &app,
            method,
            &format!("{}?owner_id=untrusted", path(operation)),
            bytes,
            &["application/json"],
            &["Bearer session"],
        )
        .await
        .error(StatusCode::BAD_REQUEST, "owner_certificate_invalid_input");
    }
    for body in [b"{}".to_vec(), b"\n".to_vec()] {
        raw(&app, "GET", &path(""), body, &[], &["Bearer session"])
            .await
            .error(StatusCode::BAD_REQUEST, "owner_certificate_invalid_input");
    }
    for id in ["not-a-uuid", "00000000-0000-0000-0000-000000000000"] {
        raw(
            &app,
            "POST",
            &format!("/api/v1/auth/certificate-bindings/{id}/prepare"),
            prepare_body().to_string().into_bytes(),
            &["application/json"],
            &["Bearer session"],
        )
        .await
        .error(StatusCode::BAD_REQUEST, "owner_certificate_invalid_input");
    }
    for types in [
        vec![],
        vec!["text/plain"],
        vec!["application/json", "application/json"],
    ] {
        raw(
            &app,
            "POST",
            &path("/prepare"),
            prepare_body().to_string().into_bytes(),
            &types,
            &["Bearer session"],
        )
        .await
        .error(StatusCode::BAD_REQUEST, "invalid_json");
    }
    assert!(observed.lock().unwrap().calls.is_empty());
}

#[tokio::test]
async fn canonical_base64_and_decoded_lengths_are_enforced_before_service_calls() {
    let harness = Harness::new();
    let observed = harness.state.clone();
    let app = router(harness);
    for encoded in ["", "Zg", "Zg===", "_w==", " Zg==", "Zh=="] {
        request(
            &app,
            "POST",
            "/prepare",
            Some(json!({"certificate_base64":encoded})),
        )
        .await
        .error(StatusCode::BAD_REQUEST, "owner_certificate_invalid_input");
    }
    request(
        &app,
        "POST",
        "/prepare",
        Some(json!({"certificate_base64":STANDARD.encode(vec![1;16385])})),
    )
    .await
    .error(StatusCode::BAD_REQUEST, "owner_certificate_invalid_input");
    for (field, sizes) in [
        ("statement_base64", vec![149, 151]),
        ("certificate_der_base64", vec![0, 16385]),
        ("signature_base64", vec![383, 385]),
    ] {
        for size in sizes {
            let mut body = registration_body();
            body[field] = json!(STANDARD.encode(vec![1; size]));
            request(&app, "POST", "/register", Some(body))
                .await
                .error(StatusCode::BAD_REQUEST, "owner_certificate_invalid_input");
        }
    }
    for expected in [json!(-1), json!(1.5), json!("1"), json!(4294967296_u64)] {
        request(
            &app,
            "POST",
            "/withdraw",
            Some(json!({"expected_revision":expected})),
        )
        .await
        .error(StatusCode::BAD_REQUEST, "invalid_json");
    }
    assert!(observed.lock().unwrap().calls.is_empty());
}

#[tokio::test]
async fn certificate_group_accepts_maximum_public_input_and_bounds_the_complete_json_entity() {
    let mut harness = Harness::new();
    let observed = harness.state.clone();
    harness.verifier = MockVerifier::new();
    harness
        .verifier
        .expect_inspect_certificate()
        .times(2)
        .returning(|bytes| {
            assert_eq!(bytes, vec![7; 16384]);
            Ok(certificate())
        });
    let app = router(harness);
    let body = json!({"certificate_base64":STANDARD.encode(vec![7;16384])}).to_string();
    assert!(body.len() > 16384 && body.len() < 32768);
    for bytes in [
        body.as_bytes().to_vec(),
        format!("{}{}", " ".repeat(32768 - body.len()), body).into_bytes(),
    ] {
        let response = raw(
            &app,
            "POST",
            &path("/prepare"),
            bytes,
            &["application/json; charset=utf-8"],
            &["Bearer session"],
        )
        .await;
        response.public();
        assert_eq!(response.status, StatusCode::OK);
    }
    let before = observed.lock().unwrap().calls.clone();
    let body = format!("{}{}", " ".repeat(32768), prepare_body());
    raw(
        &app,
        "POST",
        &path("/prepare"),
        body.into_bytes(),
        &["application/json"],
        &["Bearer session"],
    )
    .await
    .error(
        StatusCode::PAYLOAD_TOO_LARGE,
        "owner_certificate_body_too_large",
    );
    assert_eq!(observed.lock().unwrap().calls, before);
}
