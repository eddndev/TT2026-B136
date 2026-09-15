mod case_administration_support;

use axum::{body::Body, http::StatusCode};
use case_administration_support::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn strict_objects_reject_spoofed_context_duplicate_fields_and_wrong_revisions() {
    let workflow = Arc::new(Workflow::default());
    for raw in [
        "null",
        "[]",
        "{}",
        "{",
        "{\"title\":3,\"reference\":\"R\",\"profile\":null}",
    ] {
        assert_eq!(
            request(&workflow, "POST", "/api/v1/penal-cases", Some("owner"), raw)
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for field in [
        "id",
        "case_id",
        "changed_at",
        "changed_by",
        "values_digest",
        "administrative_status",
        "initial_stage",
        "required_initial_revision",
    ] {
        let mut input = creation();
        input[field] = json!("spoofed");
        assert_eq!(
            request(
                &workflow,
                "POST",
                "/api/v1/penal-cases",
                Some("owner"),
                input.to_string()
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST,
            "{field}"
        );
    }
    let mut nested = creation();
    nested["profile"]["created_by"] = json!(ACTOR);
    assert_eq!(
        request(
            &workflow,
            "POST",
            "/api/v1/penal-cases",
            Some("owner"),
            nested.to_string()
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let original = creation().to_string();
    let duplicate = original.replace("\"nuc\":\" NUC-001 \"", "\"nuc\":\"A\",\"nuc\":\"B\"");
    assert_ne!(original, duplicate);
    assert_eq!(
        request(
            &workflow,
            "POST",
            "/api/v1/penal-cases",
            Some("owner"),
            duplicate
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    for value in [
        json!(-1),
        json!(4294967296_u64),
        json!(1.5),
        json!("1"),
        json!(null),
    ] {
        let mut input = replacement(1);
        input["expected_revision"] = value;
        assert_eq!(
            request(&workflow, "PUT", &item(), Some("owner"), input.to_string())
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for state in ["Active", "closed ", "all", "deleted"] {
        assert_eq!(
            request(
                &workflow,
                "PUT",
                &status_path(),
                Some("owner"),
                json!({"expected_revision":1,"administrative_status":state}).to_string()
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for raw in [
        "{\"expected_revision\":1,\"administrative_status\":\"closed\",\"title\":\"stale\"}",
        "{\"expected_revision\":1,\"expected_revision\":2,\"administrative_status\":\"active\"}",
    ] {
        assert_eq!(
            request(&workflow, "PUT", &status_path(), Some("owner"), raw)
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn invalid_profiles_never_reach_commands_and_multiline_controls_are_explicit() {
    let workflow = Arc::new(Workflow::default());
    for (field, value) in [
        ("nuc", json!(" ")),
        ("nuc", json!("x".repeat(101))),
        ("nuc_authority", json!("x".repeat(201))),
        ("judicial_case_number", json!("x".repeat(101))),
        ("judicial_authority", json!("x".repeat(201))),
        ("offenses", json!([])),
        ("offenses", json!(["A", " A "])),
        ("offenses", json!(["x".repeat(121)])),
        ("general_information", json!("\rtext")),
        ("general_information", json!("\ttext")),
        ("general_information", json!("x".repeat(1001))),
        ("complementary_identifiers", json!("\nvalue")),
        ("complementary_identifiers", json!("x".repeat(301))),
    ] {
        let mut input = creation();
        input["profile"][field] = value;
        let response = request(
            &workflow,
            "POST",
            "/api/v1/penal-cases",
            Some("owner"),
            input.to_string(),
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{field}"
        );
        assert_eq!(
            body(response).await["error"]["code"],
            "invalid_penal_case_profile"
        );
    }
    for value in ["".to_string(), "\nTitle".into(), "x".repeat(201)] {
        let mut input = creation();
        input["title"] = json!(value);
        let response = request(
            &workflow,
            "POST",
            "/api/v1/penal-cases",
            Some("owner"),
            input.to_string(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            body(response).await["error"]["code"],
            "invalid_case_metadata"
        );
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn query_types_and_semantic_bounds_fail_before_dispatch() {
    let workflow = Arc::new(Workflow::default());
    for query in [
        "limit=x",
        "limit=-1",
        "limit=4294967296",
        "after_id=bad",
        "status=Active",
        "profile=unknown",
        "unknown=1",
        "limit=1&limit=2",
    ] {
        let response = request(
            &workflow,
            "GET",
            &format!("/api/v1/case-administrations?{query}"),
            Some("owner"),
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
    }
    for query in ["limit=0", "limit=101", "title=%0Aname", "nuc=%09N"] {
        let response = request(
            &workflow,
            "GET",
            &format!("/api/v1/case-administrations?{query}"),
            Some("owner"),
            Body::empty(),
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{query}"
        );
    }
    for (query, status) in [
        ("before_revision=0", 422),
        ("before_revision=-1", 400),
        ("before_revision=4294967296", 400),
        ("limit=101", 422),
        ("status=closed", 400),
    ] {
        assert_eq!(
            request(
                &workflow,
                "GET",
                &format!("{}/history?{query}", item()),
                Some("owner"),
                Body::empty()
            )
            .await
            .status()
            .as_u16(),
            status,
            "{query}"
        );
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn complete_body_limit_counts_trailing_bytes_and_stream_chunks() {
    let workflow = Arc::new(Workflow::default());
    for (method, path, input) in [
        ("POST", "/api/v1/penal-cases".to_string(), creation()),
        ("PUT", item(), replacement(3)),
        (
            "PUT",
            status_path(),
            json!({"expected_revision":3,"administrative_status":"closed"}),
        ),
    ] {
        let mut raw = input.to_string();
        raw.extend(std::iter::repeat_n(' ', 65_536 - raw.len()));
        assert_eq!(
            request(&workflow, method, &path, Some("owner"), raw.clone())
                .await
                .status()
                .as_u16(),
            if method == "POST" { 201 } else { 200 }
        );
        raw.push(' ');
        let response = request(&workflow, method, &path, Some("owner"), raw).await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            body(response).await["error"]["code"],
            "case_administration_body_too_large"
        );
    }
    let chunks = futures_util::stream::iter([
        Ok::<_, std::io::Error>(creation().to_string()),
        Ok(" ".repeat(65_536)),
    ]);
    assert_eq!(
        request(
            &workflow,
            "POST",
            "/api/v1/penal-cases",
            Some("owner"),
            Body::from_stream(chunks)
        )
        .await
        .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(workflow.calls.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn maximum_astral_values_fit_even_when_every_scalar_is_escaped() {
    let workflow = Arc::new(Workflow::default());
    let escaped = |length: usize| "\\ud800\\udc00".repeat(length);
    let offenses = (0..8)
        .map(|index| format!("\"{}\"", format!("\\ud800\\udc0{index}").repeat(120)))
        .collect::<Vec<_>>()
        .join(",");
    let input=format!(concat!("{{\"expected_revision\":4294967295,\"title\":\"{}\",\"reference\":\"{}\",\"profile\":{{",
        "\"nuc\":\"{}\",\"nuc_authority\":\"{}\",\"judicial_case_number\":\"{}\",\"judicial_authority\":\"{}\",",
        "\"offenses\":[{}],\"general_information\":\"{}\",\"complementary_identifiers\":\"{}\"}}}}"),
        escaped(200),escaped(100),escaped(100),escaped(200),escaped(100),escaped(200),offenses,escaped(1000),escaped(300));
    assert_eq!(input.len(), 38_161);
    assert_eq!(
        request(&workflow, "PUT", &item(), Some("owner"), input)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(workflow.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn missing_bearer_and_invalid_case_uuid_never_reach_the_workflow() {
    let workflow = Arc::new(Workflow::default());
    for (method, path, input) in [
        ("GET", item(), String::new()),
        ("POST", "/api/v1/penal-cases".into(), "{".into()),
    ] {
        let response = request(&workflow, method, &path, None, input).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body(response).await["error"]["code"], "invalid_session");
    }
    let response = request(
        &workflow,
        "GET",
        "/api/v1/cases/not-a-uuid/administration",
        Some("owner"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body(response).await["error"]["code"], "invalid_case_id");
    assert!(workflow.calls.lock().unwrap().is_empty());
}
